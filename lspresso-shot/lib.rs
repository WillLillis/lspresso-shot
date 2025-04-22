mod init_dot_lua;
pub mod types;

use lsp_types::{
    request::{GotoDeclarationResponse, GotoImplementationResponse, GotoTypeDefinitionResponse},
    CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall, CodeLens,
    CompletionItem, Diagnostic, DocumentDiagnosticReport, DocumentHighlight, DocumentLink,
    DocumentSymbolResponse, FoldingRange, FormattingOptions, GotoDefinitionResponse, Hover,
    Location, Moniker, Position, PreviousResultId, Range, SelectionRange, SemanticTokens,
    SemanticTokensDelta, SemanticTokensFullDeltaResult, SemanticTokensPartialResult,
    SemanticTokensRangeResult, SemanticTokensResult, SignatureHelp, SignatureHelpContext, TextEdit,
    WorkspaceDiagnosticReport, WorkspaceEdit,
};

use std::{
    collections::HashMap,
    fs,
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, Condvar, Mutex, OnceLock},
};

use types::{
    call_hierarchy::{
        IncomingCallsMismatchError, OutgoingCallsMismatchError, PrepareCallHierachyMismatchError,
    },
    code_lens::{CodeLensMismatchError, CodeLensResolveMismatchError},
    completion::{CompletionMismatchError, CompletionResolveMismatchError, CompletionResult},
    declaration::DeclarationMismatchError,
    definition::DefinitionMismatchError,
    diagnostic::{
        DiagnosticMismatchError, PublishDiagnosticsMismatchError, WorkspaceDiagnosticMismatchError,
    },
    document_highlight::DocumentHighlightMismatchError,
    document_link::{DocumentLinkMismatchError, DocumentLinkResolveMismatchError},
    document_symbol::DocumentSymbolMismatchError,
    folding_range::FoldingRangeMismatchError,
    formatting::{FormattingMismatchError, FormattingResult},
    hover::HoverMismatchError,
    implementation::ImplementationMismatchError,
    moniker::MonikerMismatchError,
    references::ReferencesMismatchError,
    rename::RenameMismatchError,
    selection_range::SelectionRangeMismatchError,
    semantic_tokens::{
        SemanticTokensFullDeltaMismatchError, SemanticTokensFullMismatchError,
        SemanticTokensRangeMismatchError,
    },
    signature_help::SignatureHelpMismatchError,
    type_definition::TypeDefinitionMismatchError,
    CleanResponse, Empty, EmptyResult, TestCase, TestError, TestResult, TestType, TimeoutError,
};

/// Intended to be used as a wrapper for `lspresso-shot` testing functions. If the
/// result is `Ok`, the value is returned. If `Err`, pretty-prints the error via
/// `panic`
#[macro_export]
macro_rules! lspresso_shot {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(err) => panic!("{err}"),
        }
    };
}

/// The parallelism utilized in Cargo's test runner and the concrete timeout values
/// used in our test cases do not play nicely together, leading to intermittent failures.
/// We use `RUNNER_COUNT` to restrict the number of concurrent test cases, treating
/// each case's "neovim portion" inside `run_test` as a critical section. Another
/// approach that works is to manually limit the number of threads used by the test
/// runner via `--test-threads x`, but it isn't realistic to expect consumers to do this.
///
/// It looks like this value needs to be 1, so we could replace the `u32` with a `bool`,
/// but I'll leave it as is for now in case I come up with some other workaround
static RUNNER_LIMIT: u32 = 1;
static RUNNER_COUNT: OnceLock<Arc<(Mutex<u32>, Condvar)>> = OnceLock::new();

fn get_runner_count() -> Arc<(Mutex<u32>, Condvar)> {
    #[allow(clippy::mutex_integer)]
    RUNNER_COUNT
        .get_or_init(|| Arc::new((Mutex::new(0), Condvar::new())))
        .clone()
}

/// Helper struct to automatically decrement `n_jobs` when dropped.
struct RunnerGuard<'a> {
    lock: &'a Mutex<u32>,
    cvar: &'a Condvar,
}

impl<'a> RunnerGuard<'a> {
    fn new(lock: &'a Mutex<u32>, cvar: &'a Condvar) -> Self {
        let mut n_jobs = lock.lock().expect("Mutex poisoned");

        while *n_jobs >= RUNNER_LIMIT {
            n_jobs = cvar.wait(n_jobs).expect("Condition variable poisoned");
        }

        *n_jobs += 1;
        drop(n_jobs);

        Self { lock, cvar }
    }
}

impl Drop for RunnerGuard<'_> {
    fn drop(&mut self) {
        *self.lock.lock().expect("Mutex poisoned") -= 1;
        self.cvar.notify_one();
    }
}

