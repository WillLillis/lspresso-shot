mod init_dot_lua;
pub mod types;

use init_dot_lua::LuaReplacement;
use lsp_types::{
    CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall, CodeAction,
    CodeActionContext, CodeActionResponse, CodeLens, Color, ColorInformation, ColorPresentation,
    CompletionItem, CompletionResponse, CreateFilesParams, Diagnostic, DocumentDiagnosticReport,
    DocumentHighlight, DocumentLink, DocumentSymbolResponse, FoldingRange, FormattingOptions,
    GotoDefinitionResponse, Hover, InlayHint, LinkedEditingRanges, Location, Moniker, Position,
    PrepareRenameResponse, PreviousResultId, Range, SelectionRange, SemanticTokensFullDeltaResult,
    SemanticTokensRangeResult, SemanticTokensResult, SignatureHelp, SignatureHelpContext, TextEdit,
    TypeHierarchyItem, WorkspaceDiagnosticReport, WorkspaceEdit, WorkspaceSymbol,
    WorkspaceSymbolResponse,
    request::{GotoDeclarationResponse, GotoImplementationResponse, GotoTypeDefinitionResponse},
};

// These imports are included solely for the sake of highlighting in doc comments
#[allow(unused_imports)]
use lsp_types::{
    CallHierarchyIncomingCallsParams, CallHierarchyOutgoingCallsParams, CallHierarchyPrepareParams,
    CodeActionParams, CompletionParams, DocumentDiagnosticParams, DocumentHighlightParams,
    DocumentOnTypeFormattingParams, DocumentRangeFormattingParams, GotoDefinitionParams,
    HoverParams, InlayHintParams, MonikerParams, ReferenceParams, RenameParams,
    SelectionRangeParams, SemanticTokensRangeParams, SignatureHelpParams,
    TextDocumentPositionParams, TypeHierarchyPrepareParams, WorkspaceDiagnosticParams,
    WorkspaceSymbolParams,
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
    ApproximateEq, CleanResponse, ResponseMismatchError, TestCase, TestError, TestExecutionError,
    TestExecutionResult, TestResult, TestType, TimeoutError, formatting::FormattingResult,
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

/// This is the main workhorse for the testing framework. This function
/// validates the test case and runs the actual test case. If there are results,
/// it deserializes them and compares them to the expected results. The type parameter
/// `T` is the type of the expected results.
///
/// Note that even if a given request doesn't support an `Option` response, `expected`
/// is always an `Option` here. For these cases, the expected result should be passed
/// as `Some(expected)` unconditionally in the caller
#[allow(clippy::needless_pass_by_value)]
fn collect_results<T>(
    test_case: &TestCase,
    replacements: &mut Vec<LuaReplacement>,
    expected: Option<&T>,
    cmp: Option<impl Fn(&T, &T, &TestCase) -> bool>,
) -> TestResult<(), T>
where
    T: Clone + serde::de::DeserializeOwned + std::fmt::Debug + CleanResponse + ApproximateEq,
{
    let get_results = |path: &Path| -> TestExecutionResult<T> {
        let raw_results = String::from_utf8(
            fs::read(path)
                .map_err(|e| TestExecutionError::IO(test_case.test_id.clone(), e.to_string()))?,
        )
        .map_err(|e| TestExecutionError::Utf8(test_case.test_id.clone(), e.to_string()))?;
        let raw_resp: T = serde_json::from_str(&raw_results).map_err(|e| {
            TestExecutionError::Serialization(test_case.test_id.clone(), e.to_string())
        })?;
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
        .map_err(|e| TestExecutionError::IO(test_case.test_id.clone(), e.to_string()))?;
    let results_file_path = test_case
        .get_results_file_path()
        .map_err(|e| TestExecutionError::IO(test_case.test_id.clone(), e.to_string()))?;

    match (
        expected,
        empty_result_path.exists(),
        results_file_path.exists(),
    ) {
        // Expected and got empty results
        (None, true, false) => Ok(()),
        // Expected empty results, got some
        (None, false, true) => {
            // NOTE: We may need to handle deserialization errors here
            let results: T = get_results(&results_file_path)?;
            Err(TestError::ResponseMismatch(ResponseMismatchError {
                test_id: test_case.test_id.clone(),
                expected: None,
                actual: Some(results),
            }))?
        }
        // Invariant: `results.json` and `empty` should never both exist
        (_, true, true) => unreachable!(),
        // No results
        (_, false, false) => Err(TestExecutionError::NoResults(test_case.test_id.clone()))?,
        // Expected some results, got none
        (Some(_), true, false) => Err(TestError::ResponseMismatch(ResponseMismatchError {
            test_id: test_case.test_id.clone(),
            expected: expected.cloned(),
            actual: None,
        }))?,
        // Expected and got some results
        (Some(exp), false, true) => {
            let actual: T = get_results(&results_file_path)?;
            if !cmp.as_ref().map_or_else(
                || T::approx_eq(exp, &actual),
                |cmp_fn| cmp_fn(exp, &actual, test_case),
            ) {
                Err(ResponseMismatchError {
                    test_id: test_case.test_id.clone(),
                    expected: Some((*exp).clone()),
                    actual: Some(actual),
                })?;
            }
            Ok(())
        }
    }
}

/// Invokes neovim to run the test with `test_case`'s associated `init.lua` file,
/// opening `source_path`
fn run_test(test_case: &TestCase, source_path: &Path) -> TestExecutionResult<()> {
    let init_dot_lua_path = test_case
        .get_init_lua_file_path()
        .map_err(|e| TestExecutionError::IO(test_case.test_id.clone(), e.to_string()))?;

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
        .stderr(Stdio::null()) // debugging, can print some logs from the server
        .spawn()
        .map_err(|e| TestExecutionError::Neovim(test_case.test_id.clone(), e.to_string()))?;

    // In theory, the timeout set in `init.lua` should be sufficient to prevent
    // the neovim process from hanging. However, if `init.lua` is malformed (an
    // error for this library), then the timer will never start. Add the same
    // timeout (with an arbitrary cushion) here as a fallback
    let timeout_cushion = std::time::Duration::from_millis(500);
    while start.elapsed() < test_case.timeout + timeout_cushion {
        match child.try_wait() {
            Ok(Some(_)) => {
                if test_case.did_exceed_timeout() {
                    Err(TestExecutionError::TimeoutExceeded(TimeoutError {
                        test_id: test_case.test_id.clone(),
                        timeout: test_case.timeout,
                    }))?;
                }
                return Ok(());
            }
            Ok(None) => {} // still running
            Err(e) => Err(TestExecutionError::Neovim(
                test_case.test_id.clone(),
                e.to_string(),
            ))?,
        }
    }

    // A test can also timeout due to neovim encountering an error (i.e. a malformed
    // `init.lua` file). If we have an error recorded, it's better to report that
    // than the timeout
    let error_path = test_case
        .get_error_file_path()
        .map_err(|e| TestExecutionError::IO(test_case.test_id.clone(), e.to_string()))?;
    if error_path.exists() {
        let error = fs::read_to_string(&error_path)
            .map_err(|e| TestExecutionError::IO(test_case.test_id.clone(), e.to_string()))?;
        Err(TestExecutionError::Neovim(test_case.test_id.clone(), error))?;
    }

    Err(TestExecutionError::TimeoutExceeded(TimeoutError {
        test_id: test_case.test_id.clone(),
        timeout: test_case.timeout,
    }))?
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
/// Panics if JSON serialization of `context` fails
///
/// [`textDocument/codeAction`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_codeAction
pub fn test_code_action(
    mut test_case: TestCase,
    range: Range,
    context: &CodeActionContext,
    cmp: Option<CodeActionComparator>,
    expected: Option<&CodeActionResponse>,
) -> TestResult<(), CodeActionResponse> {
    test_case.test_type = Some(TestType::CodeAction);
    let context_json =
        serde_json::to_string_pretty(context).expect("JSON serialization of `context` failed");
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamRange(range),
            LuaReplacement::ParamDirect {
                name: "context",
                json: context_json,
            },
        ],
        expected,
        cmp,
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
#[allow(clippy::result_large_err)]
pub fn test_code_action_resolve(
    mut test_case: TestCase,
    params: &CodeAction,
    cmp: Option<CodeActionResolveComparator>,
    expected: &CodeAction,
) -> TestResult<(), CodeAction> {
    test_case.test_type = Some(TestType::CodeActionResolve);
    let code_action_json =
        serde_json::to_string_pretty(params).expect("JSON serialization of `params` failed");
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
        cmp,
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
) -> TestResult<(), Vec<CodeLens>> {
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
        cmp,
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
#[allow(clippy::result_large_err)]
pub fn test_code_lens_resolve(
    mut test_case: TestCase,
    commands: Option<&Vec<String>>,
    code_lens: &CodeLens,
    cmp: Option<CodeLensResolveComparator>,
    expected: Option<&CodeLens>,
) -> TestResult<(), CodeLens> {
    test_case.test_type = Some(TestType::CodeLensResolve);
    let command_str = commands.map_or_else(String::new, |cmds| {
        cmds.iter()
            .fold(String::new(), |accum, cmd| accum + &format!("\"{cmd}\",\n"))
    });
    let code_lens_json =
        serde_json::to_string_pretty(code_lens).expect("JSON serialization of `code_lens` failed");
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
        cmp,
    )
}

pub type ColorPresentationComparator =
    fn(&Vec<ColorPresentation>, &Vec<ColorPresentation>, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/colorPresentation`] request
///
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
/// Panics if JSON serialization of `color` fails
///
/// [`textDocument/colorPresentation`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_colorPresentation
pub fn test_color_presentation(
    mut test_case: TestCase,
    color: Color,
    range: Range,
    cmp: Option<ColorPresentationComparator>,
    expected: &Vec<ColorPresentation>,
) -> TestResult<(), Vec<ColorPresentation>> {
    test_case.test_type = Some(TestType::ColorPresentation);
    let color_json =
        serde_json::to_string_pretty(&color).expect("JSON serialization of `color` failed");
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamDirect {
                name: "color",
                json: color_json,
            },
            LuaReplacement::ParamRange(range),
        ],
        Some(expected),
        cmp,
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
    cursor_pos: Position,
    cmp: Option<CompletionComparator>,
    expected: Option<&CompletionResponse>,
) -> TestResult<(), CompletionResponse> {
    test_case.test_type = Some(TestType::Completion);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition {
                pos: cursor_pos,
                name: None,
            },
        ],
        expected,
        cmp,
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
#[allow(clippy::result_large_err)]
pub fn test_completion_resolve(
    mut test_case: TestCase,
    completion_item: &CompletionItem,
    cmp: Option<CompletionResolveComparator>,
    expected: Option<&CompletionItem>,
) -> TestResult<(), CompletionItem> {
    test_case.test_type = Some(TestType::CompletionResolve);
    let completion_item_json = serde_json::to_string_pretty(completion_item)
        .expect("JSON serialization of `completion_item` failed");
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
        cmp,
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
/// # Warnings
///
/// Different values of `GotoDeclarationResponse` can be serialized to the same JSON
/// representation. Because the LSP specification is defined over JSON RPC, this means
/// that the value received by the LSP client may not match the value sent by your
/// server. This ambiguity is handled in the this function's default comparison logic,
/// but can be overriden by providing your own `cmp` function.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/declaration`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_declaration
#[allow(clippy::result_large_err)]
pub fn test_declaration(
    mut test_case: TestCase,
    cursor_pos: Position,
    cmp: Option<DeclarationComparator>,
    expected: Option<&GotoDeclarationResponse>,
) -> TestResult<(), GotoDeclarationResponse> {
    test_case.test_type = Some(TestType::Declaration);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition {
                pos: cursor_pos,
                name: None,
            },
        ],
        expected,
        cmp,
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
/// # Warnings
///
/// Different values of `GotoDefinitionResponse` can be serialized to the same JSON
/// representation. Because the LSP specification is defined over JSON RPC, this means
/// that the value received by the LSP client may not match the value sent by your
/// server. This ambiguity is handled in the this function's default comparison logic,
/// but can be overriden by providing your own `cmp` function.
///
/// # Errors
///
/// Returns [`TestError`] if the expected results don't match, or if some other failure occurs
///
/// [`textDocument/definition`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_definition
#[allow(clippy::result_large_err)]
pub fn test_definition(
    mut test_case: TestCase,
    cursor_pos: Position,
    cmp: Option<DefinitionComparator>,
    expected: Option<&GotoDefinitionResponse>,
) -> TestResult<(), GotoDefinitionResponse> {
    test_case.test_type = Some(TestType::Definition);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition {
                pos: cursor_pos,
                name: None,
            },
        ],
        expected,
        cmp,
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
#[allow(clippy::result_large_err)]
pub fn test_diagnostic(
    mut test_case: TestCase,
    identifier: Option<&str>,
    previous_result_id: Option<&str>,
    cmp: Option<DiagnosticComparator>,
    expected: &DocumentDiagnosticReport,
) -> TestResult<(), DocumentDiagnosticReport> {
    test_case.test_type = Some(TestType::Diagnostic);
    let identifier_json = identifier.map_or_else(
        || "null".to_string(), // NOTE: `vim.json.decode()` fails with an empty string
        |id| serde_json::to_string_pretty(id).expect("JSON serialization of `identifier` failed"),
    );
    let previous_result_id_json = previous_result_id.map_or_else(
        || "null".to_string(), // NOTE: `vim.json.decode()` fails with an empty string
        |id| {
            serde_json::to_string_pretty(id)
                .expect("JSON serialization of previous `previous_result_id` failed")
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
        cmp,
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
) -> TestResult<(), Vec<ColorInformation>> {
    test_case.test_type = Some(TestType::DocumentColor);
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamTextDocument],
        Some(expected),
        cmp,
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
    cursor_pos: Position,
    cmp: Option<DocumentHighlightComparator>,
    expected: Option<&Vec<DocumentHighlight>>,
) -> TestResult<(), Vec<DocumentHighlight>> {
    test_case.test_type = Some(TestType::DocumentHighlight);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition {
                pos: cursor_pos,
                name: None,
            },
        ],
        expected,
        cmp,
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
) -> TestResult<(), Vec<DocumentLink>> {
    test_case.test_type = Some(TestType::DocumentLink);
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamTextDocument],
        expected,
        cmp,
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
#[allow(clippy::result_large_err)]
pub fn test_document_link_resolve(
    mut test_case: TestCase,
    params: &DocumentLink,
    cmp: Option<DocumentLinkResolveComparator>,
    expected: Option<&DocumentLink>,
) -> TestResult<(), DocumentLink> {
    let document_link_json =
        serde_json::to_string_pretty(params).expect("JSON serialization of `params` failed");
    test_case.test_type = Some(TestType::DocumentLinkResolve);
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamDestructure {
            name: "link",
            fields: vec!["range", "target", "tooltip", "data"],
            json: document_link_json,
        }],
        expected,
        cmp,
    )
}

pub type DocumentSymbolComparator =
    fn(&DocumentSymbolResponse, &DocumentSymbolResponse, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/documentSymbol`] request
///
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// Different values of `DocumentSymbolResponse` can be serialized to the same JSON
/// representation. Because the LSP specification is defined over JSON RPC, this means
/// that the value received by the LSP client may not match the value sent by your
/// server. This ambiguity is handled in the this function's default comparison logic,
/// but can be overriden by providing your own `cmp` function.
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
) -> TestResult<(), DocumentSymbolResponse> {
    test_case.test_type = Some(TestType::DocumentSymbol);
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamTextDocument],
        expected,
        cmp,
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
) -> TestResult<(), Vec<FoldingRange>> {
    test_case.test_type = Some(TestType::FoldingRange);
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamTextDocument],
        expected,
        cmp,
    )
}

fn default_format_opts() -> FormattingOptions {
    FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        properties: HashMap::new(),
        trim_trailing_whitespace: Some(true),
        insert_final_newline: Some(true),
        trim_final_newlines: Some(true),
    }
}

pub type FormattingComparator = fn(&FormattingResult, &FormattingResult, &TestCase) -> bool;

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
///   between the expected and actual results.
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
    options: Option<&FormattingOptions>,
    cmp: Option<FormattingComparator>,
    expected: Option<&FormattingResult>,
) -> TestResult<(), FormattingResult> {
    use types::formatting::to_parent_err_type;
    test_case.test_type = Some(TestType::Formatting);
    let options_json = options
        .map_or_else(
            || serde_json::to_string_pretty(&default_format_opts()),
            serde_json::to_string_pretty,
        )
        .expect("JSON serialization of `options` failed");
    // map the child error types of `test_formatting_*` to `TestError<FormattingResult>`
    match expected {
        Some(FormattingResult::Response(edits)) => to_parent_err_type(test_formatting_resp(
            &test_case,
            options_json,
            cmp,
            Some(edits),
        )),
        Some(FormattingResult::EndState(state)) => to_parent_err_type(test_formatting_state(
            &test_case,
            options_json,
            cmp,
            state.to_string(),
        )),
        None => to_parent_err_type(test_formatting_resp(&test_case, options_json, cmp, None)),
    }
}

/// Performs the test for [`test_formatting`] when the expected result is `Some(Vec<TextEdit>)` or
/// `None`.
fn test_formatting_resp(
    test_case: &TestCase,
    options_json: String,
    cmp: Option<FormattingComparator>,
    expected: Option<&Vec<TextEdit>>,
) -> TestResult<(), Vec<TextEdit>> {
    let outer_cmp =
        |expected: &Vec<TextEdit>, actual: &Vec<TextEdit>, test_case: &TestCase| -> bool {
            let result_expected = FormattingResult::Response(expected.clone());
            let result_actual = FormattingResult::Response(actual.clone());
            cmp.as_ref().map_or_else(
                || result_expected == result_actual,
                |cmp_fn| cmp_fn(&result_expected, &result_actual, test_case),
            )
        };
    collect_results(
        test_case,
        &mut vec![
            LuaReplacement::Other {
                from: "INVOKE_FORMAT",
                to: "false".to_string(),
            },
            LuaReplacement::ParamDirect {
                name: "options",
                json: options_json,
            },
        ],
        expected,
        Some(&outer_cmp),
    )
}

/// Performs the test for [`test_formatting`] when the expected result is `String`.
#[allow(clippy::needless_pass_by_value)]
fn test_formatting_state(
    test_case: &TestCase,
    options_json: String,
    cmp: Option<FormattingComparator>,
    expected: String,
) -> TestResult<(), String> {
    let outer_cmp = |expected: &String, actual: &String, test_case: &TestCase| -> bool {
        let result_expected = FormattingResult::EndState(expected.to_string());
        let result_actual = FormattingResult::EndState(actual.to_string());
        cmp.as_ref().map_or_else(
            || result_expected == result_actual,
            |cmp_fn| cmp_fn(&result_expected, &result_actual, test_case),
        )
    };
    collect_results(
        test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::Other {
                from: "INVOKE_FORMAT",
                to: "true".to_string(),
            },
            LuaReplacement::ParamDirect {
                name: "options",
                json: options_json,
            },
        ],
        Some(&expected),
        Some(&outer_cmp),
    )
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
#[allow(clippy::result_large_err)]
pub fn test_hover(
    mut test_case: TestCase,
    cursor_pos: Position,
    cmp: Option<HoverComparator>,
    expected: Option<&Hover>,
) -> TestResult<(), Hover> {
    test_case.test_type = Some(TestType::Hover);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition {
                pos: cursor_pos,
                name: None,
            },
        ],
        expected,
        cmp,
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
/// # Warnings
///
/// Different values of `GotoImplementationResponse` can be serialized to the same JSON
/// representation. Because the LSP specification is defined over JSON RPC, this means
/// that the value received by the LSP client may not match the value sent by your
/// server. This ambiguity is handled in the this function's default comparison logic,
/// but can be overriden by providing your own `cmp` function.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/implementation`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_implementation
#[allow(clippy::result_large_err)]
pub fn test_implementation(
    mut test_case: TestCase,
    cursor_pos: Position,
    cmp: Option<ImplementationComparator>,
    expected: Option<&GotoImplementationResponse>,
) -> TestResult<(), GotoImplementationResponse> {
    test_case.test_type = Some(TestType::Implementation);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition {
                pos: cursor_pos,
                name: None,
            },
        ],
        expected,
        cmp,
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
) -> TestResult<(), Vec<CallHierarchyIncomingCall>> {
    test_case.test_type = Some(TestType::IncomingCalls);
    let call_item_json =
        serde_json::to_string_pretty(call_item).expect("JSON serialization of `call_item` failed");
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamDirect {
            name: "item",
            json: call_item_json,
        }],
        expected,
        cmp,
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
/// [`textDocument/inlayHint`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_inlayHint
pub fn test_inlay_hint(
    mut test_case: TestCase,
    range: Range,
    cmp: Option<InlayHintComparator>,
    expected: Option<&Vec<InlayHint>>,
) -> TestResult<(), Vec<InlayHint>> {
    test_case.test_type = Some(TestType::InlayHint);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamRange(range),
        ],
        expected,
        cmp,
    )
}

