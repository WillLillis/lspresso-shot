mod init_dot_lua;
pub mod types;

use init_dot_lua::LuaReplacement;
use lsp_types::{
    CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall, CodeAction,
    CodeActionContext, CodeActionResponse, CodeLens, Color, ColorInformation, ColorPresentation,
    CompletionItem, CompletionResponse, CreateFilesParams, DeleteFilesParams, Diagnostic,
    DocumentDiagnosticReport, DocumentHighlight, DocumentLink, DocumentSymbolResponse,
    FoldingRange, FormattingOptions, GotoDefinitionResponse, Hover, InlayHint, LinkedEditingRanges,
    Location, Moniker, OneOf, Position, PrepareRenameResponse, PreviousResultId, Range,
    RelatedFullDocumentDiagnosticReport, RenameFilesParams, SelectionRange,
    SemanticTokensFullDeltaResult, SemanticTokensRangeResult, SemanticTokensResult, SignatureHelp,
    SignatureHelpContext, SymbolKind, TextEdit, TypeHierarchyItem, Uri, WorkspaceDiagnosticReport,
    WorkspaceEdit, WorkspaceSymbol, WorkspaceSymbolResponse,
    request::{GotoDeclarationResponse, GotoImplementationResponse, GotoTypeDefinitionResponse},
};

// These imports are included solely for the sake of highlighting in doc comments
#[allow(unused_imports)]
use lsp_types::{
    CallHierarchyIncomingCallsParams, CallHierarchyOutgoingCallsParams, CallHierarchyPrepareParams,
    CodeActionParams, ColorPresentationParams, CompletionParams, DocumentDiagnosticParams,
    DocumentHighlightParams, DocumentOnTypeFormattingParams, DocumentRangeFormattingParams,
    ExecuteCommandParams, GotoDefinitionParams, HoverParams, InlayHintParams, MonikerParams,
    ReferenceParams, RenameParams, SelectionRangeParams, SemanticTokensRangeParams,
    SignatureHelpParams, TextDocumentPositionParams, TypeHierarchyPrepareParams,
    WorkspaceDiagnosticParams, WorkspaceSymbolParams,
    request::{GotoDeclarationParams, GotoImplementationParams, GotoTypeDefinitionParams},
};
use serde_json::Value;
#[allow(unused_imports)]
use types::ServerStartType;

use std::{
    collections::HashMap,
    fs,
    path::Path,
    process::{Command, Stdio},
    str::FromStr as _,
    sync::{Arc, Condvar, Mutex, OnceLock},
    time::Duration,
};

