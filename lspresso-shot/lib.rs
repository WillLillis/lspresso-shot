mod init_dot_lua;
pub mod types;

use init_dot_lua::LuaReplacement;
use lsp_types::{
    CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall, CodeAction,
    CodeActionContext, CodeActionResponse, CodeLens, ColorInformation, CompletionItem,
    CompletionResponse, Diagnostic, DocumentDiagnosticReport, DocumentHighlight, DocumentLink,
    DocumentSymbolResponse, FoldingRange, FormattingOptions, GotoDefinitionResponse, Hover,
    InlayHint, Location, Moniker, Position, PreviousResultId, Range, SelectionRange,
    SemanticTokens, SemanticTokensDelta, SemanticTokensFullDeltaResult,
    SemanticTokensPartialResult, SemanticTokensRangeResult, SemanticTokensResult, SignatureHelp,
    SignatureHelpContext, TextEdit, TypeHierarchyItem, WorkspaceDiagnosticReport, WorkspaceEdit,
    request::{GotoDeclarationResponse, GotoImplementationResponse, GotoTypeDefinitionResponse},
};

// These imports are included for the sake of doc comments, they aren't used
#[allow(unused_imports)]
use lsp_types::{
    CallHierarchyIncomingCallsParams, CallHierarchyOutgoingCallsParams, CallHierarchyPrepareParams,
    CodeActionParams, CompletionParams, DocumentDiagnosticParams, DocumentHighlightParams,
    GotoDefinitionParams, HoverParams, InlayHintParams, MonikerParams, ReferenceParams,
    RenameParams, SelectionRangeParams, SemanticTokensRangeParams, SignatureHelpParams,
    TypeHierarchyPrepareParams, WorkspaceDiagnosticParams,
    request::{GotoDeclarationParams, GotoImplementationParams, GotoTypeDefinitionParams},
};
#[allow(unused_imports)]
use types::ServerStartType;

use std::{
    collections::HashMap,
    fs,
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, Condvar, Mutex, OnceLock},
};

use types::{
    CleanResponse, Empty, EmptyResult, TestCase, TestError, TestResult, TestType, TimeoutError,
    call_hierarchy::{
        IncomingCallsMismatchError, OutgoingCallsMismatchError, PrepareCallHierachyMismatchError,
    },
    code_action::{CodeActionMismatchError, CodeActionResolveMismatchError},
    code_lens::{CodeLensMismatchError, CodeLensResolveMismatchError},
    completion::{CompletionMismatchError, CompletionResolveMismatchError},
    declaration::DeclarationMismatchError,
    definition::DefinitionMismatchError,
    diagnostic::{
        DiagnosticMismatchError, PublishDiagnosticsMismatchError, WorkspaceDiagnosticMismatchError,
    },
    document_color::DocumentColorMismatchError,
    document_highlight::DocumentHighlightMismatchError,
    document_link::{DocumentLinkMismatchError, DocumentLinkResolveMismatchError},
    document_symbol::DocumentSymbolMismatchError,
    folding_range::FoldingRangeMismatchError,
    formatting::{FormattingMismatchError, FormattingResult},
    hover::HoverMismatchError,
    implementation::ImplementationMismatchError,
    inlay_hint::InlayHintMismatchError,
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
    type_hierarchy::PrepareTypeHierarchyMismatchError,
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
/// expecting `Some(_)` results, then `R1 == R2`. If `None` is expected, `R1` is [`EmptyResult`]
///
/// If `R1` is `Empty`, we expect empty results and this will only return `Err`
/// or `Ok(None)`.
///
/// Otherwise, we expect some results, and this function will return `Ok(Some(_))` or `Err`.
fn test_inner<R1, R2>(
    test_case: &TestCase,
    replacements: &mut Vec<LuaReplacement>,
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
        (_, true, true) => unreachable!(),
        // No results
        (_, false, false) => Err(TestError::NoResults(test_case.test_id.clone()))?,
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
    replacements: &mut Vec<LuaReplacement>,
    expected: Option<&R1>,
    cmp: impl Fn(&R1, &R2) -> TestResult<()>,
) -> TestResult<()>
where
    R1: std::fmt::Debug,
    R2: serde::de::DeserializeOwned + std::fmt::Debug + Empty + CleanResponse,
{
    if let Some(expected) = expected {
        let actual =
            test_inner::<R2, R2>(test_case, replacements)?.expect("Expected results, got `None`");
        Ok(cmp(expected, &actual)?)
    } else {
        let empty = test_inner::<EmptyResult, R2>(test_case, replacements)?;
        assert!(empty.is_none());
        Ok(())
    }
}

pub type CodeActionComparator = fn(&CodeActionResponse, &CodeActionResponse, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/codeAction`] request
///
/// - `range`: Passed to the client via the request's [`CodeActionParams`]
/// - `context`: Passed to the client via the request's [`CodeActionParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON serialization of `range` or `context` fails
///
/// [`textDocument/codeAction`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_codeAction
pub fn test_code_action(
    mut test_case: TestCase,
    range: &Range,
    context: &CodeActionContext,
    cmp: Option<CodeActionComparator>,
    expected: Option<&CodeActionResponse>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::CodeAction);
    let range_json =
        serde_json::to_string_pretty(range).expect("JSON deserialzation of `range` failed");
    let context_json =
        serde_json::to_string_pretty(context).expect("JSON deserialzation of `params` failed");

    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamDirect {
                name: "range",
                json: range_json,
            },
            LuaReplacement::ParamDirect {
                name: "context",
                json: context_json,
            },
        ],
        expected,
        |expected, actual: &CodeActionResponse| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
                Err(CodeActionMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: (*expected).clone(),
                    actual: actual.clone(),
                })?;
            }
            Ok(())
        },
    )
}