pub type LinkedEditingRangeComparator =
    fn(&LinkedEditingRanges, &LinkedEditingRanges, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/linkedEditingRange`] request
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
/// [`textDocument/moniker`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_linkedEditingRange
pub fn test_linked_editing_range(
    mut test_case: TestCase,
    cursor_pos: Position,
    cmp: Option<LinkedEditingRangeComparator>,
    expected: Option<&LinkedEditingRanges>,
) -> TestResult<(), LinkedEditingRanges> {
    test_case.test_type = Some(TestType::LinkedEditingRange);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition {
                pos: cursor_pos,
                name: None,
            },
        ],
        expected,
        cmp,
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
    cursor_pos: Position,
    cmp: Option<MonikerComparator>,
    expected: Option<&Vec<Moniker>>,
) -> TestResult<(), Vec<Moniker>> {
    test_case.test_type = Some(TestType::Moniker);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition {
                pos: cursor_pos,
                name: None,
            },
        ],
        expected,
        cmp,
    )
}

pub type OnTypeFormattingComparator = fn(&Vec<TextEdit>, &Vec<TextEdit>, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/onTypeFormatting`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`DocumentOnTypeFormattingParams`]
/// - `character`: Passed to the client via the request's [`DocumentOnTypeFormattingParams`]
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
/// Panics if JSON serialization of `character` or `options` fails
///
/// [`callHierarchy/outgoingCalls`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_onTypeFormatting
pub fn test_on_type_formatting(
    mut test_case: TestCase,
    cursor_pos: Position,
    character: &str,
    options: Option<&FormattingOptions>,
    cmp: Option<OnTypeFormattingComparator>,
    expected: Option<&Vec<TextEdit>>,
) -> TestResult<(), Vec<TextEdit>> {
    test_case.test_type = Some(TestType::OnTypeFormatting);
    let character_json =
        serde_json::to_string_pretty(character).expect("JSON serialization of `character` failed");
    let options_json = options
        .map_or_else(
            || serde_json::to_string_pretty(&default_format_opts()),
            serde_json::to_string_pretty,
        )
        .expect("JSON serialization of `options` failed");
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition {
                pos: cursor_pos,
                name: None,
            },
            LuaReplacement::ParamDirect {
                name: "ch",
                json: character_json,
            },
            LuaReplacement::ParamDirect {
                name: "options",
                json: options_json,
            },
        ],
        expected,
        cmp,
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
) -> TestResult<(), Vec<CallHierarchyOutgoingCall>> {
    test_case.test_type = Some(TestType::OutgoingCalls);
    let call_item_json =
        serde_json::to_string_pretty(call_item).expect("JSON serialization of `call_item` failed");
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamDirect {
            name: "item",
            json: call_item_json,
        }],
        expected,
        cmp,
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
    cursor_pos: Position,
    cmp: Option<PrepareCallHierarchyComparator>,
    expected: Option<&Vec<CallHierarchyItem>>,
) -> TestResult<(), Vec<CallHierarchyItem>> {
    test_case.test_type = Some(TestType::PrepareCallHierarchy);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition {
                pos: cursor_pos,
                name: None,
            },
        ],
        expected,
        cmp,
    )
}