use types::{
    ApproximateEq, BenchmarkConfig, BenchmarkError, CleanResponse, EndCondition,
    ResponseMismatchError, StateOrResponse, TestCase, TestError, TestExecutionError,
    TestExecutionResult, TestResult, TestType, TimeoutError, to_parent_err_type,
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
    test_type: TestType,
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
    let source_path = test_case.create_test(test_type, replacements)?;
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

fn benchmark<T>(
    test_case: &TestCase,
    config: BenchmarkConfig,
    action: impl Fn() -> TestResult<(), T>,
) -> Result<Vec<Duration>, BenchmarkError> {
    let handle_result = |res: TestResult<(), T>, fail_fast: bool| -> Result<(), BenchmarkError> {
        match (fail_fast, res) {
            (true, Err(TestError::ResponseMismatch(_)) | Ok(())) | (false, _) => Ok(()),
            (true, Err(TestError::TestSetup(setup))) => Err(BenchmarkError::TestSetup(setup)),
            (true, Err(TestError::TestExecution(execution))) => {
                Err(BenchmarkError::TestExecution(execution))
            }
        }
    };
    match config.end_condition {
        EndCondition::Time(duration) => {
            let start = std::time::Instant::now();
            while start.elapsed() < duration {
                handle_result(action(), config.fail_fast)?;
            }
        }
        EndCondition::Count(iterations) => {
            for _ in 0..iterations {
                handle_result(action(), config.fail_fast)?;
            }
        }
    }
    test_case.get_benchmark_results()
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
    test_case: &TestCase,
    range: Range,
    context: &CodeActionContext,
    cmp: Option<CodeActionComparator>,
    expected: Option<&CodeActionResponse>,
) -> TestResult<(), CodeActionResponse> {
    let context_json =
        serde_json::to_string_pretty(context).expect("JSON serialization of `context` failed");
    collect_results(
        test_case,
        TestType::CodeAction,
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

/// Benchmarks the server's response time to a [`textDocument/codeAction`] request
///
/// - `end_condition`: Specifies how long the benchmark should run.
/// - `range`: Passed to the client via the request's [`CodeActionParams`]
/// - `context`: Passed to the client via the request's [`CodeActionParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `context` fails
///
/// [`textDocument/codeAction`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_codeAction
pub fn benchmark_code_action(
    test_case: &TestCase,
    config: BenchmarkConfig,
    range: Range,
    context: &CodeActionContext,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_code_action(test_case, range, context, None, None)
    })
}

pub type CodeActionResolveComparator = fn(&CodeAction, &CodeAction, &TestCase) -> bool;

/// Tests the server's response to a [`codeAction/resolve`] request
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
/// [`codeAction/resolve`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#codeAction_resolve
#[allow(clippy::result_large_err)]
pub fn test_code_action_resolve(
    test_case: &TestCase,
    params: &CodeAction,
    cmp: Option<CodeActionResolveComparator>,
    expected: &CodeAction,
) -> TestResult<(), CodeAction> {
    let code_action_json =
        serde_json::to_string_pretty(params).expect("JSON serialization of `params` failed");
    collect_results(
        test_case,
        TestType::CodeActionResolve,
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

/// Benchmarks the server's response time to a [`codeAction/resolve`] request
///
/// - `config`: Specifies how long the benchmark should run.
/// - `params`: Passed to the client via the request's [`CodeAction`] param
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `params` fails
///
/// [`codeAction/resolve`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#codeAction_resolve
pub fn benchmark_code_action_resolve(
    test_case: &TestCase,
    config: BenchmarkConfig,
    params: &CodeAction,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_code_action_resolve(test_case, params, None, &CodeAction::default())
    })
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
    test_case: &TestCase,
    commands: Option<&Vec<String>>,
    cmp: Option<CodeLensComparator>,
    expected: Option<&Vec<CodeLens>>,
) -> TestResult<(), Vec<CodeLens>> {
    let command_str = commands.map_or_else(String::new, |cmds| {
        cmds.iter()
            .fold(String::new(), |accum, cmd| accum + &format!("\"{cmd}\",\n"))
    });
    collect_results(
        test_case,
        TestType::CodeLens,
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

/// Benchmarks the server's response time to a [`textDocument/codeLens`] request
///
/// - `commands`: A list of LSP command names the client should advertise support for in its
///   capabilities (e.g. "rust-analyzer.runSingle"). This enables command-based `CodeLens`
///   responses from the server, such as "Run" or "Debug" actions.
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// [`textDocument/codeLens`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_codeLens
pub fn benchmark_code_lens(
    test_case: &TestCase,
    config: BenchmarkConfig,
    commands: Option<&Vec<String>>,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_code_lens(test_case, commands, None, None)
    })
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
    test_case: &TestCase,
    commands: Option<&Vec<String>>,
    code_lens: &CodeLens,
    cmp: Option<CodeLensResolveComparator>,
    expected: Option<&CodeLens>,
) -> TestResult<(), CodeLens> {
    let command_str = commands.map_or_else(String::new, |cmds| {
        cmds.iter()
            .fold(String::new(), |accum, cmd| accum + &format!("\"{cmd}\",\n"))
    });
    let code_lens_json =
        serde_json::to_string_pretty(code_lens).expect("JSON serialization of `code_lens` failed");
    collect_results(
        test_case,
        TestType::CodeLensResolve,
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

/// Benchmarks the server's response time to a [`codeLens/resolve`] request
///
/// - `commands` is a list of LSP command names the client should advertise support for in its
///   capabilities (e.g. "rust-analyzer.runSingle"). This enables command-based [`CodeLens`]
///   responses from the server, such as "Run" or "Debug" actions.
/// - `code_lens`: Passed to the client via the request's [`CodeLens`] param
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `code_lens` fails
///
/// [`codeLens/resolve`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#codeLens_resolve
pub fn benchmark_code_lens_resolve(
    test_case: &TestCase,
    config: BenchmarkConfig,
    commands: Option<&Vec<String>>,
    code_lens: &CodeLens,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_code_lens_resolve(test_case, commands, code_lens, None, None)
    })
}

pub type ColorPresentationComparator =
    fn(&Vec<ColorPresentation>, &Vec<ColorPresentation>, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/colorPresentation`] request
///
/// - `color`: Passed to the client via the request's [`ColorPresentationParams`] param
/// - `range`: Passed to the client via the request's [`ColorPresentationParams`] param
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
    test_case: &TestCase,
    color: Color,
    range: Range,
    cmp: Option<ColorPresentationComparator>,
    expected: &Vec<ColorPresentation>,
) -> TestResult<(), Vec<ColorPresentation>> {
    let color_json =
        serde_json::to_string_pretty(&color).expect("JSON serialization of `color` failed");
    collect_results(
        test_case,
        TestType::ColorPresentation,
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

/// Benchmarks the server's response time to a [`textDocument/colorPresentation`] request
///
/// - `color`: Passed to the client via the request's [`ColorPresentationParams`] param
/// - `range`: Passed to the client via the request's [`ColorPresentationParams`] param
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `color` fails
///
/// [`textDocument/colorPresentation`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_colorPresentation
pub fn benchmark_color_presentation(
    test_case: &TestCase,
    config: BenchmarkConfig,
    color: Color,
    range: Range,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_color_presentation(test_case, color, range, None, &vec![])
    })
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
    test_case: &TestCase,
    cursor_pos: Position,
    cmp: Option<CompletionComparator>,
    expected: Option<&CompletionResponse>,
) -> TestResult<(), CompletionResponse> {
    collect_results(
        test_case,
        TestType::Completion,
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

/// Benchmarks the server's response time to a [`textDocument/completion`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`CompletionParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// [`textDocument/completion`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_completion
pub fn benchmark_completion(
    test_case: &TestCase,
    config: BenchmarkConfig,
    cursor_pos: Position,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_completion(test_case, cursor_pos, None, None)
    })
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
    test_case: &TestCase,
    completion_item: &CompletionItem,
    cmp: Option<CompletionResolveComparator>,
    expected: Option<&CompletionItem>,
) -> TestResult<(), CompletionItem> {
    let completion_item_json = serde_json::to_string_pretty(completion_item)
        .expect("JSON serialization of `completion_item` failed");
    collect_results(
        test_case,
        TestType::CompletionResolve,
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

/// Benchmarks the server's response time to a [`completionItem/resolve`] request
///
/// - `completion_item`: Passed to the client via the request's [`CompletionItem`] param
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `completion_item` fails
///
/// [`completionItem/resolve`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#completionItem_resolve
pub fn benchmark_completion_resolve(
    test_case: &TestCase,
    config: BenchmarkConfig,
    completion_item: &CompletionItem,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_completion_resolve(test_case, completion_item, None, None)
    })
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
    test_case: &TestCase,
    cursor_pos: Position,
    cmp: Option<DeclarationComparator>,
    expected: Option<&GotoDeclarationResponse>,
) -> TestResult<(), GotoDeclarationResponse> {
    collect_results(
        test_case,
        TestType::Declaration,
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

/// Benchmarks the server's response time to a [`textDocument/declaration`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`GotoDeclarationParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// [`textDocument/declaration`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_declaration
pub fn benchmark_declaration(
    test_case: &TestCase,
    config: BenchmarkConfig,
    cursor_pos: Position,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_declaration(test_case, cursor_pos, None, None)
    })
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
    test_case: &TestCase,
    cursor_pos: Position,
    cmp: Option<DefinitionComparator>,
    expected: Option<&GotoDefinitionResponse>,
) -> TestResult<(), GotoDefinitionResponse> {
    collect_results(
        test_case,
        TestType::Definition,
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

/// Benchmarks the server's response time to a [`textDocument/definition`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`GotoDefinitionParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// [`textDocument/definition`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_definition
pub fn benchmark_definition(
    test_case: &TestCase,
    config: BenchmarkConfig,
    cursor_pos: Position,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_definition(test_case, cursor_pos, None, None)
    })
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
    test_case: &TestCase,
    identifier: Option<&str>,
    // TODO: Consider removing since we use the first result
    previous_result_id: Option<&str>,
    cmp: Option<DiagnosticComparator>,
    expected: &DocumentDiagnosticReport,
) -> TestResult<(), DocumentDiagnosticReport> {
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
        test_case,
        TestType::Diagnostic,
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

/// Benchmarks the server's response time to a [`textDocument/diagnostic`] request
///
/// - `identifier`: Passed to the client via the request's [`DocumentDiagnosticParams`]
/// - `previous_result_id`: Passed to the client via the request's [`DocumentDiagnosticParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `identifier` or `previous_result_id` fails
///
/// [`textDocument/diagnostic`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_diagnostic
pub fn benchmark_diagnostic(
    test_case: &TestCase,
    config: BenchmarkConfig,
    identifier: Option<&str>,
    previous_result_id: Option<&str>,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_diagnostic(
            test_case,
            identifier,
            previous_result_id,
            None,
            &DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport::default()),
        )
    })
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
    test_case: &TestCase,
    cmp: Option<DocumentColorComparator>,
    expected: &Vec<ColorInformation>,
) -> TestResult<(), Vec<ColorInformation>> {
    collect_results(
        test_case,
        TestType::DocumentColor,
        &mut vec![LuaReplacement::ParamTextDocument],
        Some(expected),
        cmp,
    )
}

