use std::collections::HashSet;
use std::env::temp_dir;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anstyle::{AnsiColor, Color, Style};
use lsp_types::{CompletionResponse, Diagnostic, GotoDefinitionResponse, Hover, Position};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::init_dot_lua::get_init_dot_lua;

/// Specifies the type of test to run
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TestType {
    /// Test `textDocument/hover` requests
    Hover,
    /// Test `textDocument/publishDiagnostics` requests
    Diagnostic,
    /// Test `textDocument/completion` requests
    Completion,
    /// Test `textDocument/definition` requests
    Definition,
}

/// Describes a test case to be used in an lspresso-shot test.
///
/// - `test_id`: internal identifier for a single run of a test case, *not* to be
///    set by the user.
/// - `executable_path`: path to the language server's executable.
/// - `source_path`: gives the test project-relative path for the file to be opened
///    in Neovim.
/// - `source_contents`: the contents of the source file to be opened by Neovim.
/// - `cursor_pos`: the position of the cursor within `source_contents` when the
///    lsp request being tested is executed.
/// - `other_files`: other files to be placed in the mock directory (e.g. other source
///    files, server configuration, etc.).
/// - `start_type`: indicates when the server is ready to service requests
/// - `timeout`: timeout for the test's run in Neovim. The default is 1000ms.
/// - `cleanup`: whether to delete the temporary directory on test completion.
#[derive(Debug, Clone)]
pub struct TestCase {
    pub test_id: String,
    pub executable_path: PathBuf,
    pub source_path: PathBuf,
    pub source_contents: String,
    pub cursor_pos: Option<Position>,
    pub other_files: Vec<(PathBuf, String)>,
    pub start_type: ServerStartType,
    pub timeout: Duration,
    pub cleanup: bool,
}

// TODO: Add some sort of `from_path` method for `TestCase`. Allows a user to point
// to a file or directory and automatically convert it into a `TestCase` instance.
// We need to be *very* careful in the case of directories, as the size could blow
// up easily. Might be smart to set some sort of limit on total capacity and return an
// error if converting a path would exceed it. What should this upper bound be?
// Maybe just add it for files, return Err otherwise

impl TestCase {
    pub fn new<P1: Into<PathBuf>, P2: Into<PathBuf>>(
        source_path: P1,
        executable_path: P2,
        source_contents: &str,
    ) -> Self {
        Self {
            test_id: String::new(),
            executable_path: executable_path.into(),
            source_path: source_path.into(),
            source_contents: source_contents.to_string(),
            cursor_pos: None,
            other_files: Vec::new(),
            start_type: ServerStartType::Simple,
            timeout: Duration::from_millis(2000),
            cleanup: false,
        }
    }

    /// Set the cursor position in the source file
    #[must_use]
    pub const fn cursor_pos(mut self, cursor_pos: Option<Position>) -> Self {
        self.cursor_pos = cursor_pos;
        self
    }