pub type PrepareRenameComparator =
    fn(&PrepareRenameResponse, &PrepareRenameResponse, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/prepareRename`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`TextDocumentPositionParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/prepareCallHierarchy`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_prepareRename
pub fn test_prepare_rename(
    mut test_case: TestCase,
    cursor_pos: Position,
    cmp: Option<PrepareRenameComparator>,
    expected: Option<&PrepareRenameResponse>,
) -> TestResult<(), PrepareRenameResponse> {
    test_case.test_type = Some(TestType::PrepareRename);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition {
                pos: cursor_pos,
                name: None,
            },
        ],
        expected,
        cmp,
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
    cursor_pos: Position,
    items: Option<&Vec<TypeHierarchyItem>>,
    cmp: Option<PrepareTypeHierarchyComparator>,
    expected: Option<&Vec<TypeHierarchyItem>>,
) -> TestResult<(), Vec<TypeHierarchyItem>> {
    test_case.test_type = Some(TestType::PrepareTypeHierarchy);
    // TODO: We may need to prepend the relative paths in `items` with the test case root
    let items_json = items.map_or_else(
        || "null".to_string(),
        |thi| serde_json::to_string_pretty(thi).expect("JSON serialization of type `items` failed"),
    );
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition {
                pos: cursor_pos,
                name: None,
            },
            LuaReplacement::Other {
                from: "items",
                to: items_json,
            },
        ],
        expected,
        cmp,
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
) -> TestResult<(), Vec<Diagnostic>> {
    test_case.test_type = Some(TestType::PublishDiagnostics);
    collect_results(&test_case, &mut Vec::new(), Some(expected), cmp)
}

pub type RangeFormattingComparator = fn(&Vec<TextEdit>, &Vec<TextEdit>, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/rangeFormatting`] request
///
/// - `range`: Passed to the client via the request's [`DocumentRangeFormattingParams`]
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
/// Panics if JSON serialization of `options` fails
///
/// [`textDocument/rangeFormatting`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_rangeFormatting
pub fn test_range_formatting(
    mut test_case: TestCase,
    range: Range,
    options: Option<&FormattingOptions>,
    cmp: Option<RangeFormattingComparator>,
    expected: Option<&Vec<TextEdit>>,
) -> TestResult<(), Vec<TextEdit>> {
    test_case.test_type = Some(TestType::RangeFormatting);
    let options_json = options
        .map_or_else(
            || serde_json::to_string_pretty(&default_format_opts()),
            serde_json::to_string_pretty,
        )
        .expect("JSON serialization of `options` failed");
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamRange(range),
            LuaReplacement::ParamDirect {
                name: "options",
                json: options_json,
            },
        ],
        expected,
        cmp,
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
    cursor_pos: Position,
    include_declaration: bool,
    cmp: Option<ReferencesComparator>,
    expected: Option<&Vec<Location>>,
) -> TestResult<(), Vec<Location>> {
    test_case.test_type = Some(TestType::References);
    let include_decl_json = serde_json::to_string_pretty(&include_declaration)
        .expect("JSON serialization of `include_declaration` failed");
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition {
                pos: cursor_pos,
                name: None,
            },
            LuaReplacement::ParamNested {
                name: "context",
                fields: vec![LuaReplacement::ParamDirect {
                    name: "includeDeclaration",
                    json: include_decl_json,
                }],
            },
        ],
        expected,
        cmp,
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
#[allow(clippy::result_large_err)]
pub fn test_rename(
    mut test_case: TestCase,
    cursor_pos: Position,
    new_name: &str,
    cmp: Option<RenameComparator>,
    expected: Option<&WorkspaceEdit>,
) -> TestResult<(), WorkspaceEdit> {
    test_case.test_type = Some(TestType::Rename);
    let new_name_json =
        serde_json::to_string_pretty(new_name).expect("JSON serialization of `new_name` failed");
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition {
                pos: cursor_pos,
                name: None,
            },
            LuaReplacement::ParamDirect {
                name: "newName",
                json: new_name_json,
            },
        ],
        expected,
        cmp,
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
) -> TestResult<(), Vec<SelectionRange>> {
    test_case.test_type = Some(TestType::SelectionRange);
    let positions_json =
        serde_json::to_string_pretty(positions).expect("JSON serialization of `positions` failed");
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
        cmp,
    )
}