/// Benchmarks the server's response time to a [`textDocument/documentColor`] request
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// [`textDocument/documentColor`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_documentColor
pub fn benchmark_document_color(
    test_case: &TestCase,
    config: BenchmarkConfig,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_document_color(test_case, None, &vec![])
    })
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
    test_case: &TestCase,
    cursor_pos: Position,
    cmp: Option<DocumentHighlightComparator>,
    expected: Option<&Vec<DocumentHighlight>>,
) -> TestResult<(), Vec<DocumentHighlight>> {
    collect_results(
        test_case,
        TestType::DocumentHighlight,
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

/// Benchmarks the server's response time to a [`textDocument/documentHighlight`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`DocumentHighlightParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// [`textDocument/documentHighlight`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_documentHighlight
pub fn benchmark_document_highlight(
    test_case: &TestCase,
    config: BenchmarkConfig,
    cursor_pos: Position,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_document_highlight(test_case, cursor_pos, None, None)
    })
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
    test_case: &TestCase,
    cmp: Option<DocumentLinkComparator>,
    expected: Option<&Vec<DocumentLink>>,
) -> TestResult<(), Vec<DocumentLink>> {
    collect_results(
        test_case,
        TestType::DocumentLink,
        &mut vec![LuaReplacement::ParamTextDocument],
        expected,
        cmp,
    )
}

/// Benchmarks the server's response time to a [`textDocument/documentLink`] request
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// [`textDocument/documentLink`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_documentLink
pub fn benchmark_document_link(
    test_case: &TestCase,
    config: BenchmarkConfig,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_document_link(test_case, None, None)
    })
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
    test_case: &TestCase,
    params: &DocumentLink,
    cmp: Option<DocumentLinkResolveComparator>,
    expected: Option<&DocumentLink>,
) -> TestResult<(), DocumentLink> {
    let document_link_json =
        serde_json::to_string_pretty(params).expect("JSON serialization of `params` failed");
    collect_results(
        test_case,
        TestType::DocumentLinkResolve,
        &mut vec![LuaReplacement::ParamDestructure {
            name: "link",
            fields: vec!["range", "target", "tooltip", "data"],
            json: document_link_json,
        }],
        expected,
        cmp,
    )
}

/// Benchmarks the server's response time to a [`documentLink/resolve`] request
///
/// - `link`: Passed to the client via the request's [`DocumentLink`] params
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
pub fn benchmark_document_link_resolve(
    test_case: &TestCase,
    config: BenchmarkConfig,
    params: &DocumentLink,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_document_link_resolve(test_case, params, None, None)
    })
}

pub type DocumentSymbolComparator =
    fn(&DocumentSymbolResponse, &DocumentSymbolResponse, &TestCase) -> bool;

/// Tests the server's response to a [`textDocument/documentSymbol`] request
///
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Warnings
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
    test_case: &TestCase,
    cmp: Option<DocumentSymbolComparator>,
    expected: Option<&DocumentSymbolResponse>,
) -> TestResult<(), DocumentSymbolResponse> {
    collect_results(
        test_case,
        TestType::DocumentSymbol,
        &mut vec![LuaReplacement::ParamTextDocument],
        expected,
        cmp,
    )
}