pub type CodeActionResolveComparator = fn(&CodeAction, &CodeAction, &TestCase) -> bool;

/// Tests the server's response to a [`codeLens/resolve`] request
///
/// - `params`: Passed to the client via the request's [`CodeAction`] param
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON serialization of `params` fails
///
/// [`codeLens/resolve`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#codeLens_resolve
pub fn test_code_action_resolve(
    mut test_case: TestCase,
    params: &CodeAction,
    cmp: Option<CodeActionResolveComparator>,
    expected: &CodeAction,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::CodeActionResolve);
    let code_action_json =
        serde_json::to_string_pretty(params).expect("JSON deserialzation of params failed");

    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamDestructure {
            name: "code_action",
            fields: vec![
                "title",
                "kind",
                "diagnostics",
                "edit",
                "command",
                "isPreferred",
                "disabled",
                "data",
            ],
            json: code_action_json,
        }],
        Some(expected),
        |expected, actual: &CodeAction| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
                Err(CodeActionResolveMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: (*expected).clone(),
                    actual: actual.clone(),
                })?;
            }
            Ok(())
        },
    )
}

pub type CodeLensComparator = fn(&Vec<CodeLens>, &Vec<CodeLens>, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/codeLens`] request
///
/// - `commands`: A list of LSP command names the client should advertise support for in its
///   capabilities (e.g. "rust-analyzer.runSingle"). This enables command-based `CodeLens`
///   responses from the server, such as "Run" or "Debug" actions.
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/codeLens`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_codeLens
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
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::Other {
                from: "COMMANDS",
                to: command_str,
            },
        ],
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

/// Tests the server's response to a [`codeLens/resolve`] request
///
/// - `commands` is a list of LSP command names the client should advertise support for in its
///   capabilities (e.g. "rust-analyzer.runSingle"). This enables command-based [`CodeLens`]
///   responses from the server, such as "Run" or "Debug" actions.
/// - `code_lens`: Passed to the client via the request's [`CodeLens`] param
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON serialization of `code_lens` fails
///
/// [`codeLens/resolve`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#codeLens_resolve
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
        &mut vec![
            LuaReplacement::ParamDestructure {
                name: "code_lens",
                fields: vec!["range", "data", "command"],
                json: code_lens_json,
            },
            LuaReplacement::Other {
                from: "COMMANDS",
                to: command_str,
            },
        ],
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

