mod init_dot_lua;
mod test;
pub mod types;

use lsp_types::{CompletionResponse, Diagnostic, GotoDefinitionResponse, Hover, WorkspaceEdit};
use rand::distr::Distribution;
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

use types::{
    CompletionMismatchError, CompletionResult, DefinitionMismatchError, DiagnosticMismatchError,
    HoverMismatchError, RenameMismatchError, TestCase, TestError, TestResult, TestSetupError,
    TestType, TimeoutError,
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

// TODO: We'll need some separate handling for negative cases, e.g. where it's
// expected for *no* results to be returned. This is tricky because for servers
// with a `$/progress` style startup, we need to basically poll the server for valid
// results until we find something. There's no way (that I can tell) to distinguish
// between an empty "not ready" and a true empty response -- the lua table just looks
// like this: `{ {} }`
// Do we even need to cover this use case?

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
    let actual = test_inner(&mut test_case, None)?;

    if expected != actual {
        Err(Box::new(HoverMismatchError {
            test_id: test_case.test_id,
            expected,
            actual,
        }))?;
    }

    Ok(())
}

// TODO: Accept PublishDiagnosticsParams rather than a raw `Vec<Diagnostic>`, might
// help clean up the lua logic a bit
/// Tests the server's response to a 'textDocument/publishDiagnostics' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
pub fn test_diagnostics(mut test_case: TestCase, expected: &[Diagnostic]) -> TestResult<()> {
    test_case.test_type = Some(TestType::Diagnostic);
    let actual: Vec<Diagnostic> = test_inner(&mut test_case, None)?;
    if expected != actual {
        Err(DiagnosticMismatchError {
            test_id: test_case.test_id.clone(),
            expected: expected.to_vec(),
            actual,
        })?;
    }

    Ok(())
}

/// Tests the server's response to a 'textDocument/publishDiagnostics' request
///
/// # Errors
///
/// Returns `TestError` if the test case is invalid, the expected results don't match,
/// or some other failure occurs
pub fn test_completions(mut test_case: TestCase, expected: &CompletionResult) -> TestResult<()> {
    if test_case.cursor_pos.is_none() {
        Err(TestSetupError::InvalidCursorPosition(TestType::Completion))?;
    }
    test_case.test_type = Some(TestType::Completion);
    let actual: CompletionResponse = test_inner(&mut test_case, None)?;

    if !expected.results_satisfy(&actual) {
        Err(CompletionMismatchError {
            test_id: test_case.test_id.clone(),
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
    let actual: GotoDefinitionResponse = test_inner(&mut test_case, None)?;

    if *expected != actual {
        Err(Box::new(DefinitionMismatchError {
            test_id: test_case.test_id.clone(),
            expected: expected.clone(),
            actual,
        }))?;
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
        &mut test_case,
        Some(&vec![("NEW_NAME", format!("newName = '{new_name}'"))]),
    )?;

    if *expected != actual {
        Err(Box::new(RenameMismatchError {
            test_id: test_case.test_id.clone(),
            expected: expected.clone(),
            actual,
        }))?;
    }

    Ok(())
}

/// Generates a new random test ID
fn generate_test_id() -> String {
    let range = rand::distr::Uniform::new(0, usize::MAX).unwrap();
    let mut rng = rand::rng();
    range.sample(&mut rng).to_string()
}

fn test_inner<R>(
    test_case: &mut TestCase,
    replacements: Option<&Vec<(&str, String)>>,
) -> TestResult<R>
where
    R: serde::de::DeserializeOwned,
{
    test_case.validate()?;
    test_case.test_id = generate_test_id();
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
