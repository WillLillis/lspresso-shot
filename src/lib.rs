mod init_dot_lua;
pub mod types;

use init_dot_lua::InitType;
use rand::random;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

use types::{
    CompletionInfo, CompletionMismatchError, CompletionResult, DiagnosticMismatchError,
    DiagnosticResult, HoverMismatchError, HoverResult, TestCase, TestError, TestResult,
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

/// Tests the server's response to a 'textDocument/hover' request
pub fn test_hover(
    test_case: &TestCase,
    expected_results: HoverResult,
    executable_path: &Path,
) -> TestResult<()> {
    test_case.validate()?;
    let test_id = random::<usize>().to_string();
    let test_result = test_hover_inner(test_case, expected_results, executable_path, &test_id);
    let test_dir = TestCase::get_lspresso_dir(&test_id)?;
    if test_case.cleanup && test_dir.exists() {
        fs::remove_dir_all(test_dir)?;
    }

    test_result
}

fn test_hover_inner(
    test_case: &TestCase,
    expected: HoverResult,
    executable_path: &Path,
    test_id: &str,
) -> TestResult<()> {
    let source_path = test_case.create_test(test_id, executable_path, InitType::Hover)?;
    let init_dot_lua_path = TestCase::get_init_lua_file_path(test_id)?;

    run_test(&init_dot_lua_path, &source_path)?;

    let error_path = TestCase::get_error_file_path(test_id)?;
    if error_path.exists() {
        let error = fs::read_to_string(&error_path)?;
        Err(TestError::Neovim(error))?;
    }

    let results_file_path = TestCase::get_results_file_path(test_id)?;
    let raw_results = String::from_utf8(fs::read(&results_file_path)?)
        .map_err(|e| TestError::Utf8(e.to_string()))?;
    let actual: HoverResult =
        toml::from_str(&raw_results).map_err(|e| TestError::Serialization(e.to_string()))?;

    if expected != actual {
        Err(HoverMismatchError { expected, actual })?;
    }
    Ok(())
}

/// Tests the server's response to a 'textDocument/publishDiagnostics' request
pub fn test_diagnostics(
    test_case: &TestCase,
    expected_results: &DiagnosticResult,
    executable_path: &Path,
) -> TestResult<()> {
    test_case.validate()?;
    let test_id = random::<usize>().to_string();
    let test_result =
        test_diagnostics_inner(test_case, expected_results, executable_path, &test_id);
    let test_dir = TestCase::get_lspresso_dir(&test_id)?;
    if test_case.cleanup && test_dir.exists() {
        fs::remove_dir_all(test_dir)?;
    }

    test_result
}

fn test_diagnostics_inner(
    test_case: &TestCase,
    expected: &DiagnosticResult,
    executable_path: &Path,
    test_id: &str,
) -> TestResult<()> {
    let source_path = test_case.create_test(test_id, executable_path, InitType::Diagnostic)?;
    let init_dot_lua_path = TestCase::get_init_lua_file_path(test_id)?;

    run_test(&init_dot_lua_path, &source_path)?;

    let error_path = TestCase::get_error_file_path(test_id)?;
    if error_path.exists() {
        let error = fs::read_to_string(&error_path)?;
        Err(TestError::Neovim(error))?;
    }

    let results_file_path = TestCase::get_results_file_path(test_id)?;
    let raw_results = String::from_utf8(fs::read(&results_file_path)?)
        .map_err(|e| TestError::Utf8(e.to_string()))?;
    let actual: DiagnosticResult =
        toml::from_str(&raw_results).map_err(|e| TestError::Serialization(e.to_string()))?;

    if *expected != actual {
        Err(DiagnosticMismatchError {
            expected: expected.clone(),
            actual,
        })?;
    }
    Ok(())
}

/// Tests the server's response to a 'textDocument/publishDiagnostics' request
pub fn test_completions(
    test_case: &TestCase,
    expected_results: &CompletionResult,
    executable_path: &Path,
) -> TestResult<()> {
    test_case.validate()?;
    let test_id = random::<usize>().to_string();
    let test_result =
        test_completions_inner(test_case, expected_results, executable_path, &test_id);
    let test_dir = TestCase::get_lspresso_dir(&test_id)?;
    if test_case.cleanup && test_dir.exists() {
        fs::remove_dir_all(test_dir)?;
    }

    test_result
}

fn test_completions_inner(
    test_case: &TestCase,
    expected: &CompletionResult,
    executable_path: &Path,
    test_id: &str,
) -> TestResult<()> {
    let source_path = test_case.create_test(test_id, executable_path, InitType::Completion)?;
    let init_dot_lua_path = TestCase::get_init_lua_file_path(test_id)?;

    run_test(&init_dot_lua_path, &source_path)?;

    let error_path = TestCase::get_error_file_path(test_id)?;
    if error_path.exists() {
        let error = fs::read_to_string(&error_path)?;
        Err(TestError::Neovim(error))?;
    }

    let results_file_path = TestCase::get_results_file_path(test_id)?;
    // little anon struct to make parsing the results more straightforward
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct CompletionFile {
        pub completions: Vec<CompletionInfo>,
    }
    let raw_results = String::from_utf8(fs::read(&results_file_path)?)
        .map_err(|e| TestError::Utf8(e.to_string()))?;
    let actual: Vec<CompletionInfo> = toml::from_str::<CompletionFile>(&raw_results)
        .map_err(|e| TestError::Serialization(e.to_string()))?
        .completions;

    if !expected.compare_results(&actual) {
        Err(CompletionMismatchError {
            expected: expected.clone(),
            actual,
        })?;
    }
    Ok(())
}

/// Invokes Neovim to run the test associated with the file stored at `init_dot_lua_path`,
/// opening `source_path`
fn run_test(init_dot_lua_path: &Path, source_path: &Path) -> TestResult<()> {
    Command::new("nvim")
        .arg("-u")
        .arg(init_dot_lua_path)
        .arg("--noplugin")
        .arg(source_path)
        .arg("--headless")
        .arg("-n") // disable swap files
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| TestError::Neovim(e.to_string()))?
        .wait()
        .map_err(|e| TestError::Neovim(e.to_string()))?;

    Ok(())
}
