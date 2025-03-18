mod init_dot_lua;
pub mod types;

use lsp_types::{
    request::{GotoDeclarationResponse, GotoImplementationResponse, GotoTypeDefinitionResponse},
    CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall, CodeLens, Diagnostic,
    DocumentHighlight, DocumentLink, DocumentSymbolResponse, FormattingOptions,
    GotoDefinitionResponse, Hover, Location, TextEdit, WorkspaceEdit,
};

use std::{
    collections::HashMap,
    fs,
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, Condvar, Mutex, OnceLock},
};

use types::{
    CleanResponse, CodeLensMismatchError, CodeLensResolveMismatchError, CompletionMismatchError,
    CompletionResult, DeclarationMismatchError, DefinitionMismatchError, DiagnosticMismatchError,
    DocumentHighlightMismatchError, DocumentLinkMismatchError, DocumentLinkResolveMismatchError,
    DocumentSymbolMismatchError, Empty, EmptyResult, FormattingMismatchError, FormattingResult,
    HoverMismatchError, ImplementationMismatchError, IncomingCallsMismatchError,
    OutgoingCallsMismatchError, PrepareCallHierachyMismatchError, ReferencesMismatchError,
    RenameMismatchError, TestCase, TestError, TestResult, TestSetupError, TestType, TimeoutError,
    TypeDefinitionMismatchError,
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
                Err(CodeLensResolveMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: (*expected).clone(),
                    actual: actual.clone(),
                })?;
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
    expected: Option<&CompletionResult>,
) -> TestResult<()> {
    if test_case.cursor_pos.is_none() {
        Err(TestSetupError::InvalidCursorPosition(TestType::Completion))?;
    }
    test_case.test_type = Some(TestType::Completion);

    collect_results(&test_case, None, expected, |expected, actual| {
        if !expected.results_satisfy(actual) {
            Err(CompletionMismatchError {
                test_id: test_case.test_id.clone(),
                expected: (*expected).clone(),
                actual: actual.clone(),
            })?;
        }
        Ok(())
    })
}

/// Tests the server's response to a 'textDocument/declaration' request
///
/// # Errors
///
/// Returns `TestError` if the expected results don't match, or if some other failure occurs
pub fn test_declaration(
    mut test_case: TestCase,
    expected: Option<&GotoDeclarationResponse>,
) -> TestResult<()> {
    if test_case.cursor_pos.is_none() {
        Err(TestSetupError::InvalidCursorPosition(TestType::Declaration))?;
    }
    test_case.test_type = Some(TestType::Declaration);
    collect_results(&test_case, None, expected, |expected, actual| {
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
    })
}

/// Tests the server's response to a 'textDocument/definition' request
///
/// # Errors
///
/// Returns `TestError` if the expected results don't match, or if some other failure occurs
pub fn test_definition(
    mut test_case: TestCase,
    expected: Option<&GotoDefinitionResponse>,
) -> TestResult<()> {
    if test_case.cursor_pos.is_none() {
        Err(TestSetupError::InvalidCursorPosition(TestType::Definition))?;
    }
    test_case.test_type = Some(TestType::Definition);
    collect_results(&test_case, None, expected, |expected, actual| {
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
    })
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
pub fn test_diagnostics(mut test_case: TestCase, expected: &Vec<Diagnostic>) -> TestResult<()> {
    test_case.test_type = Some(TestType::Diagnostic);
    collect_results(
        &test_case,
        None,
        Some(expected),
        |expected: &Vec<Diagnostic>, actual: &Vec<Diagnostic>| {
            if expected != actual {
                Err(DiagnosticMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                })?;
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
    expected: Option<&Vec<DocumentHighlight>>,
) -> TestResult<()> {
    if test_case.cursor_pos.is_none() {
        Err(TestSetupError::InvalidCursorPosition(
            TestType::DocumentHighlight,
        ))?;
    }
    test_case.test_type = Some(TestType::DocumentHighlight);
    collect_results(
        &test_case,
        None,
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
                Err(DocumentLinkResolveMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                })?;
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
pub fn test_hover(mut test_case: TestCase, expected: Option<&Hover>) -> TestResult<()> {
    if test_case.cursor_pos.is_none() {
        Err(TestSetupError::InvalidCursorPosition(TestType::Hover))?;
    }
    test_case.test_type = Some(TestType::Hover);
    collect_results(&test_case, None, expected, |expected, actual| {
        if expected != actual {
            Err(Box::new(HoverMismatchError {
                test_id: test_case.test_id.clone(),
                expected: expected.clone(),
                actual: actual.clone(),
            }))?;
        }
        Ok(())
    })
}

/// Tests the server's response to a 'textDocument/implementation' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
pub fn test_implementation(
    mut test_case: TestCase,
    expected: Option<&GotoImplementationResponse>,
) -> TestResult<()> {
    if test_case.cursor_pos.is_none() {
        Err(TestSetupError::InvalidCursorPosition(
            TestType::Implementation,
        ))?;
    }
    test_case.test_type = Some(TestType::Implementation);
    collect_results(&test_case, None, expected, |expected, actual| {
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
    })
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
    expected: Option<&Vec<CallHierarchyItem>>,
) -> TestResult<()> {
    if test_case.cursor_pos.is_none() {
        Err(TestSetupError::InvalidCursorPosition(
            TestType::PrepareCallHierarchy,
        ))?;
    }
    test_case.test_type = Some(TestType::PrepareCallHierarchy);
    collect_results(
        &test_case,
        None,
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

/// Tests the server's response to a 'textDocument/references' request
///
/// # Errors
///
/// Returns `TestError` if the expected results don't match, or if some other failure occurs
pub fn test_references(
    mut test_case: TestCase,
    include_declaration: bool,
    expected: Option<&Vec<Location>>,
) -> TestResult<()> {
    if test_case.cursor_pos.is_none() {
        Err(TestSetupError::InvalidCursorPosition(TestType::References))?;
    }
    test_case.test_type = Some(TestType::References);
    collect_results(
        &test_case,
        Some(&vec![(
            "SET_CONTEXT",
            format!("context = {{ includeDeclaration = {include_declaration} }}"),
        )]),
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
    new_name: &str,
    expected: Option<&WorkspaceEdit>,
) -> TestResult<()> {
    if test_case.cursor_pos.is_none() {
        Err(TestSetupError::InvalidCursorPosition(TestType::Rename))?;
    }
    test_case.test_type = Some(TestType::Rename);
    collect_results(
        &test_case,
        Some(&vec![("NEW_NAME", format!("newName = '{new_name}'"))]),
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
pub fn test_type_definition(
    mut test_case: TestCase,
    expected: Option<&GotoTypeDefinitionResponse>,
) -> TestResult<()> {
    if test_case.cursor_pos.is_none() {
        Err(TestSetupError::InvalidCursorPosition(
            TestType::TypeDefinition,
        ))?;
    }
    test_case.test_type = Some(TestType::TypeDefinition);
    collect_results(&test_case, None, expected, |expected, actual| {
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
    })
}