/// This handles the validating the test case, running the test, and gathering/deserializing
/// results.
///
/// `R2` is *always* the expected response type for the given test case. If the test case is
/// expecting `Some(_)` results, then `R1 == R2`. If `None` is expected, `R1` is `EmptyResult`.
///
/// If `R1` is `Empty`, we expect empty results and this will only return `Err`
/// or `Ok(None)`.
///
/// Otherwise, we expect some results, and this function will return `Ok(Some(_))` or `Err`.
fn test_inner<R1, R2>(
    test_case: &TestCase,
    replacements: Option<&Vec<(&str, String)>>,
) -> TestResult<Option<R2>>
where
    R1: serde::de::DeserializeOwned + std::fmt::Debug + Empty + CleanResponse,
    R2: serde::de::DeserializeOwned + std::fmt::Debug + Empty + CleanResponse,
{
    let get_results = |path: &Path| -> TestResult<R2> {
        let raw_results = String::from_utf8(
            fs::read(path).map_err(|e| TestError::IO(test_case.test_id.clone(), e.to_string()))?,
        )
        .map_err(|e| TestError::Utf8(test_case.test_id.clone(), e.to_string()))?;
        let raw_resp: R2 = serde_json::from_str(&raw_results)
            .map_err(|e| TestError::Serialization(test_case.test_id.clone(), e.to_string()))?;
        let cleaned = raw_resp.clean_response(test_case)?;
        Ok(cleaned)
    };
    test_case.validate()?;
    // Invariant: `test_case.test_type` should always be set to `Some(_)` in the caller
    let source_path = test_case.create_test(
        test_case.test_type.expect("Test type is `None`"),
        replacements,
    )?;
    run_test(test_case, &source_path)?;

    let empty_result_path = test_case
        .get_empty_file_path()
        .map_err(|e| TestError::IO(test_case.test_id.clone(), e.to_string()))?;
    let results_file_path = test_case
        .get_results_file_path()
        .map_err(|e| TestError::IO(test_case.test_id.clone(), e.to_string()))?;

    match (
        R1::is_empty(),
        empty_result_path.exists(),
        results_file_path.exists(),
    ) {
        // Expected and got empty results
        (true, true, false) => Ok(None),
        // Expected empty results, got some
        (true, false, true) => {
            // Don't propagate errors up here, as it's better for the user to see
            // that they expected empty results but got some instead
            let results: TestResult<R2> = get_results(&results_file_path);
            let actual_str = match results {
                Ok(res) => format!("{res:#?}"),
                Err(e) => format!("Invalid results: {e}"),
            };
            Err(TestError::ExpectedNone(
                test_case.test_id.clone(),
                actual_str,
            ))?
        }
        // Invariant: `results.json` and `empty` should never both exist
        (true | false, true, true) => unreachable!(),
        // No results
        (true | false, false, false) => Err(TestError::NoResults(test_case.test_id.clone()))?,
        // Expected some results, got none
        (false, true, false) => Err(TestError::ExpectedSome(test_case.test_id.clone()))?,
        // Expected and got some results
        (false, false, true) => {
            let actual: R2 = get_results(&results_file_path)?;
            Ok(Some(actual))
        }
    }
}

/// Invokes neovim to run the test with `test_case`'s associated `init.lua` file,
/// opening `source_path`
fn run_test(test_case: &TestCase, source_path: &Path) -> TestResult<()> {
    let init_dot_lua_path = test_case
        .get_init_lua_file_path()
        .map_err(|e| TestError::IO(test_case.test_id.clone(), e.to_string()))?;

    // Restrict the number of tests invoking neovim at a given time to prevent timeout issues
    let (lock, cvar) = &*get_runner_count();
    let _guard = RunnerGuard::new(lock, cvar); // Ensures proper decrement on exit

    let start = std::time::Instant::now();
    let mut child = Command::new(&test_case.nvim_path)
        .arg("-u")
        .arg(init_dot_lua_path)
        .arg("--noplugin")
        .arg(source_path)
        .arg("--headless")
        .arg("-n") // disable swap files
        .stdout(Stdio::null()) // Commenting these out can be helpful for local
        .stderr(Stdio::null()) // debugging, can print some rust-analyzer logs
        .spawn()
        .map_err(|e| TestError::Neovim(test_case.test_id.clone(), e.to_string()))?;

    while start.elapsed() < test_case.timeout {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) => {} // still running
            Err(e) => Err(TestError::Neovim(test_case.test_id.clone(), e.to_string()))?,
        }
    }

    // A test can timeout due to neovim encountering an error (i.e. a malformed
    // `init.lua` file). If we have an error recorded, it's better to report that
    // than the timeout
    let error_path = test_case
        .get_error_file_path()
        .map_err(|e| TestError::IO(test_case.test_id.clone(), e.to_string()))?;
    if error_path.exists() {
        let error = fs::read_to_string(&error_path)
            .map_err(|e| TestError::IO(test_case.test_id.clone(), e.to_string()))?;
        Err(TestError::Neovim(test_case.test_id.clone(), error))?;
    }

    Err(TestError::TimeoutExceeded(TimeoutError {
        test_id: test_case.test_id.clone(),
        timeout: test_case.timeout,
    }))?
}