    /// Change the executable path used in the test case
    #[must_use]
    pub fn exeutable_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.executable_path = path.into();
        self
    }

    /// Change the source file used in the test case
    #[must_use]
    pub fn source_file<P: Into<PathBuf>>(mut self, path: P, contents: &str) -> Self {
        self.source_path = path.into();
        self.source_contents = contents.to_string();
        self
    }

    /// Add an additional file to the test case
    #[must_use]
    pub fn other_file<P: Into<PathBuf>>(mut self, path: P, contents: &str) -> Self {
        self.other_files.push((path.into(), contents.to_string()));
        self
    }

    /// Change whether the temporary directory is cleaned up on test completion
    #[must_use]
    pub const fn cleanup(mut self, cleanup: bool) -> Self {
        self.cleanup = cleanup;
        self
    }

    /// Change the expected start type of the server
    #[must_use]
    pub fn start_type(mut self, start_type: ServerStartType) -> Self {
        self.start_type = start_type;
        self
    }

    /// Set the timeout for a test
    #[must_use]
    pub fn timeout<T: Into<Duration>>(mut self, timeout: T) -> Self {
        self.timeout = timeout.into();
        self
    }

    /// Validate the data contained within `self`
    pub fn validate(&self) -> Result<(), TestSetupError> {
        if !is_executable(&PathBuf::from("nvim")) {
            Err(TestSetupError::InvalidNeovim)?;
        }
        if !is_executable(&self.executable_path) {
            Err(TestSetupError::InvalidServerCommand(
                self.executable_path.clone(),
            ))?;
        }
        if self.source_path.to_string_lossy().is_empty() {
            Err(TestSetupError::InvalidFilePath(
                self.source_path.to_string_lossy().to_string(),
            ))?;
        }

        for (path, _) in &self.other_files {
            if path.to_string_lossy().is_empty() {
                Err(TestSetupError::InvalidFilePath(
                    path.to_string_lossy().to_string(),
                ))?;
            }
        }
        Ok(())
    }

    /// Returns the path to the directory for test `self.test_id`,
    /// creating parent directories along the way
    ///
    /// /tmp/lspresso-shot/`test_id`/
    pub fn get_lspresso_dir(&self) -> std::io::Result<PathBuf> {
        let mut tmp_dir = temp_dir();
        tmp_dir.push("lspresso-shot");
        tmp_dir.push(&self.test_id);
        fs::create_dir_all(&tmp_dir)?;
        Ok(tmp_dir)
    }

    /// Returns the path to the result file for test `self.test_id`,
    /// creating parent directories along the way
    ///
    /// /tmp/lspresso-shot/`test_id`/results.json
    pub fn get_results_file_path(&self) -> std::io::Result<PathBuf> {
        let mut lspresso_dir = self.get_lspresso_dir()?;
        fs::create_dir_all(&lspresso_dir)?;
        lspresso_dir.push("results.json");
        Ok(lspresso_dir)
    }

    /// Returns the path to a source file for test `test_id`,
    /// creating parent directories along the way
    ///
    /// /tmp/lspresso-shot/`test_id`/src/`file_path`
    pub fn get_source_file_path<P: AsRef<Path>>(&self, file_path: P) -> std::io::Result<PathBuf> {
        let mut lspresso_dir = self.get_lspresso_dir()?;
        lspresso_dir.push("src");
        fs::create_dir_all(&lspresso_dir)?;
        lspresso_dir.push(file_path);
        Ok(lspresso_dir)
    }

    /// Returns the path to a source file for test `test_id`,
    /// creating parent directories along the way
    ///
    /// /tmp/lspresso-shot/`test_id`/init.lua
    pub fn get_init_lua_file_path(&self) -> std::io::Result<PathBuf> {
        let mut lspresso_dir = self.get_lspresso_dir()?;
        fs::create_dir_all(&lspresso_dir)?;
        lspresso_dir.push("init.lua");
        Ok(lspresso_dir)
    }

    /// Returns the path to the error file for test `test_id`,
    /// creating parent directories along the way. Any errors
    /// encounted by the config's lua code will be recorded here
    ///
    /// /tmp/lspresso-shot/`test_id`/error.txt
    pub fn get_error_file_path(&self) -> std::io::Result<PathBuf> {
        let mut lspresso_dir = self.get_lspresso_dir()?;
        fs::create_dir_all(&lspresso_dir)?;
        lspresso_dir.push("error.txt");
        Ok(lspresso_dir)
    }

    /// Returns the path to the log file for test `test_id`,
    /// creating parent directories along the way. Any logs
    /// created by the config's lua code will be recorded here
    ///
    /// /tmp/lspresso-shot/`test_id`/log.txt
    pub fn get_log_file_path(&self) -> std::io::Result<PathBuf> {
        let mut lspresso_dir = self.get_lspresso_dir()?;
        fs::create_dir_all(&lspresso_dir)?;
        lspresso_dir.push("log.txt");
        Ok(lspresso_dir)
    }

    /// Creates a test directory for `test_id` based on `self`
    pub fn create_test(&self, test_type: TestType) -> TestResult<PathBuf> {
        let results_file_path = self.get_results_file_path()?;
        let init_dot_lua_path = self.get_init_lua_file_path()?;
        let root_path = self.get_lspresso_dir()?;
        let error_path = self.get_error_file_path()?;
        let log_path = self.get_log_file_path()?;
        let extension = self
            .source_path
            .extension()
            .ok_or_else(|| {
                // NOTE: use `.unwrap_or("*")` here instead?
                TestSetupError::MissingFileExtension(self.source_path.to_string_lossy().to_string())
            })?
            .to_str()
            .ok_or_else(|| {
                TestSetupError::InvalidFileExtension(self.source_path.to_string_lossy().to_string())
            })?;

        {
            let nvim_config = get_init_dot_lua(
                self,
                test_type,
                &root_path,
                &results_file_path,
                &error_path,
                &log_path,
                extension,
            );
            fs::File::create(&init_dot_lua_path)?;
            fs::write(&init_dot_lua_path, &nvim_config)?;
        }

        let source_path = self.get_source_file_path(&self.source_path)?;
        // Source file paths should always have a parent directory
        fs::create_dir_all(source_path.parent().unwrap())?;
        fs::File::create(&source_path)?;
        fs::write(&source_path, &self.source_contents)?;

        for (path, contents) in &self.other_files {
            let source_file_path = self.get_source_file_path(path)?;
            // Invariant: test source file paths should always have a parent directory
            fs::create_dir_all(source_file_path.parent().unwrap())?;
            fs::File::create(&source_file_path)?;
            fs::write(&source_file_path, contents)?;
        }
        Ok(source_path)
    }
}

