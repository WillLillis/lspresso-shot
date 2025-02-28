mod init_dot_lua;
mod test;
pub mod types;

use lsp_types::{
    CompletionResponse, Diagnostic, FormattingOptions, GotoDefinitionResponse, Hover, Location,
    TextEdit, WorkspaceEdit,
};

use std::{
    collections::HashMap,
    fs,
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, Condvar, Mutex, OnceLock},
};

use types::{
    CompletionMismatchError, CompletionResult, DefinitionMismatchError, DiagnosticMismatchError,
    FormattingMismatchError, FormattingResult, HoverMismatchError, ReferencesMismatchError,
    RenameMismatchError, TestCase, TestError, TestResult, TestSetupError, TestType, TimeoutError,
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

fn test_inner<R>(test_case: &TestCase, replacements: Option<&Vec<(&str, String)>>) -> TestResult<R>
where
    R: serde::de::DeserializeOwned,
{
    test_case.validate()?;
    // Invariant: `test_case.test_type` should always be set to `Some(_)` in the caller
    let source_path = test_case.create_test(
        test_case.test_type.expect("Test type is `None`"),
        replacements,
    )?;
    run_test(test_case, &source_path)?;

    let results_file_path = test_case
        .get_results_file_path()
        .map_err(|_| TestError::NoResults)?;
    let raw_results = String::from_utf8(
        fs::read(&results_file_path)
            .map_err(|e| TestError::IO(test_case.test_id.clone(), e.to_string()))?,
    )
    .map_err(|e| TestError::Utf8(test_case.test_id.clone(), e.to_string()))?;
    println!("Raw results: {raw_results}");
    let actual: R = serde_json::from_str(&raw_results).map_err(|e| {
        TestError::Serialization(test_case.test_id.clone(), format!("Results file -- {e}"))
    })?;

    test_case.do_cleanup();

    Ok(actual)
}

/// Invokes neovim to run the test associated with the file stored at `init_dot_lua_path`,
/// opening `source_path`
fn run_test(test_case: &TestCase, source_path: &Path) -> TestResult<()> {
    let init_dot_lua_path = test_case
        .get_init_lua_file_path()
        .map_err(|e| TestError::IO(test_case.test_id.clone(), e.to_string()))?;

    // Restrict the number of tests invoking neovim at a given time to prevent timeout issues
    let (lock, cvar) = &*get_runner_count();
    let _guard = RunnerGuard::new(lock, cvar); // Ensures proper decrement on exit

    let start = std::time::Instant::now();
    let mut child = Command::new("nvim")
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

// TODO: We'll need some separate handling for negative cases, e.g. where it's
// expected for *no* results to be returned. This is tricky because for servers
// with a `$/progress` style startup, we need to basically poll the server for valid
// results until we find something. There's no way (that I can tell) to distinguish
// between an empty "not ready" and a true empty response -- the lua table just looks
// like this: `{ {} }`
// Do we even need to cover this use case?

/// Tests the server's response to a 'textDocument/complection' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
pub fn test_completion(mut test_case: TestCase, expected: &CompletionResult) -> TestResult<()> {
    if test_case.cursor_pos.is_none() {
        Err(TestSetupError::InvalidCursorPosition(TestType::Completion))?;
    }
    test_case.test_type = Some(TestType::Completion);
    let actual: CompletionResponse = test_inner(&test_case, None)?;

    if !expected.results_satisfy(&actual) {
        Err(CompletionMismatchError {
            test_id: test_case.test_id,
            expected: expected.clone(),
            actual,
        })?;
    }

    Ok(())
}

/// Tests the server's response to a 'textDocument/definition' request
///
/// # Errors
///
/// Returns `TestError` if the expected results don't match, or if some other failure occurs
pub fn test_definition(
    mut test_case: TestCase,
    expected: &GotoDefinitionResponse,
) -> TestResult<()> {
    if test_case.cursor_pos.is_none() {
        Err(TestSetupError::InvalidCursorPosition(TestType::Definition))?;
    }
    test_case.test_type = Some(TestType::Definition);
    let actual: GotoDefinitionResponse = test_inner(&test_case, None)?;

    if *expected != actual {
        Err(Box::new(DefinitionMismatchError {
            test_id: test_case.test_id,
            expected: expected.clone(),
            actual,
        }))?;
    }

    Ok(())
}

// NOTE: As far as I can tell, we can't directly accept a `PublishDiagnosticsParams` object,
// since diagnostics are requested via a `textDocument/publishDiagnostics` notification instead
// of a request. The `vim.lsp.buf_notify` method only returns a boolean to indicate success,
// so we can't access the actual data.
/// Tests the server's response to a 'textDocument/publishDiagnostics' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
pub fn test_diagnostics(mut test_case: TestCase, expected: &[Diagnostic]) -> TestResult<()> {
    test_case.test_type = Some(TestType::Diagnostic);
    let actual: Vec<Diagnostic> = test_inner(&test_case, None)?;
    if expected != actual {
        Err(DiagnosticMismatchError {
            test_id: test_case.test_id,
            expected: expected.to_vec(),
            actual,
        })?;
    }

    Ok(())
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
    expected: &FormattingResult,
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

    let actual: FormattingResult = match expected {
        FormattingResult::Response(_) => FormattingResult::Response(test_inner::<Vec<TextEdit>>(
            &test_case,
            Some(&vec![
                ("INVOKE_FORMAT", "false".to_string()),
                ("JSON_OPTIONS", json_opts),
            ]),
        )?),
        FormattingResult::EndState(_) => FormattingResult::EndState(test_inner::<String>(
            &test_case,
            Some(&vec![
                ("INVOKE_FORMAT", "true".to_string()),
                ("JSON_OPTIONS", json_opts),
            ]),
        )?),
    };

    if *expected != actual {
        Err(FormattingMismatchError {
            test_id: test_case.test_id,
            expected: expected.clone(),
            actual,
        })?;
    }

    Ok(())
}

/// Tests the server's response to a 'textDocument/hover' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
pub fn test_hover(mut test_case: TestCase, expected: Hover) -> TestResult<()> {
    if test_case.cursor_pos.is_none() {
        Err(TestSetupError::InvalidCursorPosition(TestType::Hover))?;
    }
    test_case.test_type = Some(TestType::Hover);
    let actual = test_inner(&test_case, None)?;
    // TODO: Might be nice for end users to be able to do this somehow...
    println!("Actual: {actual:#?}");

    if expected != actual {
        Err(Box::new(HoverMismatchError {
            test_id: test_case.test_id,
            expected,
            actual,
        }))?;
    }

    Ok(())
}

/// Tests the server's response to a 'textDocument/references' request
///
/// # Errors
///
/// Returns `TestError` if the expected results don't match, or if some other failure occurs
pub fn test_references(
    mut test_case: TestCase,
    include_declaration: bool,
    expected: &Vec<Location>,
) -> TestResult<()> {
    if test_case.cursor_pos.is_none() {
        Err(TestSetupError::InvalidCursorPosition(TestType::References))?;
    }
    test_case.test_type = Some(TestType::References);
    let actual: Vec<Location> = test_inner(
        &test_case,
        Some(&vec![(
            "SET_CONTEXT",
            format!("context = {{ includeDeclaration = {include_declaration} }}"),
        )]),
    )?;

    if *expected != actual {
        Err(ReferencesMismatchError {
            test_id: test_case.test_id,
            expected: expected.clone(),
            actual,
        })?;
    }

    Ok(())
}

/// Tests the server's response to a 'textDocument/rename' request
///
/// # Errors
///
/// Returns `TestError` if the expected results don't match, or if some other failure occurs
pub fn test_rename(
    mut test_case: TestCase,
    new_name: &str,
    expected: &WorkspaceEdit,
) -> TestResult<()> {
    if test_case.cursor_pos.is_none() {
        Err(TestSetupError::InvalidCursorPosition(TestType::Rename))?;
    }
    test_case.test_type = Some(TestType::Rename);
    let actual: WorkspaceEdit = test_inner(
        &test_case,
        Some(&vec![("NEW_NAME", format!("newName = '{new_name}'"))]),
    )?;

    if *expected != actual {
        Err(Box::new(RenameMismatchError {
            test_id: test_case.test_id,
            expected: expected.clone(),
            actual,
        }))?;
    }

    Ok(())
}
