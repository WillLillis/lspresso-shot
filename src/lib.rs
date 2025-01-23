mod init_dot_lua;
pub mod types;

use init_dot_lua::TestType;
use rand::random;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
};

use types::{
    CompletionInfo, CompletionMismatchError, CompletionResult, CursorPosition,
    DefinitionMismatchError, DefinitionResult, DiagnosticMismatchError, DiagnosticResult,
    HoverMismatchError, HoverResult, TestCase, TestError, TestResult, TestSetupError,
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
pub fn test_hover(mut test_case: TestCase, expected_results: HoverResult) -> TestResult<()> {
    test_case.validate()?;
    test_case.test_id = random::<usize>().to_string();
    let test_result = test_hover_inner(&test_case, expected_results);
    let test_dir = test_case.get_lspresso_dir()?;
    if test_case.cleanup && test_dir.exists() {
        fs::remove_dir_all(test_dir)?;
    }

    test_result
}

fn test_hover_inner(test_case: &TestCase, expected: HoverResult) -> TestResult<()> {
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
    let actual: HoverResult =
        toml::from_str(&raw_results).map_err(|e| TestError::Serialization(e.to_string()))?;

    if expected != actual {
        Err(HoverMismatchError { expected, actual })?;
    }
    Ok(())
}

/// Tests the server's response to a 'textDocument/publishDiagnostics' request
pub fn test_diagnostics(
    mut test_case: TestCase,
    expected_results: &DiagnosticResult,
) -> TestResult<()> {
    test_case.validate()?;
    test_case.test_id = random::<usize>().to_string();
    let test_result = test_diagnostics_inner(&test_case, expected_results);
    let test_dir = test_case.get_lspresso_dir()?;
    if test_case.cleanup && test_dir.exists() {
        fs::remove_dir_all(test_dir)?;
    }

    test_result
}

fn test_diagnostics_inner(test_case: &TestCase, expected: &DiagnosticResult) -> TestResult<()> {
    run_test(test_case, TestType::Diagnostic)?;

    let error_path = test_case.get_error_file_path()?;
    if error_path.exists() {
        let error = fs::read_to_string(&error_path)?;
        Err(TestError::Neovim(error))?;
    }

    let results_file_path = test_case.get_results_file_path()?;
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
    mut test_case: TestCase,
    expected_results: &CompletionResult,
) -> TestResult<()> {
    test_case.validate()?;
    test_case.test_id = random::<usize>().to_string();
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
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct CompletionFile {
        pub completions: Vec<CompletionInfo>,
    }
    let raw_results = String::from_utf8(fs::read(&results_file_path)?)
        .map_err(|e| TestError::Utf8(e.to_string()))?;
    let actual: Vec<CompletionInfo> = if raw_results.is_empty() {
        Vec::new()
    } else {
        toml::from_str::<CompletionFile>(&raw_results)
            .map_err(|e| TestError::Serialization(e.to_string()))?
            .completions
    };

    if !expected.compare_results(&actual) {
        Err(CompletionMismatchError {
            expected: expected.clone(),
            actual,
        })?;
    }
    Ok(())
}

/// Tests the server's response to a 'textDocument/definition' request
pub fn test_definition(
    mut test_case: TestCase,
    expected_results: &DefinitionResult,
) -> TestResult<()> {
    test_case.validate()?;
    test_case.test_id = random::<usize>().to_string();
    let test_result = test_definition_inner(&test_case, expected_results);
    let test_dir = test_case.get_lspresso_dir()?;
    if test_case.cleanup && test_dir.exists() {
        fs::remove_dir_all(test_dir)?;
    }

    test_result
}

fn test_definition_inner(test_case: &TestCase, expected: &DefinitionResult) -> TestResult<()> {
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
    // temporary struct just to make parsing the results more straightforward
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct DefinitionFile {
        pub start_line: usize,
        pub start_column: usize,
        pub end_line: Option<usize>,
        pub end_column: Option<usize>,
        pub path: PathBuf,
    }
    let raw_results = String::from_utf8(fs::read(&results_file_path)?)
        .map_err(|e| TestError::Utf8(e.to_string()))?;
    let actual_raw: DefinitionFile = toml::from_str::<DefinitionFile>(&raw_results)
        .map_err(|e| TestError::Serialization(e.to_string()))?;
    let actual = DefinitionResult {
        start_pos: CursorPosition::new(actual_raw.start_line, actual_raw.start_column),
        end_pos: if let (Some(line), Some(col)) = (actual_raw.end_line, actual_raw.end_column) {
            Some(CursorPosition::new(line, col))
        } else {
            None
        },
        path: actual_raw.path,
    };

    if *expected != actual {
        Err(Box::new(DefinitionMismatchError {
            expected: expected.clone(),
            actual,
        }))?;
    }
    Ok(())
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

    Err(TestSetupError::TimeoutExceeded)?
}