/// Check if a path points to an executable file
fn is_executable(server_path: &Path) -> bool {
    let exec_check = |path: &Path| -> bool {
        if path.is_file() {
            #[cfg(unix)]
            {
                // On Unix, check the `x` bit
                use std::fs;
                use std::os::unix::fs::PermissionsExt;
                let metadata = fs::metadata(path).unwrap();
                metadata.permissions().mode() & 0o111 != 0
            }
            #[cfg(windows)]
            {
                // On Windows, check for common executable extensions
                let extensions = ["exe", "cmd", "bat", "com"];
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    return extensions.contains(&ext);
                }
                false
            }
        } else {
            #[cfg(windows)]
            {
                // On Windows, it's valid to omit file extensions, i.e. `gcc` can be
                // used to designate `gcc.exe`. However, this will cause `.is_file()`
                // to return `false`, so we need to check for this case here rather
                // than above
                let extensions = ["exe", "cmd", "bat", "com"];
                for ext in &extensions {
                    let Some(path) = path.to_str() else {
                        continue;
                    };
                    let ext_path = PathBuf::from(format!("{path}.{ext}"));
                    if ext_path.exists() && ext_path.is_file() {
                        return true;
                    }
                }
            }
            false
        }
    };

    if exec_check(server_path) {
        return true;
    }

    use std::env;
    let path_var = env::var_os("PATH").unwrap();
    for path in env::split_paths(&path_var) {
        let full_path = path.join(server_path);
        if exec_check(&full_path) {
            return true;
        }
    }

    false
}

// TODO: Need to find a good way to test `Simple` server setup. rust-analyzer doesn't
// support this obviously, so we can't use that. Expecting contributors to have
// asm-lsp or some other simple non-`$/progress` server installed isn't great, but maybe
// that's the only way to do it...
/// Indicates how the server initializes itself before it is ready to service
/// requests
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ServerStartType {
    /// The server is ready to serve requests immediately after attaching
    Simple,
    /// The server needs to undergo some indexing-like process reported via `$/progress`
    /// before servicing requests. The inner `String` type contains the text of the relevant
    /// progress token
    Progress(String),
}