pub type CompletionComparator = fn(&CompletionResponse, &CompletionResponse, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/completion`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`CompletionParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/completion`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_completion
pub fn test_completion(
    mut test_case: TestCase,
    cursor_pos: &Position,
    cmp: Option<CompletionComparator>,
    expected: Option<&CompletionResponse>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::Completion);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition(*cursor_pos),
        ],
        expected,
        |expected, actual| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type CompletionResolveComparator = fn(&CompletionItem, &CompletionItem, &TestCase) -> bool;

/// Tests the server's response to a [`completionItem/resolve`] request
///
/// - `completion_item`: Passed to the client via the request's [`CompletionItem`] param
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON serialization of `completion_item` fails
///
/// [`completionItem/resolve`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#completionItem_resolve
pub fn test_completion_resolve(
    mut test_case: TestCase,
    completion_item: &CompletionItem,
    cmp: Option<CompletionResolveComparator>,
    expected: Option<&CompletionItem>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::CompletionResolve);

    let completion_item_json = serde_json::to_string_pretty(completion_item)
        .expect("JSON deserialzation of completion item failed");
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamDestructure {
            name: "completion_item",
            fields: vec![
                "label",
                "labelDetails",
                "kind",
                "tags",
                "detail",
                "documentation",
                "deprecated",
                "preselect",
                "sortText",
                "filterText",
                "insertText",
                "insertTextFormat",
                "insertTextMode",
                "textEdit",
                "textEditText",
                "additionalTextEdits",
                "commitCharacters",
                "command",
                "data",
            ],
            json: completion_item_json,
        }],
        expected,
        |expected, actual| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type DeclarationComparator =
    fn(&GotoDeclarationResponse, &GotoDeclarationResponse, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/declaration`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`GotoDeclarationParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/declaration`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_declaration
pub fn test_declaration(
    mut test_case: TestCase,
    cursor_pos: &Position,
    cmp: Option<DeclarationComparator>,
    expected: Option<&GotoDeclarationResponse>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::Declaration);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition(*cursor_pos),
        ],
        expected,
        |expected, actual| {
            let eql = cmp.as_ref().map_or_else(
                || {
                    // HACK: Since the `GotoDeclarationResponse` is untagged, there's no way to differentiate
                    // between the `Array` and `Link` if we get an empty vector in response. Just
                    // treat this as a special case and say it's ok.
                    let mut eql_result = false;
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
                                eql_result = true;
                            }
                        }
                        _ => eql_result = expected == actual,
                    }
                    eql_result
                },
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type DefinitionComparator =
    fn(&GotoDefinitionResponse, &GotoDefinitionResponse, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/definition`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`GotoDefinitionParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the expected results don't match, or if some other failure occurs
///
/// [`textDocument/definition`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_definition
pub fn test_definition(
    mut test_case: TestCase,
    cursor_pos: &Position,
    cmp: Option<DefinitionComparator>,
    expected: Option<&GotoDefinitionResponse>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::Definition);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition(*cursor_pos),
        ],
        expected,
        |expected, actual| {
            let eql = cmp.as_ref().map_or_else(
                || {
                    // HACK: Since the `GotoDefinitionResponse` is untagged, there's no way to differentiate
                    // between the `Array` and `Link` if we get an empty vector in response. Just
                    // treat this as a special case and say it's ok.
                    let mut eql_result = false;
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
                                eql_result = true;
                            }
                        }
                        _ => eql_result = expected == actual,
                    }
                    eql_result
                },
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type DiagnosticComparator =
    fn(&DocumentDiagnosticReport, &DocumentDiagnosticReport, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/diagnostic`] request
///
/// - `identifier`: Passed to the client via the request's [`DocumentDiagnosticParams`]
/// - `previous_result_id`: Passed to the client via the request's [`DocumentDiagnosticParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON serialization of `identifier` or `previous_result_id` fails
///
/// [`textDocument/diagnostic`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_diagnostic
pub fn test_diagnostic(
    mut test_case: TestCase,
    identifier: Option<&str>,
    previous_result_id: Option<&str>,
    cmp: Option<DiagnosticComparator>,
    expected: &DocumentDiagnosticReport,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::Diagnostic);
    let identifier_json = identifier.map_or_else(
        || "null".to_string(), // NOTE: `vim.json.decode()` fails with an empty string
        |id| serde_json::to_string_pretty(id).expect("JSON deserialzation of identifier failed"),
    );
    let previous_result_id_json = previous_result_id.map_or_else(
        || "null".to_string(), // NOTE: `vim.json.decode()` fails with an empty string
        |id| {
            serde_json::to_string_pretty(id)
                .expect("JSON deserialzation of previous result id failed")
        },
    );
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamDirect {
                name: "identifier",
                json: identifier_json,
            },
            LuaReplacement::ParamDirect {
                name: "previousResultId",
                json: previous_result_id_json,
            },
        ],
        Some(expected),
        |expected: &DocumentDiagnosticReport, actual: &DocumentDiagnosticReport| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type DocumentColorComparator =
    fn(&Vec<ColorInformation>, &Vec<ColorInformation>, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/documentColor`] request
///
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/documentColor`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_documentColor
pub fn test_document_color(
    mut test_case: TestCase,
    cmp: Option<DocumentColorComparator>,
    expected: &Vec<ColorInformation>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::DocumentColor);
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamTextDocument],
        Some(expected),
        |expected, actual: &Vec<ColorInformation>| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
                Err(DocumentColorMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                })?;
            }
            Ok(())
        },
    )
}

pub type DocumentHighlightComparator =
    fn(&Vec<DocumentHighlight>, &Vec<DocumentHighlight>, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/documentHighlight`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`DocumentHighlightParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/documentHighlight`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_documentHighlight
pub fn test_document_highlight(
    mut test_case: TestCase,
    cursor_pos: &Position,
    cmp: Option<DocumentHighlightComparator>,
    expected: Option<&Vec<DocumentHighlight>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::DocumentHighlight);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition(*cursor_pos),
        ],
        expected,
        |expected, actual: &Vec<DocumentHighlight>| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type DocumentLinkComparator = fn(&Vec<DocumentLink>, &Vec<DocumentLink>, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/documentLink`] request
