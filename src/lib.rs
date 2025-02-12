mod init_dot_lua;
mod test;
pub mod types;

use lsp_types::{CompletionResponse, Diagnostic, GotoDefinitionResponse, Hover};
use rand::distr::Distribution;
use std::{
    fs,
    process::{Command, Stdio},
};

use types::{
    CompletionResult, DefinitionMismatchError, DiagnosticMismatchError, HoverMismatchError,
    TestCase, TestError, TestResult, TestSetupError, TestType,
};

/// Intended to be used as a wrapper for `lspresso-shot` testing functions. If the
/// result is `Ok`, the value is returned. If `Err`, pretty prints the error via
/// `panic`
#[macro_export]
macro_rules! lspresso_shot {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(err) => panic!("lspresso-shot test case failed:\n{err}"),
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
pub fn test_hover(mut test_case: TestCase, expected_results: Hover) -> TestResult<()> {
    test_case.validate()?;
    test_case.test_id = get_test_id();
    let test_result = test_hover_inner(&test_case, expected_results);
    let test_dir = test_case.get_lspresso_dir()?;
    if test_case.cleanup && test_dir.exists() {
        fs::remove_dir_all(test_dir)?;
    }

    test_result
}

fn test_hover_inner(test_case: &TestCase, expected: Hover) -> TestResult<()> {
    if test_case.cursor_pos.is_none() {
        Err(TestSetupError::InvalidCursorPosition(
            "Cursor position must be specified for hover tests".to_string(),
        ))?;
    }

    run_test(test_case, TestType::Hover)?;

    let error_path = test_case.get_error_file_path()?;
    if error_path.exists() {
        let error = fs::read_to_string(&error_path)?;
        Err(TestError::Neovim(error))?;
    }

    let results_file_path = test_case.get_results_file_path()?;
    let raw_results = String::from_utf8(fs::read(&results_file_path)?)
        .map_err(|e| TestError::Utf8(e.to_string()))?;
    let actual: Hover = serde_json::from_str(&raw_results)
        .map_err(|e| TestError::Serialization(format!("Results file -- {e}")))?;

    if expected != actual {
        Err(Box::new(HoverMismatchError { expected, actual }))?;
    }
    Ok(())
}

/// Tests the server's response to a 'textDocument/publishDiagnostics' request
pub fn test_diagnostics(
    mut test_case: TestCase,
    expected_results: &[Diagnostic],
) -> TestResult<()> {
    test_case.validate()?;
    test_case.test_id = get_test_id();
    let test_result = test_diagnostics_inner(&test_case, expected_results);
    let test_dir = test_case.get_lspresso_dir()?;
    if test_case.cleanup && test_dir.exists() {
        fs::remove_dir_all(test_dir)?;
    }

    test_result
}

fn test_diagnostics_inner(test_case: &TestCase, expected: &[Diagnostic]) -> TestResult<()> {
    run_test(test_case, TestType::Diagnostic)?;

    let error_path = test_case.get_error_file_path()?;
    if error_path.exists() {
        let error = fs::read_to_string(&error_path)?;
        Err(TestError::Neovim(error))?;
    }

    let results_file_path = test_case.get_results_file_path()?;
    let raw_results = String::from_utf8(fs::read(&results_file_path)?)
        .map_err(|e| TestError::Utf8(e.to_string()))?;
    let actual = serde_json::from_str::<Vec<Diagnostic>>(&raw_results)
        .map_err(|e| TestError::Serialization(e.to_string()))?;

    if expected != actual {
        Err(DiagnosticMismatchError {
            expected: expected.to_vec(),
            actual,
        })?;
    }
    Ok(())
}

/// Tests the server's response to a 'textDocument/publishDiagnostics' request
pub fn test_completions(
    mut test_case: TestCase,
    expected_results: &CompletionResult,
) -> TestResult<()> {
    test_case.validate()?;
    test_case.test_id = get_test_id();
    let test_result = test_completions_inner(&test_case, expected_results);
    let test_dir = test_case.get_lspresso_dir()?;
    if test_case.cleanup && test_dir.exists() {
        fs::remove_dir_all(test_dir)?;
    }

    test_result
}

fn test_completions_inner(test_case: &TestCase, expected: &CompletionResult) -> TestResult<()> {
    if test_case.cursor_pos.is_none() {
        Err(TestSetupError::InvalidCursorPosition(
            "Cursor position must be specified for completion tests".to_string(),
        ))?;
    }
    run_test(test_case, TestType::Completion)?;

    let error_path = test_case.get_error_file_path()?;
    if error_path.exists() {
        let error = fs::read_to_string(&error_path)?;
        Err(TestError::Neovim(error))?;
    }

    let results_file_path = test_case.get_results_file_path()?;
    // temporary struct just to make parsing the results more straightforward
    let raw_results = String::from_utf8(fs::read(&results_file_path)?)
        .map_err(|e| TestError::Utf8(e.to_string()))?;
    let actual = toml::from_str::<CompletionResponse>(&raw_results)
        .map_err(|e| TestError::Serialization(e.to_string()))?;
    _ = actual;
    _ = expected;

    unimplemented!();
    // TODO: Rework completions comparisons
    // if !expected.compare_results(&actual) {
    //     Err(CompletionMismatchError {
    //         expected: expected.clone(),
    //         actual,
    //     })?;
    // }
    // Ok(())
}

/// Tests the server's response to a 'textDocument/definition' request
pub fn test_definition(
    mut test_case: TestCase,
    expected_results: &GotoDefinitionResponse,
) -> TestResult<()> {
    test_case.validate()?;
    test_case.test_id = get_test_id();
    let test_result = test_definition_inner(&test_case, expected_results);
    let test_dir = test_case.get_lspresso_dir()?;
    if test_case.cleanup && test_dir.exists() {
        fs::remove_dir_all(test_dir)?;
    }

    test_result
}

fn test_definition_inner(
    test_case: &TestCase,
    expected: &GotoDefinitionResponse,
) -> TestResult<()> {
    if test_case.cursor_pos.is_none() {
        Err(TestSetupError::InvalidCursorPosition(
            "Cursor position must be specified for definition tests".to_string(),
        ))?;
    }
    run_test(test_case, TestType::Definition)?;

    let error_path = test_case.get_error_file_path()?;
    if error_path.exists() {
        let error = fs::read_to_string(&error_path)?;
        Err(TestError::Neovim(error))?;
    }
    let results_file_path = test_case.get_results_file_path()?;
    let raw_results = String::from_utf8(fs::read(&results_file_path)?)
        .map_err(|e| TestError::Utf8(e.to_string()))?;
    let actual = serde_json::from_str::<GotoDefinitionResponse>(&raw_results)
        .map_err(|e| TestError::Serialization(e.to_string()))?;

    if *expected != actual {
        Err(Box::new(DefinitionMismatchError {
            expected: expected.clone(),
            actual,
        }))?;
    }
    Ok(())
}

/// Generates a new random test ID
fn get_test_id() -> String {
    let range = rand::distr::Uniform::new(0, usize::MAX).unwrap();
    let mut rng = rand::rng();
    range.sample(&mut rng).to_string()
}

/// Invokes Neovim to run the test associated with the file stored at `init_dot_lua_path`,
/// opening `source_path`
fn run_test(test_case: &TestCase, test_type: TestType) -> TestResult<()> {
    let source_path = test_case.create_test(test_type)?;
    let init_dot_lua_path = test_case.get_init_lua_file_path()?;

    let start = std::time::Instant::now();
    let mut child = Command::new("nvim")
        .arg("-u")
        .arg(init_dot_lua_path)
        .arg("--noplugin")
        .arg(source_path)
        .arg("--headless")
        .arg("-n") // disable swap files
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| TestError::Neovim(e.to_string()))?;

    while start.elapsed() < test_case.timeout {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) => {} // still running
            Err(e) => Err(TestError::Neovim(e.to_string()))?,
        }
    }

    Err(TestSetupError::TimeoutExceeded(test_case.timeout))?
}