/// Benchmarks the server's response time to a [`textDocument/documentSymbol`] request
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/documentSymbol`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_documentSymbol
pub fn benchmark_document_symbol(
    test_case: &TestCase,
    config: BenchmarkConfig,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_document_symbol(test_case, None, None)
    })
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
    test_case: &TestCase,
    cmp: Option<FoldingRangeComparator>,
    expected: Option<&Vec<FoldingRange>>,
) -> TestResult<(), Vec<FoldingRange>> {
    collect_results(
        test_case,
        TestType::FoldingRange,
        &mut vec![LuaReplacement::ParamTextDocument],
        expected,
        cmp,
    )
}

/// Benchmarks the server's response time to a [`textDocument/foldingRange`] request
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// [`textDocument/foldingRange`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_foldingRange
pub fn benchmark_folding_range(
    test_case: &TestCase,
    config: BenchmarkConfig,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_folding_range(test_case, None, None)
    })
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

pub type FormattingComparator =
    fn(&StateOrResponse<Vec<TextEdit>>, &StateOrResponse<Vec<TextEdit>>, &TestCase) -> bool;

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
    test_case: &TestCase,
    options: Option<&FormattingOptions>,
    cmp: Option<FormattingComparator>,
    expected: Option<&StateOrResponse<Vec<TextEdit>>>,
) -> TestResult<(), StateOrResponse<Vec<TextEdit>>> {
    let options_json = options
        .map_or_else(
            || serde_json::to_string_pretty(&default_format_opts()),
            serde_json::to_string_pretty,
        )
        .expect("JSON serialization of `options` failed");
    // map the child error types of `test_formatting_*` to `TestError<StateOrResponse<Vec<TextEdit>>>`
    match expected {
        Some(StateOrResponse::Response(edits)) => to_parent_err_type(test_formatting_resp(
            test_case,
            TestType::Formatting,
            options_json,
            cmp,
            Some(edits),
        )),
        Some(StateOrResponse::State(state)) => to_parent_err_type(test_formatting_state(
            test_case,
            TestType::Formatting,
            options_json,
            cmp,
            state.to_string(),
        )),
        None => to_parent_err_type(test_formatting_resp(
            test_case,
            TestType::Formatting,
            options_json,
            cmp,
            None,
        )),
    }
}

/// Benchmarks the server's response time to a [`textDocument/formatting`] request.
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
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `options` fails
///
/// [`textDocument/formatting`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_formatting
pub fn benchmark_formatting(
    test_case: &TestCase,
    config: BenchmarkConfig,
    options: Option<&FormattingOptions>,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_formatting(test_case, options, None, None)
    })
}

/// Performs the test for [`test_formatting`] when the expected result is `Some(Vec<TextEdit>)` or
/// `None`.
fn test_formatting_resp(
    test_case: &TestCase,
    test_type: TestType,
    options_json: String,
    cmp: Option<FormattingComparator>,
    expected: Option<&Vec<TextEdit>>,
) -> TestResult<(), Vec<TextEdit>> {
    let outer_cmp =
        |expected: &Vec<TextEdit>, actual: &Vec<TextEdit>, test_case: &TestCase| -> bool {
            let result_expected = StateOrResponse::Response(expected.clone());
            let result_actual = StateOrResponse::Response(actual.clone());
            cmp.as_ref().map_or_else(
                || result_expected == result_actual,
                |cmp_fn| cmp_fn(&result_expected, &result_actual, test_case),
            )
        };
    collect_results(
        test_case,
        test_type,
        &mut vec![
            LuaReplacement::Other {
                from: "INVOKE_ACTION",
                to: "false".to_string(),
            },
            LuaReplacement::ParamTextDocument,
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
    test_type: TestType,
    options_json: String,
    cmp: Option<FormattingComparator>,
    expected: String,
) -> TestResult<(), String> {
    let outer_cmp = |expected: &String, actual: &String, test_case: &TestCase| -> bool {
        let result_expected = StateOrResponse::State(expected.to_string());
        let result_actual = StateOrResponse::State(actual.to_string());
        cmp.as_ref().map_or_else(
            || result_expected == result_actual,
            |cmp_fn| cmp_fn(&result_expected, &result_actual, test_case),
        )
    };
    collect_results(
        test_case,
        test_type,
        &mut vec![
            LuaReplacement::Other {
                from: "INVOKE_ACTION",
                to: "true".to_string(),
            },
            LuaReplacement::Other {
                from: "INVOKE_FN",
                to: "vim.lsp.buf.format".to_string(),
            },
            LuaReplacement::ParamDirect {
                name: "formatting_options",
                json: options_json,
            },
            LuaReplacement::ParamDirect {
                name: "async",
                json: "false".to_string(),
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
    test_case: &TestCase,
    cursor_pos: Position,
    cmp: Option<HoverComparator>,
    expected: Option<&Hover>,
) -> TestResult<(), Hover> {
    collect_results(
        test_case,
        TestType::Hover,
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

/// Benchmarks the server's response time to a [`textDocument/hover`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`HoverParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// [`textDocument/hover`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_hover
pub fn benchmark_hover(
    test_case: &TestCase,
    config: BenchmarkConfig,
    cursor_pos: Position,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_hover(test_case, cursor_pos, None, None)
    })
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
    test_case: &TestCase,
    cursor_pos: Position,
    cmp: Option<ImplementationComparator>,
    expected: Option<&GotoImplementationResponse>,
) -> TestResult<(), GotoImplementationResponse> {
    collect_results(
        test_case,
        TestType::Implementation,
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

/// Benchmarks the server's response time to a [`textDocument/implementation`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`GotoImplementationParams`]
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// [`textDocument/implementation`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_implementation
pub fn benchmark_implementation(
    test_case: &TestCase,
    config: BenchmarkConfig,
    cursor_pos: Position,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_implementation(test_case, cursor_pos, None, None)
    })
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
    test_case: &TestCase,
    call_item: &CallHierarchyItem,
    cmp: Option<IncomingCallsComparator>,
    expected: Option<&Vec<CallHierarchyIncomingCall>>,
) -> TestResult<(), Vec<CallHierarchyIncomingCall>> {
    let call_item_json =
        serde_json::to_string_pretty(call_item).expect("JSON serialization of `call_item` failed");
    collect_results(
        test_case,
        TestType::IncomingCalls,
        &mut vec![LuaReplacement::ParamDirect {
            name: "item",
            json: call_item_json,
        }],
        expected,
        cmp,
    )
}

/// Benchmarks the server's response time to a [`callHierarchy/incomingCalls`] request
///
/// - `call_item`: Passed to the client via the request's [`CallHierarchyIncomingCallsParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `call_item` fails
///
/// [`callHierarchy/incomingCalls`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#callHierarchy_incomingCalls
pub fn benchmark_incoming_calls(
    test_case: &TestCase,
    config: BenchmarkConfig,
    call_item: &CallHierarchyItem,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_incoming_calls(test_case, call_item, None, None)
    })
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
    test_case: &TestCase,
    range: Range,
    cmp: Option<InlayHintComparator>,
    expected: Option<&Vec<InlayHint>>,
) -> TestResult<(), Vec<InlayHint>> {
    collect_results(
        test_case,
        TestType::InlayHint,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamRange(range),
        ],
        expected,
        cmp,
    )
}