///
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/documentLink`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_documentLink
pub fn test_document_link(
    mut test_case: TestCase,
    cmp: Option<DocumentLinkComparator>,
    expected: Option<&Vec<DocumentLink>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::DocumentLink);
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamTextDocument],
        expected,
        |expected, actual: &Vec<DocumentLink>| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type DocumentLinkResolveComparator = fn(&DocumentLink, &DocumentLink, &TestCase) -> bool;

/// Tests the server's response to a [`documentLink/resolve`] request
///
/// - `link`: Passed to the client via the request's [`DocumentLink`] params
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON serialization of `link` fails
///
/// [`documentLink/resolve`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#documentLink_resolve
pub fn test_document_link_resolve(
    mut test_case: TestCase,
    params: &DocumentLink,
    cmp: Option<DocumentLinkResolveComparator>,
    expected: Option<&DocumentLink>,
) -> TestResult<()> {
    let document_link_json =
        serde_json::to_string_pretty(params).expect("JSON deserialzation of document link failed");
    test_case.test_type = Some(TestType::DocumentLinkResolve);
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamDestructure {
            name: "link",
            fields: vec!["range", "target", "tooltip", "data"],
            json: document_link_json,
        }],
        expected,
        |expected, actual: &DocumentLink| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type DocumentSymbolComparator =
    fn(&DocumentSymbolResponse, &DocumentSymbolResponse, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/documentSymbol`] request
///
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/documentSymbol`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_documentSymbol
pub fn test_document_symbol(
    mut test_case: TestCase,
    cmp: Option<DocumentSymbolComparator>,
    expected: Option<&DocumentSymbolResponse>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::DocumentSymbol);
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamTextDocument],
        expected,
        |expected, actual| {
            let eql = cmp.as_ref().map_or_else(
                || {
                    // HACK: Since the two types of DocumentSymbolResponse are untagged, there's no
                    // way to differentiate between the two if we get an empty vector in response.
                    // Just treat this as a special case and say it's ok.
                    let mut eql_result = false;
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
                                eql_result = true;
                            }
                        }
                        _ => eql_result = expected == actual,
                    }
                    eql_result
                },
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
                Err(DocumentSymbolMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                })?;
            }

            Ok(())
        },
    )
}

pub type FoldingRangeComparator = fn(&Vec<FoldingRange>, &Vec<FoldingRange>, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/foldingRange`] request
///
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/foldingRange`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_foldingRange
pub fn test_folding_range(
    mut test_case: TestCase,
    cmp: Option<FoldingRangeComparator>,
    expected: Option<&Vec<FoldingRange>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::FoldingRange);
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamTextDocument],
        expected,
        |expected, actual: &Vec<FoldingRange>| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type FormattingComparator = fn(&Vec<TextEdit>, &Vec<TextEdit>, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/formatting`] request.
