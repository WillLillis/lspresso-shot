mod init_dot_lua;
pub mod types;

use init_dot_lua::{get_init_dot_lua, InitType};
use rand::random;
use std::{
    env::temp_dir,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::Result;
use types::{
    HoverMismatchError, HoverResult, HoverTestError, HoverTestResult, TestCase, TestSetupError,
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

/// Returns the path to the directory for test `test_id`,
/// creating parent directories along the way
///
/// /tmp/lspresso-shot/<test_id>/
fn get_lspresso_dir(test_id: &str) -> PathBuf {
    let mut tmp_dir = temp_dir();
    tmp_dir.push("lspresso-shot");
    tmp_dir.push(test_id);
    fs::create_dir_all(&tmp_dir).unwrap();
    tmp_dir
}

/// Returns the path to the result file for test `test_id`,
/// creating parent directories along the way
///
/// /tmp/lspresso-shot/<test_id>/results.toml
fn get_results_file_path(test_id: &str) -> PathBuf {
    let mut lspresso_dir = get_lspresso_dir(test_id);
    fs::create_dir_all(&lspresso_dir).unwrap();
    lspresso_dir.push("results.toml");
    lspresso_dir
}

/// Returns the path to a source file for test `test_id`,
/// creating parent directories along the way
///
/// /tmp/lspresso-shot/<test_id>/src/<file_path>
fn get_source_file_path(test_id: &str, file_path: &Path) -> PathBuf {
    let mut lspresso_dir = get_lspresso_dir(test_id);
    lspresso_dir.push("src");
    fs::create_dir_all(&lspresso_dir).unwrap();
    lspresso_dir.push(file_path);
    lspresso_dir
}

/// Returns the path to a source file for test `test_id`,
/// creating parent directories along the way
///
/// /tmp/lspresso-shot/<test_id>/init.lua
fn get_init_lua_file_path(test_id: &str) -> PathBuf {
    let mut lspresso_dir = get_lspresso_dir(test_id);
    fs::create_dir_all(&lspresso_dir).unwrap();
    lspresso_dir.push("init.lua");
    lspresso_dir
}

// TODO: Finish this...Either make a blind guess based on the project's directory
// name, or instead try to find it by manually stepping the members of target/debug
// Provide debug and release variants or a parameter?
/// Returns the path to the current project's debug executable
pub fn executable_path() -> Result<PathBuf> {
    let mut path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    path.push("target");
    path.push("debug");
    Ok(path)
}

/// Tests the server's response to a 'textDocument/hover' request
pub fn test_hover(
    test_case: &TestCase,
    expected_results: HoverResult,
    executable_path: &Path,
) -> HoverTestResult<()> {
    test_case.validate()?;
    let test_id = random::<usize>().to_string();
    let test_result = test_hover_inner(test_case, expected_results, executable_path, &test_id);
    let test_dir = get_lspresso_dir(&test_id);
    if test_case.cleanup && test_dir.exists() {
        fs::remove_dir_all(test_dir)?;
    }

    test_result
}

pub fn test_hover_inner(
    test_case: &TestCase,
    expected: HoverResult,
    executable_path: &Path,
    test_id: &str,
) -> HoverTestResult<()> {
    // TODO: This setup code can probably be pulled into a helper to clean things up
    // Let's wait and see what common functionality appears in the other test functions
    if test_case.cursor_pos.line == 0 {
        Err(TestSetupError::InvalidCursorPosition)?;
    }
    let results_file_path = get_results_file_path(test_id);
    let init_dot_lua_path = get_init_lua_file_path(test_id);
    let root_path = get_lspresso_dir(test_id);
    let extension = test_case
        .source_path
        .extension()
        .ok_or_else(|| {
            TestSetupError::MissingFileExtension(
                test_case.source_path.to_string_lossy().to_string(),
            )
        })?
        .to_str()
        .ok_or_else(|| {
            TestSetupError::InvalidFileExtension(
                test_case.source_path.to_string_lossy().to_string(),
            )
        })?;

    {
        let nvim_config = get_init_dot_lua(
            InitType::Hover,
            &root_path,
            &results_file_path,
            executable_path,
            extension,
        )
        // TODO: Cursor position should probably be optional for other tests,
        // figure that out...
        .replace("CURSOR_LINE", &test_case.cursor_pos.line.to_string())
        .replace("CURSOR_COLUMN", &test_case.cursor_pos.column.to_string());
        fs::File::create(&init_dot_lua_path)?;
        fs::write(&init_dot_lua_path, &nvim_config)?;
    }

    let source_path = get_source_file_path(test_id, &test_case.source_path);
    // Source file paths should always have a parent directory
    fs::create_dir_all(source_path.parent().unwrap())?;
    fs::File::create(&source_path)?;
    fs::write(&source_path, &test_case.source_contents)?;

    for (path, contents) in &test_case.other_files {
        let source_file_path = get_source_file_path(test_id, path);
        // Source file paths should always have a parent directory
        fs::create_dir_all(source_file_path.parent().unwrap()).unwrap();
        fs::File::create(&source_file_path)?;
        fs::write(&source_file_path, contents)?;
    }

    Command::new("nvim")
        .arg("-u")
        .arg(init_dot_lua_path)
        .arg("--noplugin")
        .arg(&source_path)
        .arg("--headless")
        .arg("-n") // disable swap files
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| HoverTestError::Neovim(e.to_string()))?
        .wait()
        .map_err(|e| HoverTestError::Neovim(e.to_string()))?;

    let raw_results = String::from_utf8(fs::read(&results_file_path)?)
        .map_err(|e| HoverTestError::Utf8(e.to_string()))?;
    let actual: HoverResult =
        toml::from_str(&raw_results).map_err(|e| HoverTestError::Serialization(e.to_string()))?;

    if expected != actual {
        Err(HoverMismatchError { expected, actual })?;
    }
    Ok(())
}