pub type SemanticTokensFullComparator =
    fn(&SemanticTokensResult, &SemanticTokensResult, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/semanticTokens/full`] request
///
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Warnings
///
/// Different values of `SemanticTokensResult` can be serialized to the same JSON
/// representation. Because the LSP specification is defined over JSON RPC, this means
/// that the value received by the LSP client may not match the value sent by your
/// server. This ambiguity is handled in the this function's default comparison logic,
/// but can be overriden by providing your own `cmp` function.
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
) -> TestResult<(), SemanticTokensResult> {
    test_case.test_type = Some(TestType::SemanticTokensFull);
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamTextDocument],
        expected,
        cmp,
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
/// # Warnings
///
/// Different values of `SemanticTokensFullDeltaResult` can be serialized to the same JSON
/// representation. Because the LSP specification is defined over JSON RPC, this means
/// that the value received by the LSP client may not match the value sent by your
/// server. This ambiguity is handled in the this function's default comparison logic,
/// but can be overriden by providing your own `cmp` function.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/semanticTokens/full`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#semanticTokens_fullRequest
/// [`textDocument/semanticTokens/full/delta`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#semanticTokens_deltaRequest
#[allow(clippy::result_large_err)]
pub fn test_semantic_tokens_full_delta(
    mut test_case: TestCase,
    cmp: Option<SemanticTokensFullDeltaComparator>,
    expected: Option<&SemanticTokensFullDeltaResult>,
) -> TestResult<(), SemanticTokensFullDeltaResult> {
    test_case.test_type = Some(TestType::SemanticTokensFullDelta);
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamTextDocument],
        expected,
        cmp,
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
/// # Warnings
///
/// Different values of `SemanticTokensRangeResult` can be serialized to the same JSON
/// representation. Because the LSP specification is defined over JSON RPC, this means
/// that the value received by the LSP client may not match the value sent by your
/// server. This ambiguity is handled in the this function's default comparison logic,
/// but can be overriden by providing your own `cmp` function.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/semanticTokens/range`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#semanticTokens_rangeRequest
pub fn test_semantic_tokens_range(
    mut test_case: TestCase,
    range: Range,
    cmp: Option<SemanticTokensRangeComparator>,
    expected: Option<&SemanticTokensRangeResult>,
) -> TestResult<(), SemanticTokensRangeResult> {
    test_case.test_type = Some(TestType::SemanticTokensRange);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamRange(range),
        ],
        expected,
        cmp,
    )
}