fn collect_results<R1, R2>(
    test_case: &TestCase,
    replacements: Option<&Vec<(&str, String)>>,
    expected: Option<&R1>,
    cmp: impl Fn(&R1, &R2) -> TestResult<()>,
) -> TestResult<()>
where
    R1: std::fmt::Debug,
    R2: serde::de::DeserializeOwned + std::fmt::Debug + Empty + CleanResponse,
{
    if let Some(expected) = expected {
        let actual = test_inner::<R2, R2>(test_case, replacements)?.unwrap();
        Ok(cmp(expected, &actual)?)
    } else {
        let empty = test_inner::<EmptyResult, R2>(test_case, replacements)?;
        assert!(empty.is_none());
        Ok(())
    }
}

fn get_cursor_replacement(cursor_pos: &Position) -> (&str, String) {
    (
        "SET_CURSOR_POSITION",
        format!(
            "position = {{ line = {}, character = {} }}",
            cursor_pos.line, cursor_pos.character
        ),
    )
}

pub type CodeLensComparator = fn(&Vec<CodeLens>, &Vec<CodeLens>, &TestCase) -> bool;

/// Tests the server's response to a 'textDocument/codeLens' request
///
/// - `commands` is a list of LSP command names the client should advertise support for in its
///   capabilities (e.g. "rust-analyzer.runSingle"). This enables command-based `CodeLens`
///   responses from the server, such as "Run" or "Debug" actions.
///
/// - `cmp` is an optional custom comparator function that can be used to compare the expected
///   and actual results. Becaue the `CodeLens` struct can contain arbitrary JSON, it's not feasible
///   to clean results from test-case specific information (e.g. the root path).
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
pub fn test_code_lens(
    mut test_case: TestCase,
    commands: Option<&Vec<String>>,
    cmp: Option<CodeLensComparator>,
    expected: Option<&Vec<CodeLens>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::CodeLens);
    let command_str = commands.map_or_else(String::new, |cmds| {
        cmds.iter()
            .fold(String::new(), |accum, cmd| accum + &format!("\"{cmd}\",\n"))
    });

    collect_results(
        &test_case,
        Some(&vec![("COMMANDS", command_str)]),
        expected,
        |expected, actual: &Vec<CodeLens>| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
                Err(CodeLensMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: (*expected).clone(),
                    actual: actual.clone(),
                })?;
            }
            Ok(())
        },
    )
}

pub type CodeLensResolveComparator = fn(&CodeLens, &CodeLens, &TestCase) -> bool;

/// Tests the server's response to a 'codeLens/resolve' request
///
/// - `commands` is a list of LSP command names the client should advertise support for in its
///   capabilities (e.g. "rust-analyzer.runSingle"). This enables command-based `CodeLens`
///   responses from the server, such as "Run" or "Debug" actions.
///
/// - `cmp` is an optional custom comparator function that can be used to compare the expected
///   and actual results. Becaue the `CodeLens` struct can contain arbitrary JSON, it's not feasible
///   to clean results from test-case specific information (e.g. the root path).
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON deserialization of `code_lens` fails
pub fn test_code_lens_resolve(
    mut test_case: TestCase,
    commands: Option<&Vec<String>>,
    code_lens: &CodeLens,
    cmp: Option<CodeLensResolveComparator>,
    expected: Option<&CodeLens>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::CodeLensResolve);
    let command_str = commands.map_or_else(String::new, |cmds| {
        cmds.iter()
            .fold(String::new(), |accum, cmd| accum + &format!("\"{cmd}\",\n"))
    });
    let code_lens_json =
        serde_json::to_string_pretty(code_lens).expect("JSON deserialzation of code lens failed");

    collect_results(
        &test_case,
        Some(&vec![
            ("COMMANDS", command_str),
            ("CODE_LENS", code_lens_json),
        ]),
        expected,
        |expected, actual: &CodeLens| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
                Err(Box::new(CodeLensResolveMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: (*expected).clone(),
                    actual: actual.clone(),
                }))?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'textDocument/complection' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