/// Benchmarks the server's response time to a [`textDocument/inlayHint`] request
///
/// - `range`: Passed to the client via the request's [`InlayHintParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// [`textDocument/inlayHint`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_inlayHint
pub fn benchmark_inlay_hint(
    test_case: &TestCase,
    config: BenchmarkConfig,
    range: Range,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_inlay_hint(test_case, range, None, None)
    })
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
    test_case: &TestCase,
    cursor_pos: Position,
    cmp: Option<LinkedEditingRangeComparator>,
    expected: Option<&LinkedEditingRanges>,
) -> TestResult<(), LinkedEditingRanges> {
    collect_results(
        test_case,
        TestType::LinkedEditingRange,
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

/// Benchmarks the server's response time to a [`textDocument/linkedEditingRange`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`MonikerParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `call_item` fails
///
/// [`textDocument/moniker`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_linkedEditingRange
pub fn benchmark_linked_editing_range(
    test_case: &TestCase,
    config: BenchmarkConfig,
    cursor_pos: Position,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_linked_editing_range(test_case, cursor_pos, None, None)
    })
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
    test_case: &TestCase,
    cursor_pos: Position,
    cmp: Option<MonikerComparator>,
    expected: Option<&Vec<Moniker>>,
) -> TestResult<(), Vec<Moniker>> {
    collect_results(
        test_case,
        TestType::Moniker,
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

/// Benchmarks the server's response time to a [`textDocument/moniker`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`MonikerParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `call_item` fails
///
/// [`textDocument/moniker`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_moniker
pub fn benchmark_moniker(
    test_case: &TestCase,
    config: BenchmarkConfig,
    cursor_pos: Position,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_moniker(test_case, cursor_pos, None, None)
    })
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
    test_case: &TestCase,
    cursor_pos: Position,
    character: &str,
    options: Option<&FormattingOptions>,
    cmp: Option<OnTypeFormattingComparator>,
    expected: Option<&Vec<TextEdit>>,
) -> TestResult<(), Vec<TextEdit>> {
    let character_json =
        serde_json::to_string_pretty(character).expect("JSON serialization of `character` failed");
    let options_json = options
        .map_or_else(
            || serde_json::to_string_pretty(&default_format_opts()),
            serde_json::to_string_pretty,
        )
        .expect("JSON serialization of `options` failed");
    collect_results(
        test_case,
        TestType::OnTypeFormatting,
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
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `character` or `options` fails
///
/// [`callHierarchy/outgoingCalls`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_onTypeFormatting
pub fn benchmark_on_type_formatting(
    test_case: &TestCase,
    config: BenchmarkConfig,
    cursor_pos: Position,
    character: &str,
    options: Option<&FormattingOptions>,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_on_type_formatting(test_case, cursor_pos, character, options, None, None)
    })
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
    test_case: &TestCase,
    call_item: &CallHierarchyItem,
    cmp: Option<OutgoingCallsComparator>,
    expected: Option<&Vec<CallHierarchyOutgoingCall>>,
) -> TestResult<(), Vec<CallHierarchyOutgoingCall>> {
    let call_item_json =
        serde_json::to_string_pretty(call_item).expect("JSON serialization of `call_item` failed");
    collect_results(
        test_case,
        TestType::OutgoingCalls,
        &mut vec![LuaReplacement::ParamDirect {
            name: "item",
            json: call_item_json,
        }],
        expected,
        cmp,
    )
}

/// Benchmarks the server's response time to a [`callHierarchy/outgoingCalls`] request
///
/// - `call_item`: Passed to the client via the request's [`CallHierarchyOutgoingCallsParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `call_item` fails
///
/// [`callHierarchy/outgoingCalls`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#callHierarchy_outgoingCalls
pub fn benchmark_outgoing_calls(
    test_case: &TestCase,
    config: BenchmarkConfig,
    call_item: &CallHierarchyItem,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_outgoing_calls(test_case, call_item, None, None)
    })
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
    test_case: &TestCase,
    cursor_pos: Position,
    cmp: Option<PrepareCallHierarchyComparator>,
    expected: Option<&Vec<CallHierarchyItem>>,
) -> TestResult<(), Vec<CallHierarchyItem>> {
    collect_results(
        test_case,
        TestType::PrepareCallHierarchy,
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

/// Benchmarks the server's response time to a [`textDocument/prepareCallHierarchy`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`CallHierarchyPrepareParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// [`textDocument/prepareCallHierarchy`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_prepareCallHierarchy
pub fn benchmark_prepare_call_hierarchy(
    test_case: &TestCase,
    config: BenchmarkConfig,
    cursor_pos: Position,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_prepare_call_hierarchy(test_case, cursor_pos, None, None)
    })
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
    test_case: &TestCase,
    cursor_pos: Position,
    cmp: Option<PrepareRenameComparator>,
    expected: Option<&PrepareRenameResponse>,
) -> TestResult<(), PrepareRenameResponse> {
    collect_results(
        test_case,
        TestType::PrepareRename,
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

/// Benchmarks the server's response time to a [`textDocument/prepareRename`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`TextDocumentPositionParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// [`textDocument/prepareCallHierarchy`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_prepareRename
pub fn benchmark_prepare_rename(
    test_case: &TestCase,
    config: BenchmarkConfig,
    cursor_pos: Position,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_prepare_rename(test_case, cursor_pos, None, None)
    })
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
    test_case: &TestCase,
    cursor_pos: Position,
    items: Option<&Vec<TypeHierarchyItem>>,
    cmp: Option<PrepareTypeHierarchyComparator>,
    expected: Option<&Vec<TypeHierarchyItem>>,
) -> TestResult<(), Vec<TypeHierarchyItem>> {
    // TODO: We may need to prepend the relative paths in `items` with the test case root
    let items_json = items.map_or_else(
        || "null".to_string(),
        |thi| serde_json::to_string_pretty(thi).expect("JSON serialization of type `items` failed"),
    );
    collect_results(
        test_case,
        TestType::PrepareTypeHierarchy,
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

/// Benchmarks the server's response time to a [`textDocument/prepareTypeHierarchy`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`TypeHierarchyPrepareParams`]
/// - `items`: Type hierarchy items provided to the client via [`TypeHierarchyPrepareParams`].
///   The `uri` field of each item should be *relative* to the test case root, instead of an
///   absolute path. (i.e. `uri = "file://src/test_file.rs"`)
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `items` fails
///
/// [`textDocument/prepareTypeHierarchy`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_prepareTypeHierarchy
pub fn benchmark_prepare_type_hierarchy(
    test_case: &TestCase,
    config: BenchmarkConfig,
    cursor_pos: Position,
    items: Option<&Vec<TypeHierarchyItem>>,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_prepare_type_hierarchy(test_case, cursor_pos, items, None, None)
    })
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
    test_case: &TestCase,
    cmp: Option<PublishDiagnosticsComparator>,
    expected: &Vec<Diagnostic>,
) -> TestResult<(), Vec<Diagnostic>> {
    collect_results(
        test_case,
        TestType::PublishDiagnostics,
        &mut Vec::new(),
        Some(expected),
        cmp,
    )
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
    test_case: &TestCase,
    range: Range,
    options: Option<&FormattingOptions>,
    cmp: Option<RangeFormattingComparator>,
    expected: Option<&Vec<TextEdit>>,
) -> TestResult<(), Vec<TextEdit>> {
    let options_json = options
        .map_or_else(
            || serde_json::to_string_pretty(&default_format_opts()),
            serde_json::to_string_pretty,
        )
        .expect("JSON serialization of `options` failed");
    collect_results(
        test_case,
        TestType::RangeFormatting,
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

/// Benchmarks the server's response time to a [`textDocument/rangeFormatting`] request
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
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `options` fails
///
/// [`textDocument/rangeFormatting`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_rangeFormatting
pub fn benchmark_range_formatting(
    test_case: &TestCase,
    config: BenchmarkConfig,
    range: Range,
    options: Option<&FormattingOptions>,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_range_formatting(test_case, range, options, None, None)
    })
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
    test_case: &TestCase,
    cursor_pos: Position,
    include_declaration: bool,
    cmp: Option<ReferencesComparator>,
    expected: Option<&Vec<Location>>,
) -> TestResult<(), Vec<Location>> {
    let include_decl_json = serde_json::to_string_pretty(&include_declaration)
        .expect("JSON serialization of `include_declaration` failed");
    collect_results(
        test_case,
        TestType::References,
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

/// Benchmarks the server's response time to a [`textDocument/references`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`ReferenceParams`]
/// - `include_declaration`: Passed to the client via the request's [`ReferenceParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `include_declaration` fails
///
/// [`textDocument/references`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_references
pub fn benchmark_references(
    test_case: &TestCase,
    config: BenchmarkConfig,
    cursor_pos: Position,
    include_declaration: bool,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_references(test_case, cursor_pos, include_declaration, None, None)
    })
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
    test_case: &TestCase,
    cursor_pos: Position,
    new_name: &str,
    cmp: Option<RenameComparator>,
    expected: Option<&WorkspaceEdit>,
) -> TestResult<(), WorkspaceEdit> {
    let new_name_json =
        serde_json::to_string_pretty(new_name).expect("JSON serialization of `new_name` failed");
    collect_results(
        test_case,
        TestType::Rename,
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

/// Benchmarks the server's response time to a [`textDocument/rename`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`RenameParams`]
/// - `new_name`: Passed to the client via the request's [`RenameParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `new_name` fails
///
/// [`textDocument/rename`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_rename
pub fn benchmark_rename(
    test_case: &TestCase,
    config: BenchmarkConfig,
    cursor_pos: Position,
    new_name: &str,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_rename(test_case, cursor_pos, new_name, None, None)
    })
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
    test_case: &TestCase,
    positions: &Vec<Position>,
    cmp: Option<SelectionRangeComparator>,
    expected: Option<&Vec<SelectionRange>>,
) -> TestResult<(), Vec<SelectionRange>> {
    let positions_json =
        serde_json::to_string_pretty(positions).expect("JSON serialization of `positions` failed");
    collect_results(
        test_case,
        TestType::SelectionRange,
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

/// Benchmarks the server's response time to a [`textDocument/selectionRange`] request
///
/// - `positions`: Passed to the client via the request's [`SelectionRangeParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `positions` fails
///
/// [`textDocument/typeDefinition`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_selectionRange
pub fn benchmark_selection_range(
    test_case: &TestCase,
    config: BenchmarkConfig,
    positions: &Vec<Position>,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_selection_range(test_case, positions, None, None)
    })
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
    test_case: &TestCase,
    cmp: Option<SemanticTokensFullComparator>,
    expected: Option<&SemanticTokensResult>,
) -> TestResult<(), SemanticTokensResult> {
    collect_results(
        test_case,
        TestType::SemanticTokensFull,
        &mut vec![LuaReplacement::ParamTextDocument],
        expected,
        cmp,
    )
}