///
/// - `options`:  The formatting options passed to the LSP client. If `None`, then
///   the following default is used:
///
/// ```rust
/// lsp_types::FormattingOptions {
///     tab_size: 4,
///     insert_spaces: true,
///     properties: std::collections::HashMap::new(),
///     trim_trailing_whitespace: Some(true),
///     insert_final_newline: Some(true),
///     trim_final_newlines: Some(true),
/// };
/// ```
///
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results. Note that a custom comparator is only
///   availble for the `FormattingResult::Response` variant.
///
/// # Errors
///
/// Returns [`TestError`] if the expected results don't match, or if some other failure occurs
///
/// # Panics
///
/// Panics if JSON serialization of `options` fails
///
/// [`textDocument/formatting`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_formatting
pub fn test_formatting(
    mut test_case: TestCase,
    options: Option<FormattingOptions>,
    cmp: Option<FormattingComparator>,
    expected: Option<&FormattingResult>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::Formatting);
    let opts = options.unwrap_or_else(|| FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        properties: HashMap::new(),
        trim_trailing_whitespace: Some(true),
        insert_final_newline: Some(true),
        trim_final_newlines: Some(true),
    });
    let format_opts_json = serde_json::to_string_pretty(&opts)
        .expect("JSON deserialzation of formatting options failed");
    match expected {
        Some(FormattingResult::Response(edits)) => collect_results(
            &test_case,
            &mut vec![
                LuaReplacement::Other {
                    from: "INVOKE_FORMAT",
                    to: "false".to_string(),
                },
                LuaReplacement::ParamDirect {
                    name: "options",
                    json: format_opts_json,
                },
            ],
            Some(edits),
            |expected, actual: &Vec<TextEdit>| {
                let eql = cmp.as_ref().map_or_else(
                    || expected == actual,
                    |cmp_fn| cmp_fn(expected, actual, &test_case),
                );
                if !eql {
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
            &mut vec![
                LuaReplacement::ParamTextDocument,
                LuaReplacement::Other {
                    from: "INVOKE_FORMAT",
                    to: "true".to_string(),
                },
                LuaReplacement::ParamDirect {
                    name: "options",
                    json: format_opts_json,
                },
            ],
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
            &mut vec![
                LuaReplacement::Other {
                    from: "INVOKE_FORMAT",
                    to: "false".to_string(),
                },
                LuaReplacement::ParamDirect {
                    name: "options",
                    json: format_opts_json,
                },
            ],
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

pub type HoverComparator = fn(&Hover, &Hover, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/hover`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`HoverParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/hover`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_hover
pub fn test_hover(
    mut test_case: TestCase,
    cursor_pos: &Position,
    cmp: Option<HoverComparator>,
    expected: Option<&Hover>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::Hover);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition(*cursor_pos),
        ],
        expected,
        |expected, actual| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type ImplementationComparator =
    fn(&GotoImplementationResponse, &GotoImplementationResponse, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/implementation`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`GotoImplementationParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/implementation`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_implementation
pub fn test_implementation(
    mut test_case: TestCase,
    cursor_pos: &Position,
    cmp: Option<ImplementationComparator>,
    expected: Option<&GotoImplementationResponse>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::Implementation);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition(*cursor_pos),
        ],
        expected,
        |expected, actual| {
            let eql = cmp.as_ref().map_or_else(
                || {
                    // HACK: Since `GotoImplementationResponse` is untagged, there is no way to
                    // differentiate between the `Array` and `Link` variants if we get an empty
                    // vector in response.
                    // Just treat this as a special case and say it's ok.
                    let mut eql_result = false;
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
                                eql_result = true;
                            }
                        }
                        _ => eql_result = expected == actual,
                    }
                    eql_result
                },
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type IncomingCallsComparator =
    fn(&Vec<CallHierarchyIncomingCall>, &Vec<CallHierarchyIncomingCall>, &TestCase) -> bool;

/// Tests the server's response to a [`callHierarchy/incomingCalls`] request
///
/// - `call_item`: Passed to the client via the request's [`CallHierarchyIncomingCallsParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON serialization of `call_item` fails
///
/// [`callHierarchy/incomingCalls`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#callHierarchy_incomingCalls
pub fn test_incoming_calls(
    mut test_case: TestCase,
    call_item: &CallHierarchyItem,
    cmp: Option<IncomingCallsComparator>,
    expected: Option<&Vec<CallHierarchyIncomingCall>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::IncomingCalls);
    let call_item_json =
        serde_json::to_string_pretty(call_item).expect("JSON deserialzation of call item failed");
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamDirect {
            name: "item",
            json: call_item_json,
        }],
        expected,
        |expected, actual: &Vec<CallHierarchyIncomingCall>| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type InlayHintComparator = fn(&Vec<InlayHint>, &Vec<InlayHint>, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/inlayHint`] request
///
/// - `range`: Passed to the client via the request's [`InlayHintParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON serialization of `range` fails
///
/// [`textDocument/inlayHint`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_inlayHint
pub fn test_inlay_hint(
    mut test_case: TestCase,
    range: &Range,
    cmp: Option<InlayHintComparator>,
    expected: Option<&Vec<InlayHint>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::InlayHint);
    let range_json =
        serde_json::to_string_pretty(range).expect("JSON serialzation of range failed");
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamDirect {
                name: "range",
                json: range_json,
            },
        ],
        expected,
        |expected, actual: &Vec<InlayHint>| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
                Err(InlayHintMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                })?;
            }
            Ok(())
        },
    )
}

pub type MonikerComparator = fn(&Vec<Moniker>, &Vec<Moniker>, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/moniker`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`MonikerParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON serialization of `call_item` fails
///
/// [`textDocument/moniker`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_moniker
pub fn test_moniker(
    mut test_case: TestCase,
    cursor_pos: &Position,
    cmp: Option<MonikerComparator>,
    expected: Option<&Vec<Moniker>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::Moniker);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition(*cursor_pos),
        ],
        expected,
        |expected, actual: &Vec<Moniker>| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type OutgoingCallsComparator =
    fn(&Vec<CallHierarchyOutgoingCall>, &Vec<CallHierarchyOutgoingCall>, &TestCase) -> bool;

/// Tests the server's response to a [`callHierarchy/outgoingCalls`] request
///
/// - `call_item`: Passed to the client via the request's [`CallHierarchyOutgoingCallsParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON serialization of `call_item` fails
///
/// [`callHierarchy/outgoingCalls`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#callHierarchy_outgoingCalls
pub fn test_outgoing_calls(
    mut test_case: TestCase,
    call_item: &CallHierarchyItem,
    cmp: Option<OutgoingCallsComparator>,
    expected: Option<&Vec<CallHierarchyOutgoingCall>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::OutgoingCalls);
    let call_item_json =
        serde_json::to_string_pretty(call_item).expect("JSON deserialzation of call item failed");
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamDirect {
            name: "item",
            json: call_item_json,
        }],
        expected,
        |expected, actual| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type PrepareCallHierarchyComparator =
    fn(&Vec<CallHierarchyItem>, &Vec<CallHierarchyItem>, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/prepareCallHierarchy`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`CallHierarchyPrepareParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/prepareCallHierarchy`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_prepareCallHierarchy
pub fn test_prepare_call_hierarchy(
    mut test_case: TestCase,
    cursor_pos: &Position,
    cmp: Option<PrepareCallHierarchyComparator>,
    expected: Option<&Vec<CallHierarchyItem>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::PrepareCallHierarchy);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition(*cursor_pos),
        ],
        expected,
        |expected, actual: &Vec<CallHierarchyItem>| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type PrepareTypeHierarchyComparator =
    fn(&Vec<TypeHierarchyItem>, &Vec<TypeHierarchyItem>, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/prepareTypeHierarchy`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`TypeHierarchyPrepareParams`]