pub type SignatureHelpComparator = fn(&SignatureHelp, &SignatureHelp, &TestCase) -> bool;

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
    cursor_pos: Position,
    context: Option<&SignatureHelpContext>,
    cmp: Option<SignatureHelpComparator>,
    expected: Option<&SignatureHelp>,
) -> TestResult<(), SignatureHelp> {
    test_case.test_type = Some(TestType::SignatureHelp);
    let context_json = context.map_or_else(
        || "null".to_string(),
        |ctx| serde_json::to_string_pretty(ctx).expect("JSON serialization of `context` failed"),
    );
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition {
                pos: cursor_pos,
                name: None,
            },
            LuaReplacement::ParamDirect {
                name: "context",
                json: context_json,
            },
        ],
        expected,
        cmp,
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
/// # Warnings
///
/// Different values of `GotoTypeDefinitionResponse` can be serialized to the same JSON
/// representation. Because the LSP specification is defined over JSON RPC, this means
/// that the value received by the LSP client may not match the value sent by your
/// server. This ambiguity is handled in the this function's default comparison logic,
/// but can be overriden by providing your own `cmp` function.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/typeDefinition`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_typeDefinition
#[allow(clippy::result_large_err)]
pub fn test_type_definition(
    mut test_case: TestCase,
    cursor_pos: Position,
    cmp: Option<TypeDefinitionComparator>,
    expected: Option<&GotoTypeDefinitionResponse>,
) -> TestResult<(), GotoTypeDefinitionResponse> {
    test_case.test_type = Some(TestType::TypeDefinition);
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamPosition {
                pos: cursor_pos,
                name: None,
            },
        ],
        expected,
        cmp,
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
    identifier: Option<&str>,
    previous_result_ids: &Vec<PreviousResultId>,
    cmp: Option<WorkspaceDiagnosticComparator>,
    expected: &WorkspaceDiagnosticReport,
) -> TestResult<(), WorkspaceDiagnosticReport> {
    test_case.test_type = Some(TestType::WorkspaceDiagnostic);
    let identifier_json = identifier.map_or_else(
        || "null".to_string(), // NOTE: `vim.json.decode()` fails with an empty string
        |id| serde_json::to_string_pretty(id).expect("JSON serialization of `identifier` failed"),
    );
    let previous_result_ids_json = serde_json::to_string_pretty(previous_result_ids)
        .expect("JSON serialization of `previous_result_id` failed");
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
        cmp,
    )
}