/// Tests the server's response to a [`textDocument/semanticTokens/full`] request
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// [`textDocument/semanticTokens/full`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#semanticTokens_fullRequest
pub fn benchmark_semantic_tokens_full(
    test_case: &TestCase,
    config: BenchmarkConfig,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_semantic_tokens_full(test_case, None, None)
    })
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
    test_case: &TestCase,
    cmp: Option<SemanticTokensFullDeltaComparator>,
    expected: Option<&SemanticTokensFullDeltaResult>,
) -> TestResult<(), SemanticTokensFullDeltaResult> {
    collect_results(
        test_case,
        TestType::SemanticTokensFullDelta,
        &mut vec![LuaReplacement::ParamTextDocument],
        expected,
        cmp,
    )
}

/// Benchmarks the server's response time to a [`textDocument/semanticTokens/full/delta`] request
///
/// First sends a [`textDocument/semanticTokens/full`] request to get the initial state,
/// and then issues a [`textDocument/semanticTokens/full/delta`] request if the first
/// response contained a `result_id`.
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// [`textDocument/semanticTokens/full`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#semanticTokens_fullRequest
/// [`textDocument/semanticTokens/full/delta`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#semanticTokens_deltaRequest
pub fn benchmark_semantic_tokens_full_delta(
    test_case: &TestCase,
    config: BenchmarkConfig,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_semantic_tokens_full_delta(test_case, None, None)
    })
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
    test_case: &TestCase,
    range: Range,
    cmp: Option<SemanticTokensRangeComparator>,
    expected: Option<&SemanticTokensRangeResult>,
) -> TestResult<(), SemanticTokensRangeResult> {
    collect_results(
        test_case,
        TestType::SemanticTokensRange,
        &mut vec![
            LuaReplacement::ParamTextDocument,
            LuaReplacement::ParamRange(range),
        ],
        expected,
        cmp,
    )
}