pub fn test_completion(
    mut test_case: TestCase,
    cursor_pos: &Position,
    expected: Option<&CompletionResult>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::Completion);
    collect_results(
        &test_case,
        Some(&vec![get_cursor_replacement(cursor_pos)]),
        expected,
        |expected, actual| {
            if !expected.results_satisfy(actual) {
                Err(CompletionMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: (*expected).clone(),
                    actual: actual.clone(),
                })?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'completionItem/resolve' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON deserialization of `completion_item` fails
pub fn test_completion_resolve(
    mut test_case: TestCase,
    completion_item: &CompletionItem,
    expected: Option<&CompletionItem>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::CompletionResolve);

    let completion_item = serde_json::to_string_pretty(completion_item)
        .expect("JSON deserialzation of completion item failed");
    collect_results(
        &test_case,
        Some(&vec![("COMPLETION_ITEM", completion_item)]),
        expected,
        |expected, actual| {
            if expected != actual {
                Err(Box::new(CompletionResolveMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: (*expected).clone(),
                    actual: actual.clone(),
                }))?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'textDocument/declaration' request
///
/// # Errors
///
/// Returns `TestError` if the expected results don't match, or if some other failure occurs
pub fn test_declaration(
    mut test_case: TestCase,
    cursor_pos: &Position,
    expected: Option<&GotoDeclarationResponse>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::Declaration);
    collect_results(
        &test_case,
        Some(&vec![get_cursor_replacement(cursor_pos)]),
        expected,
        |expected, actual| {
            // HACK: Since the `GotoDeclarationResponse` is untagged, there's no way to differentiate
            // between the `Array` and `Link` if we get an empty vector in response. Just
            // treat this as a special case and say it's ok.
            match (expected, actual) {
                (
                    GotoDeclarationResponse::Array(array_items),
                    GotoDeclarationResponse::Link(link_items),
                )
                | (
                    GotoDeclarationResponse::Link(link_items),
                    GotoDeclarationResponse::Array(array_items),
                ) => {
                    if array_items.is_empty() && link_items.is_empty() {
                        return Ok(());
                    }
                }
                _ => {}
            }

            if *expected != *actual {
                Err(Box::new(DeclarationMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                }))?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'textDocument/definition' request
///
/// # Errors
///
/// Returns `TestError` if the expected results don't match, or if some other failure occurs
pub fn test_definition(
    mut test_case: TestCase,
    cursor_pos: &Position,
    expected: Option<&GotoDefinitionResponse>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::Definition);
    collect_results(
        &test_case,
        Some(&vec![get_cursor_replacement(cursor_pos)]),
        expected,
        |expected, actual| {
            // HACK: Since the `GotoDefinitionResponse` is untagged, there's no way to differentiate
            // between the `Array` and `Link` if we get an empty vector in response. Just
            // treat this as a special case and say it's ok.
            match (expected, actual) {
                (
                    GotoDefinitionResponse::Array(array_items),
                    GotoDefinitionResponse::Link(link_items),
                )
                | (
                    GotoDefinitionResponse::Link(link_items),
                    GotoDefinitionResponse::Array(array_items),
                ) => {
                    if array_items.is_empty() && link_items.is_empty() {
                        return Ok(());
                    }
                }
                _ => {}
            }

            if *expected != *actual {
                Err(Box::new(DefinitionMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                }))?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'textDocument/diagnostic' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON deserialization of `identifier` or `previous_result_id` fails
pub fn test_diagnostic(
    mut test_case: TestCase,
    identifier: Option<&str>,
    previous_result_id: Option<&str>,
    expected: &DocumentDiagnosticReport,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::Diagnostic);
    let identifier = identifier.map_or_else(
        || "null".to_string(), // NOTE: `vim.json.decode()` fails with an empty string
        |id| serde_json::to_string_pretty(id).expect("JSON deserialzation of identifier failed"),
    );
    let previous_result_id = previous_result_id.map_or_else(
        || "null".to_string(), // NOTE: `vim.json.decode()` fails with an empty string
        |id| {
            serde_json::to_string_pretty(id)
                .expect("JSON deserialzation of previous result id failed")
        },
    );
    collect_results(
        &test_case,
        Some(&vec![
            ("IDENTIFIER", identifier),
            ("PREVIOUS_RESULT_ID", previous_result_id),
        ]),
        Some(expected),
        |expected: &DocumentDiagnosticReport, actual: &DocumentDiagnosticReport| {
            if expected != actual {
                Err(Box::new(DiagnosticMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                }))?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'textDocument/documentHighlight' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
pub fn test_document_highlight(
    mut test_case: TestCase,
    cursor_pos: &Position,
    expected: Option<&Vec<DocumentHighlight>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::DocumentHighlight);
    collect_results(
        &test_case,
        Some(&vec![get_cursor_replacement(cursor_pos)]),
        expected,
        |expected, actual: &Vec<DocumentHighlight>| {
            if expected != actual {
                Err(DocumentHighlightMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                })?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'textDocument/documentLink' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
pub fn test_document_link(
    mut test_case: TestCase,
    expected: Option<&Vec<DocumentLink>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::DocumentLink);
    collect_results(
        &test_case,
        None,
        expected,
        |expected, actual: &Vec<DocumentLink>| {
            if expected != actual {
                Err(DocumentLinkMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                })?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'documentLink/resolve' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON deserialization of `link` fails
pub fn test_document_link_resolve(
    mut test_case: TestCase,
    link: &DocumentLink,
    expected: Option<&DocumentLink>,
) -> TestResult<()> {
    let json_link =
        serde_json::to_string_pretty(link).expect("JSON deserialzation of document link failed");
    test_case.test_type = Some(TestType::DocumentLinkResolve);
    collect_results(
        &test_case,
        Some(&vec![("DOC_LINK", json_link)]),
        expected,
        |expected, actual: &DocumentLink| {
            if expected != actual {
                Err(Box::new(DocumentLinkResolveMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                }))?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'textDocument/documentSymbol' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
pub fn test_document_symbol(
    mut test_case: TestCase,
    expected: Option<&DocumentSymbolResponse>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::DocumentSymbol);
    collect_results(&test_case, None, expected, |expected, actual| {
        // HACK: Since the two types of DocumentSymbolResponse are untagged, there's no
        // way to differentiate between the two if we get an empty vector in response.
        // Just treat this as a special case and say it's ok.
        match (expected, actual) {
            (
                DocumentSymbolResponse::Flat(flat_items),
                DocumentSymbolResponse::Nested(nested_items),
            )
            | (
                DocumentSymbolResponse::Nested(nested_items),
                DocumentSymbolResponse::Flat(flat_items),
            ) => {
                if flat_items.is_empty() && nested_items.is_empty() {
                    return Ok(());
                }
            }
            _ => {}
        }
        if expected != actual {
            Err(DocumentSymbolMismatchError {
                test_id: test_case.test_id.clone(),
                expected: expected.clone(),
                actual: actual.clone(),
            })?;
        }

        Ok(())
    })
}

/// Tests the server's response to a 'textDocument/foldingRange' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
pub fn test_folding_range(
    mut test_case: TestCase,
    expected: Option<&Vec<FoldingRange>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::FoldingRange);
    collect_results(
        &test_case,
        None,
        expected,
        |expected, actual: &Vec<FoldingRange>| {
            if expected != actual {
                Err(FoldingRangeMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                })?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'textDocument/formatting' request. If `options`
/// is `None`, the following default is used:
///
/// ```rust
/// lsp_types::FormattingOptions {
///     tab_size: 4,
///     insert_spaces: true,
///     properties: std::collections::HashMap::new(),
///     trim_trailing_whitespace: None,
///     insert_final_newline: None,
///     trim_final_newlines: None,
/// };
///
/// ```
/// # Errors
///
/// Returns `TestError` if the expected results don't match, or if some other failure occurs
///
/// # Panics
///
/// Panics if JSON deserialization of `options` fails
pub fn test_formatting(
    mut test_case: TestCase,
    options: Option<FormattingOptions>,
    expected: Option<&FormattingResult>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::Formatting);
    let opts = options.unwrap_or_else(|| FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        properties: HashMap::new(),
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    });
    let json_opts = serde_json::to_string_pretty(&opts)
        .expect("JSON deserialzation of formatting options failed");
    match expected {
        Some(FormattingResult::Response(edits)) => collect_results(
            &test_case,
            Some(&vec![
                ("INVOKE_FORMAT", "false".to_string()),
                ("JSON_OPTIONS", json_opts),
            ]),
            Some(edits),
            |expected, actual: &Vec<TextEdit>| {
                if expected != actual {
                    Err(FormattingMismatchError {
                        test_id: test_case.test_id.clone(),
                        expected: FormattingResult::Response(expected.clone()),
                        actual: FormattingResult::Response(actual.clone()),
                    })?;
                }
                Ok(())
            },
        ),
        Some(FormattingResult::EndState(state)) => collect_results(
            &test_case,
            Some(&vec![
                ("INVOKE_FORMAT", "true".to_string()),
                ("JSON_OPTIONS", json_opts),
            ]),
            Some(state),
            |expected, actual: &String| {
                if expected != actual {
                    Err(FormattingMismatchError {
                        test_id: test_case.test_id.clone(),
                        expected: FormattingResult::EndState(expected.clone()),
                        actual: FormattingResult::EndState(actual.clone()),
                    })?;
                }
                Ok(())
            },
        ),
        None => collect_results(
            &test_case,
            Some(&vec![
                ("INVOKE_FORMAT", "false".to_string()),
                ("JSON_OPTIONS", json_opts),
            ]),
            None,
            |expected: &Vec<TextEdit>, actual: &Vec<TextEdit>| {
                if expected != actual {
                    Err(FormattingMismatchError {
                        test_id: test_case.test_id.clone(),
                        expected: FormattingResult::Response(expected.clone()),
                        actual: FormattingResult::Response(actual.clone()),
                    })?;
                }
                Ok(())
            },
        ),
    }
}

/// Tests the server's response to a 'textDocument/hover' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
pub fn test_hover(
    mut test_case: TestCase,
    cursor_pos: &Position,
    expected: Option<&Hover>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::Hover);
    collect_results(
        &test_case,
        Some(&vec![get_cursor_replacement(cursor_pos)]),
        expected,
        |expected, actual| {
            if expected != actual {
                Err(Box::new(HoverMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                }))?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'textDocument/implementation' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
pub fn test_implementation(
    mut test_case: TestCase,
    cursor_pos: &Position,
    expected: Option<&GotoImplementationResponse>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::Implementation);
    collect_results(
        &test_case,
        Some(&vec![get_cursor_replacement(cursor_pos)]),
        expected,
        |expected, actual| {
            // HACK: Since `GotoImplementationResponse` is untagged, there is no way to
            // differentiate between the `Array` and `Link` variants if we get an empty
            // vector in response.
            // Just treat this as a special case and say it's ok.
            match (expected, actual) {
                (
                    GotoImplementationResponse::Array(array_items),
                    GotoImplementationResponse::Link(link_items),
                )
                | (
                    GotoImplementationResponse::Link(link_items),
                    GotoImplementationResponse::Array(array_items),
                ) => {
                    if array_items.is_empty() && link_items.is_empty() {
                        return Ok(());
                    }
                }
                _ => {}
            }
            if expected != actual {
                Err(Box::new(ImplementationMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                }))?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'callHierarchy/incomingCalls' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON deserialization of `call_item` fails
pub fn test_incoming_calls(
    mut test_case: TestCase,
    call_item: &CallHierarchyItem,
    expected: Option<&Vec<CallHierarchyIncomingCall>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::IncomingCalls);
    collect_results(
        &test_case,
        Some(&vec![(
            "CALL_ITEM",
            serde_json::to_string_pretty(call_item)
                .expect("JSON deserialzation of call item failed"),
        )]),
        expected,
        |expected, actual: &Vec<CallHierarchyIncomingCall>| {
            if expected != actual {
                Err(IncomingCallsMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                })?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'textDocument/prepareCallHierarchy' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON deserialization of `call_item` fails
pub fn test_moniker(
    mut test_case: TestCase,
    cursor_pos: &Position,
    expected: Option<&Vec<Moniker>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::Moniker);
    collect_results(
        &test_case,
        Some(&vec![get_cursor_replacement(cursor_pos)]),
        expected,
        |expected, actual: &Vec<Moniker>| {
            if expected != actual {
                Err(MonikerMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                })?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'textDocument/prepareCallHierarchy' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON deserialization of `call_item` fails
pub fn test_outgoing_calls(
    mut test_case: TestCase,
    call_item: &CallHierarchyItem,
    expected: Option<&Vec<CallHierarchyOutgoingCall>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::OutgoingCalls);
    collect_results(
        &test_case,
        Some(&vec![(
            "CALL_ITEM",
            serde_json::to_string_pretty(call_item)
                .expect("JSON deserialzation of call item failed"),
        )]),
        expected,
        |expected, actual: &Vec<CallHierarchyOutgoingCall>| {
            if expected != actual {
                Err(OutgoingCallsMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                })?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'textDocument/prepareCallHierarchy' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
pub fn test_prepare_call_hierarchy(
    mut test_case: TestCase,
    cursor_pos: &Position,
    expected: Option<&Vec<CallHierarchyItem>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::PrepareCallHierarchy);
    collect_results(
        &test_case,
        Some(&vec![get_cursor_replacement(cursor_pos)]),
        expected,
        |expected, actual: &Vec<CallHierarchyItem>| {
            if expected != actual {
                Err(PrepareCallHierachyMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                })?;
            }
            Ok(())
        },
    )
}

// NOTE: As far as I can tell, we can't directly accept a `PublishDiagnosticsParams` object,
// since diagnostics are requested via a `textDocument/publishDiagnostics` notification instead
// of a request. The `vim.lsp.buf_notify` method only returns a boolean to indicate success,
// so we can't access the actual data.
/// Tests the server's response to a 'textDocument/publishDiagnostics' request.
///
/// Specifying a `ServerStartType::Progress` for a diagnostics test is overloaded to
/// determine which `DiagnosticChanged` autocmd to use. This can be useful if your
/// server sends multiple `textDocument/publishDiagnostics` notifications before
/// fully analyzing a source file.
///
/// An `Option` is not used for `expected` because the LSP spec does not allow for
/// nil parameters in the `textDocument/publishDiagnostics` notification.
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
pub fn test_publish_diagnostics(
    mut test_case: TestCase,
    expected: &Vec<Diagnostic>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::PublishDiagnostics);
    collect_results(
        &test_case,
        None,
        Some(expected),
        |expected: &Vec<Diagnostic>, actual: &Vec<Diagnostic>| {
            if expected != actual {
                Err(PublishDiagnosticsMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                })?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'textDocument/references' request
///
/// # Errors
///
/// Returns `TestError` if the expected results don't match, or if some other failure occurs
pub fn test_references(
    mut test_case: TestCase,
    cursor_pos: &Position,
    include_declaration: bool,
    expected: Option<&Vec<Location>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::References);
    collect_results(
        &test_case,
        Some(&vec![
            get_cursor_replacement(cursor_pos),
            (
                "SET_CONTEXT",
                format!("context = {{ includeDeclaration = {include_declaration} }}"),
            ),
        ]),
        expected,
        |expected, actual: &Vec<Location>| {
            if expected != actual {
                Err(ReferencesMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                })?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'textDocument/rename' request
///
/// # Errors
///
/// Returns `TestError` if the expected results don't match, or if some other failure occurs
#[allow(clippy::missing_panics_doc)]
pub fn test_rename(
    mut test_case: TestCase,
    cursor_pos: &Position,
    new_name: &str,
    expected: Option<&WorkspaceEdit>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::Rename);
    collect_results(
        &test_case,
        Some(&vec![
            get_cursor_replacement(cursor_pos),
            ("NEW_NAME", format!("newName = '{new_name}'")),
        ]),
        expected,
        |expected, actual| {
            if expected != actual {
                Err(Box::new(RenameMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                }))?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'textDocument/typeDefinition' request
///
/// # Errors
///
/// Returns `TestError` if the expected results don't match, or if some other failure occurs
///
/// # Panics
///
/// Panics if JSON deserialization of `positions` fails
pub fn test_selection_range(
    mut test_case: TestCase,
    positions: &Vec<Position>,
    expected: Option<&Vec<SelectionRange>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::SelectionRange);
    let positions_str =
        serde_json::to_string_pretty(positions).expect("JSON deserialzation of `positions` failed");

    collect_results(
        &test_case,
        Some(&vec![("POSITIONS", positions_str)]),
        expected,
        |expected, actual: &Vec<SelectionRange>| {
            if expected != actual {
                Err(SelectionRangeMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                })?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'textDocument/semanticTokens/full' request
///
/// # Errors
///
/// Returns `TestError` if the expected results don't match, or if some other failure occurs
pub fn test_semantic_tokens_full(
    mut test_case: TestCase,
    expected: Option<&SemanticTokensResult>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::SemanticTokensFull);
    collect_results(&test_case, None, expected, |expected, actual| {
        // HACK: Since the `SemanticTokensResult` is untagged, there's no way to differentiate
        // between `SemanticTokensResult::Tokens` and `SemanticTokensResult::Partial`, as they
        // are structurally identical when we have
        // `SemanticTokensResult::Tokens(SemanticTokens { result_id: None, ...)`
        // Treat this as a special case and say it's ok.
        match (expected, actual) {
            (
                SemanticTokensResult::Tokens(SemanticTokens {
                    result_id: None,
                    data: token_data,
                }),
                SemanticTokensResult::Partial(SemanticTokensPartialResult { data: partial_data }),
            )
            | (
                SemanticTokensResult::Partial(SemanticTokensPartialResult { data: partial_data }),
                SemanticTokensResult::Tokens(SemanticTokens {
                    result_id: None,
                    data: token_data,
                }),
            ) => {
                if token_data == partial_data {
                    return Ok(());
                }
            }
            _ => {}
        }

        if expected != actual {
            Err(SemanticTokensFullMismatchError {
                test_id: test_case.test_id.clone(),
                expected: expected.clone(),
                actual: actual.clone(),
            })?;
        }
        Ok(())
    })
}

/// Tests the server's response to a 'textDocument/semanticTokens/full/delta' request
///
/// First sends a `textDocument/semanticTokens/full` request to get the initial state,
/// and then issues a `textDocument/semanticTokens/full/delta` request if the first
/// response contained a `result_id`.
///
/// # Errors
///
/// Returns `TestError` if the expected results don't match, or if some other failure occurs
pub fn test_semantic_tokens_full_delta(
    mut test_case: TestCase,
    expected: Option<&SemanticTokensFullDeltaResult>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::SemanticTokensFullDelta);
    collect_results(&test_case, None, expected, |expected, actual| {
        match (expected, actual) {
            (
                SemanticTokensFullDeltaResult::Tokens(SemanticTokens {
                    result_id: None,
                    data: token_data,
                }),
                SemanticTokensFullDeltaResult::TokensDelta(SemanticTokensDelta {
                    result_id: None,
                    edits: edit_data,
                }),
            )
            | (
                SemanticTokensFullDeltaResult::TokensDelta(SemanticTokensDelta {
                    result_id: None,
                    edits: edit_data,
                }),
                SemanticTokensFullDeltaResult::Tokens(SemanticTokens {
                    result_id: None,
                    data: token_data,
                }),
            ) if token_data.is_empty() && edit_data.is_empty() => return Ok(()),
            (
                SemanticTokensFullDeltaResult::Tokens(SemanticTokens {
                    result_id: None,
                    data: token_data,
                }),
                SemanticTokensFullDeltaResult::PartialTokensDelta {
                    edits: partial_data,
                },
            )
            | (
                SemanticTokensFullDeltaResult::PartialTokensDelta {
                    edits: partial_data,
                },
                SemanticTokensFullDeltaResult::Tokens(SemanticTokens {
                    result_id: None,
                    data: token_data,
                }),
            ) if token_data.is_empty() && partial_data.is_empty() => return Ok(()),
            (
                SemanticTokensFullDeltaResult::TokensDelta(SemanticTokensDelta {
                    result_id: None,
                    edits: edit_data,
                }),
                SemanticTokensFullDeltaResult::PartialTokensDelta {
                    edits: partial_data,
                },
            )
            | (
                SemanticTokensFullDeltaResult::PartialTokensDelta {
                    edits: partial_data,
                },
                SemanticTokensFullDeltaResult::TokensDelta(SemanticTokensDelta {
                    result_id: None,
                    edits: edit_data,
                }),
            ) if edit_data.is_empty() && partial_data.is_empty() => return Ok(()),
            (
                SemanticTokensFullDeltaResult::Tokens(SemanticTokens {
                    result_id: None,
                    data: token_data,
                }),
                SemanticTokensFullDeltaResult::PartialTokensDelta { edits: edit_data },
            )
            | (
                SemanticTokensFullDeltaResult::PartialTokensDelta { edits: edit_data },
                SemanticTokensFullDeltaResult::Tokens(SemanticTokens {
                    result_id: None,
                    data: token_data,
                }),
            ) if edit_data.is_empty() && token_data.is_empty() => return Ok(()),
            _ => {}
        }
        if expected != actual {
            Err(Box::new(SemanticTokensFullDeltaMismatchError {
                test_id: test_case.test_id.clone(),
                expected: expected.clone(),
                actual: actual.clone(),
            }))?;
        }
        Ok(())
    })
}

/// Tests the server's response to a 'textDocument/semanticTokens/range' request
///
/// # Errors
///
/// Returns `TestError` if the expected results don't match, or if some other failure occurs
///
/// # Panics
///
/// Panics if JSON deserialization of `range` fails
pub fn test_semantic_tokens_range(
    mut test_case: TestCase,
    range: &Range,
    expected: Option<&SemanticTokensRangeResult>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::SemanticTokensRange);

    let range_json =
        serde_json::to_string_pretty(range).expect("JSON deserialzation of range failed");
    collect_results(
        &test_case,
        Some(&vec![("RANGE", range_json)]),
        expected,
        |expected, actual| {
            // HACK: Since the `SemanticTokensRangeResult` is untagged, there's no way
            // to differentiate between `SemanticTokensRangeResult::Tokens` and
            // `SemanticTokensRangeResult::Partial`, as they are structurally identical
            // when we have `SemanticTokensResult::Tokens(SemanticTokens { result_id: None, ...)`
            // Treat this as a special case and say it's ok.
            match (expected, actual) {
                (
                    SemanticTokensRangeResult::Tokens(SemanticTokens {
                        result_id: None,
                        data: token_data,
                    }),
                    SemanticTokensRangeResult::Partial(SemanticTokensPartialResult {
                        data: partial_data,
                    }),
                )
                | (
                    SemanticTokensRangeResult::Partial(SemanticTokensPartialResult {
                        data: partial_data,
                    }),
                    SemanticTokensRangeResult::Tokens(SemanticTokens {
                        result_id: None,
                        data: token_data,
                    }),
                ) => {
                    if token_data == partial_data {
                        return Ok(());
                    }
                }
                _ => {}
            }

            if expected != actual {
                Err(SemanticTokensRangeMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                })?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'textDocument/signatureHelp' request
///
/// # Errors
///
/// Returns `TestError` if the expected results don't match, or if some other failure occurs
///
/// # Panics
///
/// Panics if JSON deserialization of `context` fails
pub fn test_signature_help(
    mut test_case: TestCase,
    cursor_pos: &Position,
    context: Option<&SignatureHelpContext>,
    expected: Option<&SignatureHelp>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::SignatureHelp);
    let context = context.map_or_else(
        || "null".to_string(),
        |ctx| {
            serde_json::to_string_pretty(ctx)
                .expect("JSON deserialzation of signature help context failed")
        },
    );
    collect_results(
        &test_case,
        Some(&vec![
            get_cursor_replacement(cursor_pos),
            ("SIGNATURE_CONTEXT", context),
        ]),
        expected,
        |expected, actual| {
            if expected != actual {
                Err(SignatureHelpMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                })?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'textDocument/typeDefinition' request
///
/// # Errors
///
/// Returns `TestError` if the expected results don't match, or if some other failure occurs
pub fn test_type_definition(
    mut test_case: TestCase,
    cursor_pos: &Position,
    expected: Option<&GotoTypeDefinitionResponse>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::TypeDefinition);
    collect_results(
        &test_case,
        Some(&vec![get_cursor_replacement(cursor_pos)]),
        expected,
        |expected, actual| {
            // HACK: Since the `GotoTypeDefinitionResponse` is untagged, there's no way
            // to differentiate between the `Array` and `Link` if we get an empty vector
            // in response. Just treat this as a special case and say it's ok.
            match (expected, actual) {
                (
                    GotoTypeDefinitionResponse::Array(array_items),
                    GotoTypeDefinitionResponse::Link(link_items),
                )
                | (
                    GotoTypeDefinitionResponse::Link(link_items),
                    GotoTypeDefinitionResponse::Array(array_items),
                ) => {
                    if array_items.is_empty() && link_items.is_empty() {
                        return Ok(());
                    }
                }
                _ => {}
            }

            if *expected != *actual {
                Err(Box::new(TypeDefinitionMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                }))?;
            }
            Ok(())
        },
    )
}

/// Tests the server's response to a 'workspace/diagnostic' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON deserialization of `identifier` or `previous_result_id` fails
pub fn test_workspace_diagnostic(
    mut test_case: TestCase,
    identifier: Option<String>,
    previous_result_ids: &Vec<PreviousResultId>,
    expected: &WorkspaceDiagnosticReport,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::WorkspaceDiagnostic);
    let identifier = identifier.map_or_else(
        || "null".to_string(), // NOTE: `vim.json.decode()` fails with an empty string
        |id| serde_json::to_string_pretty(&id).expect("JSON deserialzation of identifier failed"),
    );
    let previous_result_ids = serde_json::to_string_pretty(previous_result_ids)
        .expect("JSON deserialzation of previous result id failed");
    collect_results(
        &test_case,
        Some(&vec![
            ("IDENTIFIER", identifier),
            ("PREVIOUS_RESULT_ID", previous_result_ids),
        ]),
        Some(expected),
        |expected: &WorkspaceDiagnosticReport, actual: &WorkspaceDiagnosticReport| {
            if expected != actual {
                Err(Box::new(WorkspaceDiagnosticMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                }))?;
            }
            Ok(())
        },
    )
}