/// - `items`: Type hierarchy items provided to the client via [`TypeHierarchyPrepareParams`].
///   The `uri` field of each item should be *relative* to the test case root, instead of an
///   absolute path. (i.e. `uri = "file://src/test_file.rs"`)
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON serialization of `items` fails
///
/// [`textDocument/prepareTypeHierarchy`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_prepareTypeHierarchy
pub fn test_prepare_type_hierarchy(
    mut test_case: TestCase,
    cursor_pos: &Position,
    items: Option<&Vec<TypeHierarchyItem>>,
    cmp: Option<PrepareTypeHierarchyComparator>,
    expected: Option<&Vec<TypeHierarchyItem>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::PrepareTypeHierarchy);
    // TODO: We may need to prepend the relative paths in `items` with the test case root
    let items_json = items.map_or_else(
        || "null".to_string(),
        |thi| {
            serde_json::to_string_pretty(thi)
                .expect("JSON deserialzation of type hierarchy items failed")
        },
    );
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition(*cursor_pos),
            LuaReplacement::Other {
                from: "items",
                to: items_json,
            },
        ],
        expected,
        |expected, actual: &Vec<TypeHierarchyItem>| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
                Err(PrepareTypeHierarchyMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                })?;
            }
            Ok(())
        },
    )
}

pub type PublishDiagnosticsComparator = fn(&Vec<Diagnostic>, &Vec<Diagnostic>, &TestCase) -> bool;

// NOTE: As far as I can tell, we can't directly accept a `PublishDiagnosticsParams` object,
// since diagnostics are requested via a `textDocument/publishDiagnostics` notification instead
// of a request. The `vim.lsp.buf_notify` method only returns a boolean to indicate success,
// so we can't access the actual data.

/// Tests the server's response to a [`textDocument/publishDiagnostics`] request.
///
/// Specifying a [`ServerStartType::Progress`] for a diagnostics test is overloaded to
/// determine which [`DiagnosticChanged`] autocmd to use. This can be useful if your
/// server sends multiple [`textDocument/publishDiagnostics`] notifications before
/// fully analyzing a source file.
///
/// An `Option` is not used for `expected` because the LSP spec does not allow for
/// nil parameters in the [`textDocument/publishDiagnostics`] notification
///
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/publishDiagnostics`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_publishDiagnostics
/// [`DiagnosticChanged`]: https://neovim.io/doc/user/diagnostic.html#DiagnosticChanged
pub fn test_publish_diagnostics(
    mut test_case: TestCase,
    cmp: Option<PublishDiagnosticsComparator>,
    expected: &Vec<Diagnostic>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::PublishDiagnostics);
    collect_results(
        &test_case,
        &mut Vec::new(),
        Some(expected),
        |expected: &Vec<Diagnostic>, actual: &Vec<Diagnostic>| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type ReferencesComparator = fn(&Vec<Location>, &Vec<Location>, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/references`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`ReferenceParams`]