/// Benchmarks the server's response time to a [`textDocument/semanticTokens/range`] request
///
/// - `range`: Passed to the client via the request's [`SemanticTokensRangeParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// [`textDocument/semanticTokens/range`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#semanticTokens_rangeRequest
pub fn benchmark_semantic_tokens_range(
    test_case: &TestCase,
    config: BenchmarkConfig,
    range: Range,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_semantic_tokens_range(test_case, range, None, None)
    })
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
    test_case: &TestCase,
    cursor_pos: Position,
    context: Option<&SignatureHelpContext>,
    cmp: Option<SignatureHelpComparator>,
    expected: Option<&SignatureHelp>,
) -> TestResult<(), SignatureHelp> {
    let context_json = context.map_or_else(
        || "null".to_string(),
        |ctx| serde_json::to_string_pretty(ctx).expect("JSON serialization of `context` failed"),
    );
    collect_results(
        test_case,
        TestType::SignatureHelp,
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

/// Benchmarks the server's response time to a [`textDocument/signatureHelp`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`SignatureHelpParams`]
/// - `context`: Passed to the client via the request's [`SignatureHelpParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `context` fails
///
/// [`textDocument/signatureHelp`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_signatureHelp
pub fn benchmark_signature_help(
    test_case: &TestCase,
    config: BenchmarkConfig,
    cursor_pos: Position,
    context: Option<&SignatureHelpContext>,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_signature_help(test_case, cursor_pos, context, None, None)
    })
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
    test_case: &TestCase,
    cursor_pos: Position,
    cmp: Option<TypeDefinitionComparator>,
    expected: Option<&GotoTypeDefinitionResponse>,
) -> TestResult<(), GotoTypeDefinitionResponse> {
    collect_results(
        test_case,
        TestType::TypeDefinition,
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

/// Benchmarks the server's response time to a [`textDocument/typeDefinition`] request
///
/// - `cursor_pos`: The position of the cursor when the request is issued. Passed
///   to the client via the request's [`GotoTypeDefinitionParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// [`textDocument/typeDefinition`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_typeDefinition
pub fn benchmark_type_definition(
    test_case: &TestCase,
    config: BenchmarkConfig,
    cursor_pos: Position,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_type_definition(test_case, cursor_pos, None, None)
    })
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
    test_case: &TestCase,
    identifier: Option<&str>,
    // TODO: Consider removing since we use the first result
    previous_result_ids: &Vec<PreviousResultId>,
    cmp: Option<WorkspaceDiagnosticComparator>,
    expected: &WorkspaceDiagnosticReport,
) -> TestResult<(), WorkspaceDiagnosticReport> {
    let identifier_json = identifier.map_or_else(
        || "null".to_string(), // NOTE: `vim.json.decode()` fails with an empty string
        |id| serde_json::to_string_pretty(id).expect("JSON serialization of `identifier` failed"),
    );
    let previous_result_ids_json = serde_json::to_string_pretty(previous_result_ids)
        .expect("JSON serialization of `previous_result_id` failed");
    collect_results(
        test_case,
        TestType::WorkspaceDiagnostic,
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

/// Benchmarks the server's response time to a [`workspace/diagnostic`] request
///
/// - `identifier`: Passed to the client via the request's [`WorkspaceDiagnosticParams`]
/// - `previous_result_id`: Passed to the client via the request's [`WorkspaceDiagnosticParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `identifier` or `previous_result_id` fails
///
/// [`workspace/diagnostic`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_diagnostic
pub fn benchmark_workspace_diagnostic(
    test_case: &TestCase,
    config: BenchmarkConfig,
    identifier: Option<&str>,
    previous_result_ids: &Vec<PreviousResultId>,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_workspace_diagnostic(
            test_case,
            identifier,
            previous_result_ids,
            None,
            &WorkspaceDiagnosticReport::default(),
        )
    })
}

pub type WorkspaceExecuteCommandComparator = fn(&Value, &Value, &TestCase) -> bool;

/// Tests the server's response to a [`workspace/executeCommand`] request
///
/// - `commands`: A list of LSP command names the client should advertise support for in its
///   capabilities (e.g. "rust-analyzer.runSingle"). This enables command-based `CodeLens`
/// - `command`: The command to be executed. Passed to the client via the request's
///   [`ExecuteCommandParams`]
/// - `arguments`: The arguments to be passed to the command. Passed to the client via
///   the request's [`ExecuteCommandParams`]
/// - `cmp`: An optional custom comparator function that can be used to determine equality
///   between the expected and actual results.
///
/// # Warnings
///
/// Responses of `serde_json::Value::Null` are equivalent to a lack of response. If you expect
/// to receive a `null` response, you should use `None` as the expected value.
///
/// # Errors
///
/// Returns [`TestError`] if the test case is invalid, the expected results don't match,
/// or some other failure occurs
///
/// # Panics
///
/// Panics if JSON serialization of `command` or `arguments` fails
///
/// [`workspace/executeCommand`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_executeCommand
pub fn test_workspace_execute_command(
    test_case: &TestCase,
    commands: Option<&Vec<String>>,
    command: &str,
    arguments: Option<&Vec<Value>>,
    cmp: Option<WorkspaceExecuteCommandComparator>,
    expected: Option<&Value>,
) -> TestResult<(), Value> {
    let command_str = commands.map_or_else(String::new, |cmds| {
        cmds.iter()
            .fold(String::new(), |accum, cmd| accum + &format!("\"{cmd}\",\n"))
    });
    let command_json =
        serde_json::to_string_pretty(command).expect("JSON serialization of `command` failed");
    let arguments_json = arguments.map_or_else(
        || "null".to_string(),
        |args| {
            serde_json::to_string_pretty(args).expect("JSON serialization of `arguments` failed")
        },
    );
    collect_results(
        test_case,
        TestType::WorkspaceExecuteCommand,
        &mut vec![
            LuaReplacement::Other {
                from: "COMMANDS",
                to: command_str,
            },
            LuaReplacement::ParamDirect {
                name: "command",
                json: command_json,
            },
            LuaReplacement::ParamDirect {
                name: "arguments",
                json: arguments_json,
            },
            LuaReplacement::Other {
                from: "INVOKE_ACTION",
                to: false.to_string(),
            },
        ],
        expected,
        cmp,
    )
}

/// Benchmarks the server's response time to a [`workspace/executeCommand`] request
///
/// - `commands`: A list of LSP command names the client should advertise support for in its
///   capabilities (e.g. "rust-analyzer.runSingle"). This enables command-based `CodeLens`
/// - `command`: The command to be executed. Passed to the client via the request's
///   [`ExecuteCommandParams`]
/// - `arguments`: The arguments to be passed to the command. Passed to the client via
///   the request's [`ExecuteCommandParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `command` or `arguments` fails
///
/// [`workspace/executeCommand`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_executeCommand
pub fn benchmark_workspace_execute_command(
    test_case: &TestCase,
    config: BenchmarkConfig,
    commands: Option<&Vec<String>>,
    command: &str,
    arguments: Option<&Vec<Value>>,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_workspace_execute_command(test_case, commands, command, arguments, None, None)
    })
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
    test_case: &TestCase,
    query: &str,
    cmp: Option<WorkspaceSymbolComparator>,
    expected: Option<&WorkspaceSymbolResponse>,
) -> TestResult<(), WorkspaceSymbolResponse> {
    let query_json =
        serde_json::to_string_pretty(query).expect("JSON serialization of `query` failed");
    collect_results(
        test_case,
        TestType::WorkspaceSymbol,
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

/// Benchmarks the server's response time to a [`workspace/symbol`] request
///
/// - `query`: Passed to the client via the request's [`WorkspaceSymbolParams`]
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `query` fails
///
/// [`workspace/symbol`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_symbol
pub fn benchmark_workspace_symbol(
    test_case: &TestCase,
    config: BenchmarkConfig,
    query: &str,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_workspace_symbol(test_case, query, None, None)
    })
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
    test_case: &TestCase,
    params: &WorkspaceSymbol,
    cmp: Option<WorkspaceSymbolResolveComparator>,
    expected: &WorkspaceSymbol,
) -> TestResult<(), WorkspaceSymbol> {
    let params_json =
        serde_json::to_string_pretty(params).expect("JSON serialization of `params` failed");
    collect_results(
        test_case,
        TestType::WorkspaceSymbolResolve,
        &mut vec![LuaReplacement::ParamDestructure {
            name: "symbol",
            fields: vec!["name", "kind", "tags", "containerName", "location", "data"],
            json: params_json,
        }],
        Some(expected),
        cmp,
    )
}