pub type WorkspaceSymbolComparator =
    fn(&WorkspaceSymbolResponse, &WorkspaceSymbolResponse, &TestCase) -> bool;

/// Tests the server's response to a [`workspace/symbol`] request
///
/// - `query`: Passed to the client via the request's [`WorkspaceSymbolParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Warnings
///
/// Different values of `GotoTypeDefinitionResponse` can be serialized to the same JSON
/// representation. Because the LSP specification is defined over JSON RPC, this means
/// that the value received by the LSP client may not match the value sent by your
/// server. This ambiguity is handled in the this function's default comparison logic,
/// but can be overriden by providing your own `cmp` function.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON serialization of `query` fails
///
/// [`workspace/symbol`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_symbol
pub fn test_workspace_symbol(
    mut test_case: TestCase,
    query: &str,
    cmp: Option<WorkspaceSymbolComparator>,
    expected: Option<&WorkspaceSymbolResponse>,
) -> TestResult<(), WorkspaceSymbolResponse> {
    test_case.test_type = Some(TestType::WorkspaceSymbol);
    let query_json =
        serde_json::to_string_pretty(query).expect("JSON serialization of `query` failed");
    collect_results(
        &test_case,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamDirect {
                name: "query",
                json: query_json,
            },
        ],
        expected,
        cmp,
    )
}