/// - `include_declaration`: Passed to the client via the request's [`ReferenceParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON serialization of `include_declaration` fails
///
/// [`textDocument/references`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_references
pub fn test_references(
    mut test_case: TestCase,
    cursor_pos: &Position,
    include_declaration: bool,
    cmp: Option<ReferencesComparator>,
    expected: Option<&Vec<Location>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::References);
    let include_decl_json = serde_json::to_string_pretty(&include_declaration)
        .expect("JSON deserialzation of include declaration failed");
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition(*cursor_pos),
            LuaReplacement::ParamNested {
                name: "context",
                fields: vec![LuaReplacement::ParamDirect {
                    name: "includeDeclaration",
                    json: include_decl_json,
                }],
            },
        ],
        expected,
        |expected, actual: &Vec<Location>| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type RenameComparator = fn(&WorkspaceEdit, &WorkspaceEdit, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/rename`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`RenameParams`]
/// - `new_name`: Passed to the client via the request's [`RenameParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON serialization of `new_name` fails
///
/// [`textDocument/rename`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_rename
pub fn test_rename(
    mut test_case: TestCase,
    cursor_pos: &Position,
    new_name: &str,
    cmp: Option<RenameComparator>,
    expected: Option<&WorkspaceEdit>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::Rename);
    let new_name_json =
        serde_json::to_string_pretty(new_name).expect("JSON deserialzation of new name failed");
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition(*cursor_pos),
            LuaReplacement::ParamDirect {
                name: "newName",
                json: new_name_json,
            },
        ],
        expected,
        |expected, actual| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type SelectionRangeComparator =
    fn(&Vec<SelectionRange>, &Vec<SelectionRange>, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/selectionRange`] request
///
/// - `positions`: Passed to the client via the request's [`SelectionRangeParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON serialization of `positions` fails
///
/// [`textDocument/typeDefinition`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_selectionRange
pub fn test_selection_range(
    mut test_case: TestCase,
    positions: &Vec<Position>,
    cmp: Option<SelectionRangeComparator>,
    expected: Option<&Vec<SelectionRange>>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::SelectionRange);
    let positions_json =
        serde_json::to_string_pretty(positions).expect("JSON deserialzation of `positions` failed");
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamDirect {
                name: "positions",
                json: positions_json,
            },
        ],
        expected,
        |expected, actual: &Vec<SelectionRange>| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type SemanticTokensFullComparator =
    fn(&SemanticTokensResult, &SemanticTokensResult, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/semanticTokens/full`] request
///
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/semanticTokens/full`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#semanticTokens_fullRequest
pub fn test_semantic_tokens_full(
    mut test_case: TestCase,
    cmp: Option<SemanticTokensFullComparator>,
    expected: Option<&SemanticTokensResult>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::SemanticTokensFull);
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamTextDocument],
        expected,
        |expected, actual| {
            let eql = cmp.as_ref().map_or_else(
                || {
                    // HACK: Since the `SemanticTokensResult` is untagged, there's no way to differentiate
                    // between `SemanticTokensResult::Tokens` and `SemanticTokensResult::Partial`, as they
                    // are structurally identical when we have
                    // `SemanticTokensResult::Tokens(SemanticTokens { result_id: None, ...)`
                    // Treat this as a special case and say it's ok.
                    let mut eql_result = false;
                    match (expected, actual) {
                        (
                            SemanticTokensResult::Tokens(SemanticTokens {
                                result_id: None,
                                data: token_data,
                            }),
                            SemanticTokensResult::Partial(SemanticTokensPartialResult {
                                data: partial_data,
                            }),
                        )
                        | (
                            SemanticTokensResult::Partial(SemanticTokensPartialResult {
                                data: partial_data,
                            }),
                            SemanticTokensResult::Tokens(SemanticTokens {
                                result_id: None,
                                data: token_data,
                            }),
                        ) => {
                            if token_data == partial_data {
                                eql_result = true;
                            }
                        }
                        _ => eql_result = expected == actual,
                    }
                    eql_result
                },
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
                Err(SemanticTokensFullMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                })?;
            }
            Ok(())
        },
    )
}

pub type SemanticTokensFullDeltaComparator =
    fn(&SemanticTokensFullDeltaResult, &SemanticTokensFullDeltaResult, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/semanticTokens/full/delta`] request
///
/// First sends a [`textDocument/semanticTokens/full`] request to get the initial state,
/// and then issues a [`textDocument/semanticTokens/full/delta`] request if the first
/// response contained a `result_id`.
///
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/semanticTokens/full`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#semanticTokens_fullRequest
/// [`textDocument/semanticTokens/full/delta`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#semanticTokens_deltaRequest
pub fn test_semantic_tokens_full_delta(
    mut test_case: TestCase,
    cmp: Option<SemanticTokensFullDeltaComparator>,
    expected: Option<&SemanticTokensFullDeltaResult>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::SemanticTokensFullDelta);
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamTextDocument],
        expected,
        |expected, actual| {
            let eql = cmp.as_ref().map_or_else(
                || {
                    // HACK: Since the `SemanticTokensFullDeltaResult` is untagged, there's no way
                    // to differentiate between the `Tokens`, `TokensDelta`, and `PartialTokensDelta`
                    // variants if we get an empty vector in response. Just treat this as a special
                    // case and say it's ok.
                    // #[allow(unused_assignments)] // false positive?
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
                        ) if token_data.is_empty() && edit_data.is_empty() => true,
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
                        ) if token_data.is_empty() && partial_data.is_empty() => true,
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
                        ) if edit_data.is_empty() && partial_data.is_empty() => true,
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
                        ) if edit_data.is_empty() && token_data.is_empty() => true,
                        _ => expected == actual,
                    }
                },
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
                Err(Box::new(SemanticTokensFullDeltaMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                }))?;
            }
            Ok(())
        },
    )
}