#[derive(Debug, Error)]
pub enum TestSetupError {
    #[error("Source file \"{0}\" must have an extension")]
    MissingFileExtension(String),
    #[error("The server command \"{}\" is not executable", ._0.display())]
    InvalidServerCommand(PathBuf),
    #[error("The command \"nvim\" is not executable")]
    InvalidNeovim,
    #[error("The extension of source file \"{0}\" is invalid")]
    InvalidFileExtension(String),
    #[error("Source file path \"{0}\" is invalid")]
    InvalidFilePath(String),
    #[error("{0}")]
    InvalidCursorPosition(String),
    #[error("Test timout of {:.3}s exceeded", ._0.as_secs_f64())]
    TimeoutExceeded(Duration),
}

impl From<std::io::Error> for TestError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value.to_string())
    }
}

pub type TestResult<T> = Result<T, TestError>;

#[derive(Debug, Error)]
pub enum TestError {
    #[error(transparent)]
    HoverMismatch(#[from] Box<HoverMismatchError>), // NOTE: `Box`ed because large
    #[error(transparent)]
    DiagnosticMismatch(#[from] DiagnosticMismatchError),
    #[error(transparent)]
    CompletionMismatch(#[from] CompletionMismatchError),
    #[error(transparent)]
    DefinitionMismatch(#[from] Box<DefinitionMismatchError>), // NOTE: `Box`ed because large
    #[error(transparent)]
    Setup(#[from] TestSetupError),
    #[error("{0}")]
    Neovim(String),
    #[error("{0}")]
    IO(String),
    #[error("{0}")]
    Utf8(String),
    #[error("{0}")]
    Serialization(String),
}

const GREEN: Option<Color> = Some(anstyle::Color::Ansi(AnsiColor::Green));
const RED: Option<Color> = Some(anstyle::Color::Ansi(AnsiColor::Red));

fn paint(color: Option<impl Into<Color>>, text: &str) -> String {
    let style = Style::new().fg_color(color.map(Into::into));
    format!("{style}{text}{style:#}")
}

fn write_fields_comparison<T: Serialize>(
    f: &mut std::fmt::Formatter<'_>,
    name: &str,
    expected: &T,
    actual: &T,
    indent: usize,
) -> std::fmt::Result {
    let compare_fields = |f: &mut std::fmt::Formatter<'_>,
                          indent: usize,
                          key: &str,
                          expected: &serde_json::Value,
                          actual: &serde_json::Value| {
        let padding = "  ".repeat(indent);
        let key_render = format!("{key}: ");

        if expected == actual {
            writeln!(
                f,
                "{}",
                paint(GREEN, &format!("{padding}{key_render}{expected}"))
            )?;
        } else {
            let expected_render = if expected.is_string() {
                format!("\n{padding}    {expected}")
            } else {
                format!(" {expected}")
            };
            let actual_render = if actual.is_string() {
                format!("\n{padding}    {actual}")
            } else {
                format!(" {actual}")
            };
            writeln!(
                f,
                "{}",
                paint(
                    RED,
                    &format!("{padding}{key_render}\n{padding}  Expected:{expected_render}\n{padding}  Actual:{actual_render}")
                )
            )?;
        }

        std::fmt::Result::Ok(())
    };
    let mut expected_value = serde_json::to_value(expected).unwrap();
    let actual_value = serde_json::to_value(actual).unwrap();
    let padding = "  ".repeat(indent);
    let key_render = if indent == 0 {
        String::new()
    } else {
        format!("{name}: ")
    };

    match expected_value {
        serde_json::Value::Object(ref mut map) => {
            map.sort_keys(); // ensure a deterministic ordering
            writeln!(f, "{padding}{key_render}{{",)?;
            for (expected_key, expected_val) in map {
                let actual_val = actual_value.get(expected_key).unwrap().to_owned();
                match expected_val {
                    serde_json::Value::Object(_) => {
                        write_fields_comparison(
                            f,
                            expected_key,
                            expected_val,
                            &actual_val,
                            indent + 1,
                        )?;
                    }
                    _ => {
                        compare_fields(f, indent + 1, expected_key, expected_val, &actual_val)?;
                    }
                }
            }
            writeln!(f, "{padding}}},")?;
        }
        serde_json::Value::Array(ref array) => {
            writeln!(f, "{padding}{key_render}[")?;
            for (i, expected_val) in array.iter().enumerate() {
                let actual_val = actual_value
                    .get(i)
                    .unwrap_or(&serde_json::Value::Null)
                    .to_owned();
                write_fields_comparison(f, name, expected_val, &actual_val, indent + 1)?;
            }
            writeln!(f, "{padding}],")?;
        }
        _ => compare_fields(f, indent + 1, name, &expected_value, &actual_value)?,
    }

    Ok(())
}

#[derive(Debug, Error)]
pub struct HoverMismatchError {
    pub expected: Hover,
    pub actual: Hover,
}

impl std::fmt::Display for HoverMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write_fields_comparison(f, "Hover", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub struct DiagnosticMismatchError {
    pub expected: Vec<Diagnostic>,
    pub actual: Vec<Diagnostic>,
}

impl std::fmt::Display for DiagnosticMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write_fields_comparison(f, "Diagnostic", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}

// Completion Types
#[derive(Debug, Clone)]
pub enum CompletionResult {
    /// Expect less than this many completion results, ignoring their contents
    LessThan(usize),
    /// Expect exactly this many completion results, ignoring their contents
    Exactly(usize),
    /// Expect more than  this many completion results, ignoring their contents
    MoreThan(usize),
    /// Expect this exact set of completion results, ignoring their order
    Contains(HashSet<CompletionResponse>),
    /// Expect this exact set of completion results in a specific order
    Exact(Vec<CompletionResponse>),
}

// impl CompletionResult {
//     /// Compares the expected results in `self` to the `actual` results, respecting
//     /// the intended behavior for each enum variant of `Self`
//     ///
//     /// Returns true if the two are considered equal, false otherwise
//     #[must_use]
//     pub fn compare_results(&self, actual: &CompletionResponse) -> bool {
//         match self {
//             Self::LessThan(n_expected) => *n_expected > actual.len(),
//             Self::Exactly(n_expected) => *n_expected == actual.len(),
//             Self::MoreThan(n_expected) => *n_expected < actual.len(),
//             Self::Contains(expected) => {
//                 let mut remaining = expected.clone();
//                 for result in actual {
//                     if remaining.is_empty() {
//                         return false;
//                     }
//                     if remaining.contains(result) {
//                         remaining.remove(result);
//                     }
//                 }
//                 remaining.is_empty()
//             }
//             Self::Exact(results) => {
//                 if results.len() != actual.len() {
//                     return false;
//                 }
//
//                 for (expected, actual) in results.iter().zip(actual.iter()) {
//                     if expected != actual {
//                         return false;
//                     }
//                 }
//                 true
//             }
//         }
//     }
// }

#[derive(Debug, Error)]
pub struct CompletionMismatchError {
    pub expected: CompletionResult,
    pub actual: CompletionResponse,
}

impl std::fmt::Display for CompletionMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        _ = f;
        // TODO: Rework the different Eq. kinds
        Ok(())
    }
}

#[derive(Debug, Error)]
pub struct DefinitionMismatchError {
    pub expected: GotoDefinitionResponse,
    pub actual: GotoDefinitionResponse,
}

impl std::fmt::Display for DefinitionMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write_fields_comparison(f, "GotoDefinition", &self.expected, &self.actual, 0)?;

        Ok(())
    }
}