/// Benchmarks the server's response time to a [`workspaceSymbol/resolve`] request
///
/// - `params`: Passed to the client via the request's [`WorkspaceSymbol`] param
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `params` fails
///
/// [`workspaceSymbole/resolve`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_symbolResolve
pub fn benchmark_workspace_symbol_resolve(
    test_case: &TestCase,
    config: BenchmarkConfig,
    params: &WorkspaceSymbol,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_workspace_symbol_resolve(
            test_case,
            params,
            None,
            &WorkspaceSymbol {
                name: String::new(),
                kind: SymbolKind::FILE,
                tags: None,
                container_name: None,
                location: OneOf::Left(Location {
                    uri: Uri::from_str("").unwrap(),
                    range: Range::default(),
                }),
                data: None,
            },
        )
    })
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
/// [`workspace/willCreateFiles`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_willCreateFiles
#[allow(clippy::result_large_err)]
pub fn test_workspace_will_create_files(
    test_case: &TestCase,
    params: &CreateFilesParams,
    cmp: Option<WorkspaceWillCreateFilesComparator>,
    expected: Option<&WorkspaceEdit>,
) -> TestResult<(), WorkspaceEdit> {
    let params_json =
        serde_json::to_string_pretty(params).expect("JSON serialization of `params` failed");
    collect_results(
        test_case,
        TestType::WorkspaceWillCreateFiles,
        &mut vec![LuaReplacement::ParamDestructure {
            name: "create_params",
            fields: vec!["files"],
            json: params_json,
        }],
        expected,
        cmp,
    )
}

/// Benchmarks the server's response time to a [`workspace/willCreateFiles`] request
///
/// - `params`: Passed to the client via the request's [`CreateFilesParams`] param
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `params` fails
///
/// [`workspace/willCreateFiles`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_willCreateFiles
pub fn benchmark_workspace_will_create_files(
    test_case: &TestCase,
    config: BenchmarkConfig,
    params: &CreateFilesParams,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_workspace_will_create_files(test_case, params, None, None)
    })
}

pub type WorkspaceWillDeleteFilesComparator = fn(&WorkspaceEdit, &WorkspaceEdit, &TestCase) -> bool;

/// Tests the server's response to a [`workspace/willDeleteFiles`] request
///
/// - `params`: Passed to the client via the request's [`DeleteFilesParams`] param
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
/// [`workspace/willDeleteFiles`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_willDeleteFiles
#[allow(clippy::result_large_err)]
pub fn test_workspace_will_delete_files(
    test_case: &TestCase,
    params: &DeleteFilesParams,
    cmp: Option<WorkspaceWillDeleteFilesComparator>,
    expected: Option<&WorkspaceEdit>,
) -> TestResult<(), WorkspaceEdit> {
    let params_json =
        serde_json::to_string_pretty(params).expect("JSON serialization of `params` failed");
    collect_results(
        test_case,
        TestType::WorkspaceWillDeleteFiles,
        &mut vec![LuaReplacement::ParamDestructure {
            name: "delete_params",
            fields: vec!["files"],
            json: params_json,
        }],
        expected,
        cmp,
    )
}

/// Benchmarks the server's response time to a [`workspace/willDeleteFiles`] request
///
/// - `params`: Passed to the client via the request's [`DeleteFilesParams`] param
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `params` fails
///
/// [`workspace/willDeleteFiles`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_willDeleteFiles
pub fn benchmark_workspace_will_delete_files(
    test_case: &TestCase,
    config: BenchmarkConfig,
    params: &DeleteFilesParams,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_workspace_will_delete_files(test_case, params, None, None)
    })
}

pub type WorkspaceWillRenameFilesComparator = fn(&WorkspaceEdit, &WorkspaceEdit, &TestCase) -> bool;

/// Tests the server's response to a [`workspace/willRenameFiles`] request
///
/// - `params`: Passed to the client via the request's [`RenameFilesParams`] param
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
/// [`workspace/willRenameFiles`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_willRenameFiles
#[allow(clippy::result_large_err)]
pub fn test_workspace_will_rename_files(
    test_case: &TestCase,
    params: &RenameFilesParams,
    cmp: Option<WorkspaceWillRenameFilesComparator>,
    expected: Option<&WorkspaceEdit>,
) -> TestResult<(), WorkspaceEdit> {
    let params_json =
        serde_json::to_string_pretty(params).expect("JSON serialization of `params` failed");
    collect_results(
        test_case,
        TestType::WorkspaceWillRenameFiles,
        &mut vec![LuaReplacement::ParamDestructure {
            name: "rename_params",
            fields: vec!["files"],
            json: params_json,
        }],
        expected,
        cmp,
    )
}

/// Benchmarks the server's response time to a [`workspace/willRenameFiles`] request
///
/// - `params`: Passed to the client via the request's [`RenameFilesParams`] param
///
/// # Errors
///
/// Returns [`BenchmarkError`] if the test case is invalid or if benchmarking fails
///
/// # Panics
///
/// Panics if JSON serialization of `params` fails
///
/// [`workspace/willRenameFiles`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_willRenameFiles
pub fn benchmark_workspace_will_rename_files(
    test_case: &TestCase,
    config: BenchmarkConfig,
    params: &RenameFilesParams,
) -> Result<Vec<Duration>, BenchmarkError> {
    benchmark(test_case, config, || {
        test_workspace_will_rename_files(test_case, params, None, None)
    })
}