pub type WorkspaceSymbolResolveComparator =
    fn(&WorkspaceSymbol, &WorkspaceSymbol, &TestCase) -> bool;

/// Tests the server's response to a [`workspaceSymbol/resolve`] request
///
/// - `params`: Passed to the client via the request's [`WorkspaceSymbol`] param
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
/// [`workspaceSymbole/resolve`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_symbolResolve
#[allow(clippy::result_large_err)]
pub fn test_workspace_symbol_resolve(
    mut test_case: TestCase,
    params: &WorkspaceSymbol,
    cmp: Option<WorkspaceSymbolResolveComparator>,
    expected: &WorkspaceSymbol,
) -> TestResult<(), WorkspaceSymbol> {
    test_case.test_type = Some(TestType::WorkspaceSymbolResolve);
    let params_json =
        serde_json::to_string_pretty(params).expect("JSON serialization of `params` failed");
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamDestructure {
            name: "symbol",
            fields: vec!["name", "kind", "tags", "containerName", "location", "data"],
            json: params_json,
        }],
        Some(expected),
        cmp,
    )
}

pub type WorkspaceWillCreateFilesComparator = fn(&WorkspaceEdit, &WorkspaceEdit, &TestCase) -> bool;

/// Tests the server's response to a [`workspace/willCreateFiles`] request
///
/// - `params`: Passed to the client via the request's [`CreateFilesParams`] param
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
/// [`workspaceSymbole/resolve`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_willCreateFiles
#[allow(clippy::result_large_err)]
pub fn test_workspace_will_create_files(
    mut test_case: TestCase,
    params: &CreateFilesParams,
    cmp: Option<WorkspaceWillCreateFilesComparator>,
    expected: Option<&WorkspaceEdit>,
) -> TestResult<(), WorkspaceEdit> {
    test_case.test_type = Some(TestType::WorkspaceWillCreateFiles);
    let params_json =
        serde_json::to_string_pretty(params).expect("JSON serialization of `params` failed");
    collect_results(
        &test_case,
        &mut vec![LuaReplacement::ParamDestructure {
            name: "create_params",
            fields: vec!["files"],
            json: params_json,
        }],
        expected,
        cmp,
    )
}