pub type SemanticTokensRangeComparator =
    fn(&SemanticTokensRangeResult, &SemanticTokensRangeResult, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/semanticTokens/range`] request
///
/// - `range`: Passed to the client via the request's [`SemanticTokensRangeParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON serialization of `range` fails
///
/// [`textDocument/semanticTokens/range`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#semanticTokens_rangeRequest
pub fn test_semantic_tokens_range(
    mut test_case: TestCase,
    range: &Range,
    cmp: Option<SemanticTokensRangeComparator>,
    expected: Option<&SemanticTokensRangeResult>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::SemanticTokensRange);
    let range_json =
        serde_json::to_string_pretty(range).expect("JSON deserialzation of range failed");
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamDirect {
                name: "range",
                json: range_json,
            },
        ],
        expected,
        |expected, actual| {
            let eql = cmp.as_ref().map_or_else(
                || {
                    // HACK: Since the `SemanticTokensRangeResult` is untagged, there's no way
                    // to differentiate between `SemanticTokensRangeResult::Tokens` and
                    // `SemanticTokensRangeResult::Partial`, as they are structurally identical
                    // when we have `SemanticTokensResult::Tokens(SemanticTokens { result_id: None, ...)`
                    // Treat this as a special case and say it's ok.
                    let mut eql_result = false;
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
                                eql_result = true;
                            }
                        }
                        _ => eql_result = expected == actual,
                    }
                    eql_result
                },
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type SeignatureHelpComparator = fn(&SignatureHelp, &SignatureHelp, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/signatureHelp`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`SignatureHelpParams`]
/// - `context`: Passed to the client via the request's [`SignatureHelpParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON serialization of `context` fails
///
/// [`textDocument/signatureHelp`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_signatureHelp
pub fn test_signature_help(
    mut test_case: TestCase,
    cursor_pos: &Position,
    context: Option<&SignatureHelpContext>,
    cmp: Option<SeignatureHelpComparator>,
    expected: Option<&SignatureHelp>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::SignatureHelp);
    let context_json = context.map_or_else(
        || "null".to_string(),
        |ctx| {
            serde_json::to_string_pretty(ctx)
                .expect("JSON deserialzation of signature help context failed")
        },
    );
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition(*cursor_pos),
            LuaReplacement::ParamDirect {
                name: "context",
                json: context_json,
            },
        ],
        expected,
        |expected, actual| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type TypeDefinitionComparator =
    fn(&GotoTypeDefinitionResponse, &GotoTypeDefinitionResponse, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/typeDefinition`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`GotoTypeDefinitionParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/typeDefinition`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_typeDefinition
pub fn test_type_definition(
    mut test_case: TestCase,
    cursor_pos: &Position,
    cmp: Option<TypeDefinitionComparator>,
    expected: Option<&GotoTypeDefinitionResponse>,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::TypeDefinition);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition(*cursor_pos),
        ],
        expected,
        |expected, actual| {
            let eql = cmp.as_ref().map_or_else(
                || {
                    // HACK: Since the `GotoTypeDefinitionResponse` is untagged, there's no way
                    // to differentiate between the `Array` and `Link` if we get an empty vector
                    // in response. Just treat this as a special case and say it's ok.
                    let mut eql_result = false;
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
                                eql_result = true;
                            }
                        }
                        _ => eql_result = expected == actual,
                    }
                    eql_result
                },
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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

pub type WorkspaceDiagnosticComparator =
    fn(&WorkspaceDiagnosticReport, &WorkspaceDiagnosticReport, &TestCase) -> bool;

/// Tests the server's response to a [`workspace/diagnostic`] request
///
/// - `identifier`: Passed to the client via the request's [`WorkspaceDiagnosticParams`]
/// - `previous_result_id`: Passed to the client via the request's [`WorkspaceDiagnosticParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON serialization of `identifier` or `previous_result_id` fails
///
/// [`workspace/diagnostic`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_diagnostic
pub fn test_workspace_diagnostic(
    mut test_case: TestCase,
    identifier: Option<String>,
    previous_result_ids: &Vec<PreviousResultId>,
    cmp: Option<WorkspaceDiagnosticComparator>,
    expected: &WorkspaceDiagnosticReport,
) -> TestResult<()> {
    test_case.test_type = Some(TestType::WorkspaceDiagnostic);
    let identifier_json = identifier.map_or_else(
        || "null".to_string(), // NOTE: `vim.json.decode()` fails with an empty string
        |id| serde_json::to_string_pretty(&id).expect("JSON deserialzation of identifier failed"),
    );
    let previous_result_ids_json = serde_json::to_string_pretty(previous_result_ids)
        .expect("JSON deserialzation of previous result id failed");
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamDirect {
                name: "identifier",
                json: identifier_json,
            },
            LuaReplacement::ParamDirect {
                name: "previousResultIds",
                json: previous_result_ids_json,
            },
        ],
        Some(expected),
        |expected: &WorkspaceDiagnosticReport, actual: &WorkspaceDiagnosticReport| {
            let eql = cmp.as_ref().map_or_else(
                || expected == actual,
                |cmp_fn| cmp_fn(expected, actual, &test_case),
            );
            if !eql {
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
