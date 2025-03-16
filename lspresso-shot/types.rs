use std::{
    collections::{HashMap, HashSet},
    env::temp_dir,
    fs,
    num::NonZeroU32,
    path::{Path, PathBuf},
    str::FromStr as _,
    time::Duration,
};

use anstyle::{AnsiColor, Color, Style};
use lsp_types::{
    request::{GotoDeclarationResponse, GotoImplementationResponse, GotoTypeDefinitionResponse},
    CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall, CompletionItem,
    CompletionList, CompletionResponse, Diagnostic, DocumentChangeOperation, DocumentChanges,
    DocumentHighlight, DocumentLink, DocumentSymbolResponse, GotoDefinitionResponse, Hover,
    Location, Position, ResourceOp, TextEdit, Uri, WorkspaceEdit,
};
use rand::distr::Distribution as _;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::init_dot_lua::get_init_dot_lua;

/// Specifies the type of test to run
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TestType {
    /// Test `textDocument/completion` requests
    Completion,
    /// Test `textDocument/declaration` requests
    Declaration,
    /// Test `textDocument/definition` requests
    Definition,
    /// Test `textDocument/publishDiagnostics` requests
    Diagnostic,
    /// Test `textDocument/documentHighlight` requests
    DocumentHighlight,
    /// Test `textDocument/documentLink` requests
    DocumentLink,
    /// Test `textDocument/documentSymbol` requests
    DocumentSymbol,
    /// Test `textDocument/formatting` requests
    Formatting,
    /// Test `textDocument/hover` requests
    Hover,
    /// Test `textDocument/implementations` requests
    Implementation,
    /// Test `callHierarchy/incomingCalls` requests
    IncomingCalls,
    /// Test `callHierarchy/outgoingCalls` requests
    OutgoingCalls,
    /// Test `textDocument/prepareCallHierarchy` requests
    PrepareCallHierarchy,
    /// Test `textDocument/references` requests
    References,
    /// Test `textDocument/rename` requests
    Rename,
    /// Test `textDocument/typeDefinition` requests
    TypeDefinition,
}

impl std::fmt::Display for TestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Completion => "textDocument/completion",
                Self::Declaration => "textDocument/declaration",
                Self::Definition => "textDocument/definition",
                Self::Diagnostic => "textDocument/publishDiagnostics",
                Self::DocumentHighlight => "textDocument/documentHighlight",
                Self::DocumentLink => "textDocument/documentLink",
                Self::DocumentSymbol => "textDocument/documentSymbol",
                Self::Formatting => "textDocument/formatting",
                Self::Hover => "textDocument/hover",
                Self::Implementation => "textDocument/implementation",
                Self::IncomingCalls => "callHierarchy/incomingCalls",
                Self::OutgoingCalls => "callHierarchy/outgoingCalls",
                Self::PrepareCallHierarchy => "textDocument/prepareCallHierarchy",
                Self::References => "textDocument/references",
                Self::Rename => "textDocument/rename",
                Self::TypeDefinition => "textDocument/typeDefinition",
            }
        )?;
        Ok(())
    }
}

/// Represents a file to be used in the test case.
#[derive(Debug, Clone)]
pub struct TestFile {
    /// Path to this file relative to the test case source root.
    pub path: PathBuf,
    /// The contents of the source file.
    pub contents: String,
}

impl TestFile {
    pub fn new<P: Into<PathBuf>, T: Into<String>>(path: P, contents: T) -> Self {
        Self {
            path: path.into(),
            contents: contents.into(),
        }
    }
}

/// Describes a test case to be used in an lspresso-shot test.
///
/// - `test_id`: internal identifier for a single run of a test case, *not* to be
///    set by the user.
/// - `test_type`: internal marker for the test type to be run, *not to be set by
///    the user.
/// - `executable_path`: path to the language server's executable.
/// - `nvim_path`: path to/command for the Neovim executable. The default is "nvim".
/// - `source_file`: the source file to be opened by Neovim.
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
    pub test_type: Option<TestType>,
    pub executable_path: PathBuf,
    pub nvim_path: PathBuf,
    pub source_file: TestFile,
    pub cursor_pos: Option<Position>,
    pub other_files: Vec<TestFile>,
    pub start_type: ServerStartType,
    pub timeout: Duration,
    pub cleanup: bool,
}

impl TestCase {
    /// Create a new `TestCase`. `self.nvim_path` is assigned to the contents of `LSPRESSO_NVIM`
    /// if it is set, otherwise "nvim".
    pub fn new<P1: Into<PathBuf>>(executable_path: P1, source_file: TestFile) -> Self {
        let nvim_path = std::env::var("LSPRESSO_NVIM")
            .unwrap_or_else(|_| "nvim".into())
            .into();
        Self {
            test_id: Self::generate_test_id(),
            test_type: None,
            executable_path: executable_path.into(),
            nvim_path,
            source_file,
            cursor_pos: None,
            other_files: Vec::new(),
            start_type: ServerStartType::Simple,
            timeout: Duration::from_secs(1),
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

    /// Change the nvim path used in the test case
    #[must_use]
    pub fn nvim_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.nvim_path = path.into();
        self
    }

    /// Change the source file used in the test case
    #[must_use]
    pub fn source_file(mut self, source_file: TestFile) -> Self {
        self.source_file = source_file;
        self
    }

    /// Add an additional file to the test case
    #[must_use]
    pub fn other_file(mut self, other_file: TestFile) -> Self {
        self.other_files.push(other_file);
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

    /// Generates a new random test ID
    fn generate_test_id() -> String {
        let range = rand::distr::Uniform::new(0, usize::MAX).unwrap();
        let mut rng = rand::rng();
        range.sample(&mut rng).to_string()
    }

    /// Removes the associated test directory if `self.cleanup`. *Intentionally*
    /// ignores any errors, as these should not be surfaced to the user. Error prints
    /// are left to aid in internal development.
    pub fn do_cleanup(&self) {
        let test_dir = match self.get_lspresso_dir() {
            Ok(dir) => dir,
            Err(e) => {
                eprintln!("Test cleanup error (dir fetch): {e}");
                return;
            }
        };
        if self.cleanup && test_dir.exists() {
            if let Err(e) = fs::remove_dir_all(test_dir) {
                eprintln!("Test cleanup error (dir removal): {e}");
            }
        }
    }

    /// Validate the data contained within `self`
    ///
    /// # Errors
    ///
    /// Returns `TestSetupError` if `nvim` isn't executable, the provided server
    /// isn't executable, or if an invalid test file path is found
    pub fn validate(&self) -> TestSetupResult<()> {
        if !is_executable(&self.nvim_path) {
            Err(TestSetupError::InvalidNeovim(self.nvim_path.clone()))?;
        }
        if !is_executable(&self.executable_path) {
            Err(TestSetupError::InvalidServerCommand(
                self.executable_path.clone(),
            ))?;
        }

        self.validate_path(&self.source_file.path)?;
        for TestFile { ref path, .. } in &self.other_files {
            self.validate_path(path)?;
        }

        Ok(())
    }

    /// Validate the user-provided path a test case file
    fn validate_path(&self, input_path: &Path) -> TestSetupResult<()> {
        let test_case_root = self.get_source_file_path("")?;
        let full_path = self.get_source_file_path(input_path)?;
        if full_path.to_string_lossy().is_empty()
            || input_path.is_absolute()
            || !full_path.starts_with(test_case_root)
        {
            Err(TestSetupError::InvalidFilePath(
                input_path.to_string_lossy().to_string(),
            ))?;
        }

        Ok(())
    }

    /// Returns the path to the directory for test `self.test_id`,
    /// creating parent directories along the way
    ///
    /// `/tmp/lspresso-shot/<test_id>/`
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the the test directory can't be created
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
    /// `/tmp/lspresso-shot/<test_id>/results.json`
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the the test directory can't be created
    pub fn get_results_file_path(&self) -> std::io::Result<PathBuf> {
        let mut lspresso_dir = self.get_lspresso_dir()?;
        lspresso_dir.push("results.json");
        Ok(lspresso_dir)
    }

    /// Returns the path to the *empty* result file for test `self.test_id`,
    /// creating parent directories along the way. This file will always be
    /// empty, but its existance marks an empty result resturned by the server.
    ///
    /// `/tmp/lspresso-shot/<test_id>/empty`
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the the test directory can't be created
    pub fn get_empty_file_path(&self) -> std::io::Result<PathBuf> {
        let mut lspresso_dir = self.get_lspresso_dir()?;
        lspresso_dir.push("empty");
        Ok(lspresso_dir)
    }

    /// Returns the path to a source file for test `test_id`,
    /// creating parent directories along the way
    ///
    /// `/tmp/lspresso-shot/<test_id>/src/<file_path>`
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the the test directory can't be created
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
    /// `/tmp/lspresso-shot/<test_id>/init.lua`
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the the test directory can't be created
    pub fn get_init_lua_file_path(&self) -> std::io::Result<PathBuf> {
        let mut lspresso_dir = self.get_lspresso_dir()?;
        lspresso_dir.push("init.lua");
        Ok(lspresso_dir)
    }

    /// Returns the path to the error file for test `test_id`,
    /// creating parent directories along the way. Any non-fatal
    /// errors encounted by the lua code will be recorded here.
    ///
    /// `/tmp/lspresso-shot/<test_id>/error.txt`
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the the test directory can't be created
    pub fn get_error_file_path(&self) -> std::io::Result<PathBuf> {
        let mut lspresso_dir = self.get_lspresso_dir()?;
        lspresso_dir.push("error.txt");
        Ok(lspresso_dir)
    }

    /// Returns the path to the log file for test `test_id`,
    /// creating parent directories along the way. Any logs
    /// created by the lua code will be recorded here.
    ///
    /// `/tmp/lspresso-shot/<test_id>/log.txt`
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the the test directory can't be created
    pub fn get_log_file_path(&self) -> std::io::Result<PathBuf> {
        let mut lspresso_dir = self.get_lspresso_dir()?;
        lspresso_dir.push("log.txt");
        Ok(lspresso_dir)
    }

    /// Creates a test directory for `test_id` based on `self`. Returns the full
    /// path to the source file to be opened.
    ///
    /// # Errors
    ///
    /// Returns `TestSetupError` if the test directory can't be created,
    ///
    /// # Panics
    ///
    /// Will panic if a test source file path doesn't have a parent directory (this
    /// should not be possible)
    pub fn create_test(
        &self,
        test_type: TestType,
        replacements: Option<&Vec<(&str, String)>>,
    ) -> TestSetupResult<PathBuf> {
        {
            let nvim_config = get_init_dot_lua(self, test_type, replacements)?;
            let init_dot_lua_path = self.get_init_lua_file_path()?;
            fs::File::create(&init_dot_lua_path)?;
            fs::write(&init_dot_lua_path, &nvim_config)?;
        }

        let source_path = self.get_source_file_path(&self.source_file.path)?;
        // Invariant: test source file paths should always have a parent directory
        fs::create_dir_all(source_path.parent().unwrap())?;
        fs::File::create(&source_path)?;
        fs::write(&source_path, &self.source_file.contents)?;

        for TestFile { path, contents } in &self.other_files {
            let source_file_path = self.get_source_file_path(path)?;
            // Invariant: test file paths should always have a parent directory
            fs::create_dir_all(source_file_path.parent().unwrap())?;
            fs::File::create(&source_file_path)?;
            fs::write(&source_file_path, contents)?;
        }

        Ok(source_path)
    }
}

/// Check if a path points to an executable file
///
/// # Panics
///
/// Will panic on failure to check to metadata of a path
fn is_executable(server_path: &Path) -> bool {
    let exec_check = |path: &Path| -> bool {
        if path.is_file() {
            #[cfg(unix)]
            {
                // On Unix, check the `x` bit
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

    let path_var = std::env::var_os("PATH").unwrap();
    for path in std::env::split_paths(&path_var) {
        let full_path = path.join(server_path);
        if exec_check(&full_path) {
            return true;
        }
    }

    false
}

/// Indicates how the server initializes itself before it is ready to service
/// requests
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ServerStartType {
    /// The server is ready to serve requests immediately after attaching
    Simple,
    /// The server needs to undergo some indexing-like process reported via `$/progress`
    /// before properly servicing requests. Listen to progress messages and issue
    /// the related request after the ith one is received.
    ///
    /// The inner `NonZeroU32` type indicates on which `end` `$/progress` message the
    /// server is ready to respond to a particular request.
    ///
    /// The inner `String` type contains the text of the relevant progress token
    /// (i.e. "rustAnalyzer/indexing").
    Progress(NonZeroU32, String),
}

pub type TestSetupResult<T> = Result<T, TestSetupError>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TestSetupError {
    #[error("Source file \"{0}\" must have an extension")]
    MissingFileExtension(String),
    #[error("The server command/path \"{}\" is not executable", ._0.display())]
    InvalidServerCommand(PathBuf),
    #[error("The neovim command \"{}\" is not executable", ._0.display())]
    InvalidNeovim(PathBuf),
    #[error("The extension of source file \"{0}\" is invalid")]
    InvalidFileExtension(String),
    #[error("Source file path \"{0}\" is invalid")]
    InvalidFilePath(String),
    #[error("Cursor position must be specified for {0} tests")]
    InvalidCursorPosition(TestType),
    #[error("{0}")]
    IO(String),
}

impl From<std::io::Error> for TestSetupError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value.to_string())
    }
}

pub type TestResult<T> = Result<T, TestError>;

// NOTE: Certain variants' inner types are `Box`ed because they are large
#[derive(Debug, Error, PartialEq)]
pub enum TestError {
    #[error("Test {0}: Expected `None`, got:\n{1}")]
    ExpectedNone(String, String),
    #[error("Test {0}: Expected valid results, got `None`")]
    ExpectedSome(String),
    #[error(transparent)]
    CompletionMismatch(#[from] CompletionMismatchError),
    #[error(transparent)]
    DeclarationMismatch(#[from] Box<DeclarationMismatchError>),
    #[error(transparent)]
    DefinitionMismatch(#[from] Box<DefinitionMismatchError>),
    #[error(transparent)]
    DiagnosticMismatch(#[from] DiagnosticMismatchError),
    #[error(transparent)]
    DocumentHighlightMismatch(#[from] DocumentHighlightMismatchError),
    #[error(transparent)]
    DocumentLinkMismatch(#[from] DocumentLinkMismatchError),
    #[error(transparent)]
    DocumentSymbolMismatch(#[from] DocumentSymbolMismatchError),
    #[error(transparent)]
    FormattingMismatch(#[from] FormattingMismatchError),
    #[error(transparent)]
    HoverMismatch(#[from] Box<HoverMismatchError>),
    #[error(transparent)]
    ImplementationMismatch(#[from] Box<ImplementationMismatchError>),
    #[error(transparent)]
    IncomingCallsMismatch(#[from] IncomingCallsMismatchError),
    #[error(transparent)]
    OutgoingCallsMismatch(#[from] OutgoingCallsMismatchError),
    #[error(transparent)]
    PrepareCallHierarchyMismatch(#[from] PrepareCallHierachyMismatchError),
    #[error(transparent)]
    ReferencesMismatch(#[from] ReferencesMismatchError),
    #[error(transparent)]
    RenameMismatch(#[from] Box<RenameMismatchError>),
    #[error(transparent)]
    TypeDefinitionMismatch(#[from] Box<TypeDefinitionMismatchError>),
    #[error("Test {0}: No results were written")]
    NoResults(String),
    #[error(transparent)]
    Setup(#[from] TestSetupError),
    #[error("Test {0}: Neovim Error\n{1}")]
    Neovim(String, String),
    #[error("Test {0}: IO Error\n{1}")]
    IO(String, String),
    #[error("Test {0}: UTF8 Error\n{1}")]
    Utf8(String, String),
    #[error("Test {0}: Serialization Error\n{1}")]
    Serialization(String, String),
    #[error(transparent)]
    TimeoutExceeded(TimeoutError),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct TimeoutError {
    pub test_id: String,
    pub timeout: Duration,
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Test {}: Test timout of {:.3}s exceeded",
            self.test_id,
            self.timeout.as_secs_f64()
        )?;

        Ok(())
    }
}

const GREEN: Option<Color> = Some(anstyle::Color::Ansi(AnsiColor::Green));
const RED: Option<Color> = Some(anstyle::Color::Ansi(AnsiColor::Red));

fn paint(color: Option<impl Into<Color>>, text: &str) -> String {
    let style = Style::new().fg_color(color.map(Into::into));
    format!("{style}{text}{style:#}")
}

// TODO: Our rendering logic could probably use some cleanup/fxes
fn compare_fields(
    f: &mut std::fmt::Formatter<'_>,
    indent: usize,
    key: &str,
    expected: &serde_json::Value,
    actual: &serde_json::Value,
) -> std::fmt::Result {
    let padding = "  ".repeat(indent);
    let key_render = format!("{key}: ");

    if expected == actual {
        writeln!(
            f,
            "{}",
            paint(GREEN, &format!("{padding}{key_render}{expected}"))
        )?;
    } else {
        // TODO: Pull in some sort of diffing library to make this more readable,
        // as it can be very difficult to spot what's off when comparing long strings
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
}

fn write_fields_comparison<T: Serialize>(
    f: &mut std::fmt::Formatter<'_>,
    name: &str,
    expected: &T,
    actual: &T,
    indent: usize,
) -> std::fmt::Result {
    let mut expected_value = serde_json::to_value(expected).unwrap();
    let mut actual_value = serde_json::to_value(actual).unwrap();
    let padding = "  ".repeat(indent);
    let key_render = if indent == 0 {
        String::new()
    } else {
        format!("{name}: ")
    };

    match expected_value {
        serde_json::Value::Object(ref mut map) => {
            let expected_keys: HashSet<_> = map.keys().map(|k| k.to_owned()).collect();
            map.sort_keys(); // ensure a deterministic ordering
            writeln!(f, "{padding}{key_render}{{",)?;
            for (expected_key, expected_val) in &map.clone() {
                let actual_val = actual_value
                    .get(expected_key)
                    .unwrap_or(&serde_json::Value::Null)
                    .to_owned();
                match expected_val {
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
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
            // Display entries present in the `actual` map but not in the `expected` map
            if let Some(ref mut actual_map) = actual_value.as_object_mut() {
                actual_map.sort_keys(); // ensure a deterministic ordering
                for (actual_key, actual_val) in actual_map
                    .iter()
                    .filter(|(k, _)| !expected_keys.contains(k.as_str()))
                {
                    compare_fields(
                        f,
                        indent + 1,
                        actual_key,
                        &serde_json::Value::Null,
                        actual_val,
                    )?;
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
            // Display entries present in the `actual` array but not in the `expected` array
            for i in array.len()..actual_value.as_array().map_or(0, |a| a.len()) {
                let actual_val = actual_value
                    .get(i)
                    .unwrap_or(&serde_json::Value::Null)
                    .to_owned();
                write_fields_comparison(
                    f,
                    name,
                    &serde_json::Value::Null,
                    &actual_val,
                    indent + 1,
                )?;
            }
            writeln!(f, "{padding}],")?;
        }
        _ => compare_fields(f, indent + 1, name, &expected_value, &actual_value)?,
    }

    Ok(())
}

// `textDocument/completion` is a bit different from other requests. Servers commonly
// send a *bunch* of completion items and rely on the editor's lsp client to filter
// them out/ display the most relevant ones first. This is fine, but it means that
// doing a simple equality check for this isn't realistic and would be a serious
// pain for library consumers. I'd like to experiment with the different ways we
// can handle this, but for now we'll just allow for exact matching, and a simple
// "contains" check.
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionResult {
    /// Expect this exact set of completion items in the provided order
    Exact(CompletionResponse),
    /// Expect to at least see these completion items in any order.
    /// NOTE: This discards the `CompletionList.is_incomplete` field and only
    /// considers `CompletionList.items`
    Contains(Vec<CompletionItem>),
}

impl CompletionResult {
    /// Compares the expected results in `self` to the `actual` results, respecting
    /// the intended behavior for each enum variant of `Self`
    ///
    /// Returns true if the two are considered equal, false otherwise
    #[must_use]
    pub fn results_satisfy(&self, actual: &CompletionResponse) -> bool {
        match self {
            Self::Contains(expected_results) => {
                let actual_items = match actual {
                    CompletionResponse::Array(a) => a,
                    CompletionResponse::List(CompletionList { items, .. }) => items,
                };
                let mut expected = expected_results.clone();
                for item in actual_items {
                    if let Some(i) = expected
                        .iter()
                        .enumerate()
                        .find(|(_, e)| *e == item)
                        .map(|(i, _)| i)
                    {
                        expected.remove(i);
                    };
                }

                expected.is_empty()
            }
            Self::Exact(expected_results) => match (expected_results, actual) {
                (CompletionResponse::Array(expected), CompletionResponse::Array(actual)) => {
                    expected == actual
                }
                (
                    CompletionResponse::List(CompletionList {
                        is_incomplete: expected_is_incomplete,
                        items: expected_items,
                    }),
                    CompletionResponse::List(CompletionList {
                        is_incomplete: actual_is_incomplete,
                        items: actual_items,
                    }),
                ) => {
                    expected_is_incomplete == actual_is_incomplete && expected_items == actual_items
                }
                (CompletionResponse::Array(_), CompletionResponse::List(_))
                | (CompletionResponse::List(_), CompletionResponse::Array(_)) => false,
            },
        }
    }
}

#[derive(Debug, Error, PartialEq)]
pub struct CompletionMismatchError {
    pub test_id: String,
    pub expected: CompletionResult,
    pub actual: CompletionResponse,
}

// TODO: Cleanup/ consolidate this logic with Self::compare_results
impl std::fmt::Display for CompletionMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.expected {
            CompletionResult::Contains(expected_results) => {
                let actual_items = match &self.actual {
                    CompletionResponse::Array(a) => a,
                    CompletionResponse::List(CompletionList { items, .. }) => items,
                };
                let mut expected = expected_results.clone();
                for item in actual_items {
                    if let Some(i) = expected
                        .iter()
                        .enumerate()
                        .find(|(_, e)| **e == *item)
                        .map(|(i, _)| i)
                    {
                        expected.remove(i);
                    };
                }

                writeln!(
                    f,
                    "Unprovided item{}:",
                    if expected.len() > 1 { "s" } else { "" }
                )?;
                for item in &expected {
                    writeln!(
                        f,
                        "{}",
                        paint(RED, &format!("{}", serde_json::to_value(item).unwrap()))
                    )?;
                }
                writeln!(
                    f,
                    "\nProvided item{}:",
                    if actual_items.len() > 1 { "s" } else { "" }
                )?;
                for item in actual_items {
                    writeln!(
                        f,
                        "{}",
                        paint(RED, &format!("{}", serde_json::to_value(item).unwrap()))
                    )?;
                }
            }
            CompletionResult::Exact(expected_results) => match (expected_results, &self.actual) {
                (CompletionResponse::Array(_), CompletionResponse::Array(_))
                | (CompletionResponse::List(_), CompletionResponse::List(_)) => {
                    write_fields_comparison(
                        f,
                        "CompletionResponse",
                        expected_results,
                        &self.actual,
                        0,
                    )?;
                }
                // Different completion types, indicate so and compare the inner items
                (
                    CompletionResponse::Array(expected_items),
                    CompletionResponse::List(CompletionList {
                        items: actual_items,
                        ..
                    }),
                ) => {
                    writeln!(
                        f,
                        "Expected `CompletionResponse::Array`, got `CompletionResponse::List`"
                    )?;
                    write_fields_comparison(
                        f,
                        "CompletionResponse",
                        expected_items,
                        actual_items,
                        0,
                    )?;
                }
                (
                    CompletionResponse::List(CompletionList {
                        items: expected_items,
                        ..
                    }),
                    CompletionResponse::Array(actual_items),
                ) => {
                    writeln!(
                        f,
                        "Expected `CompletionResponse::List`, got `CompletionResponse::Array`"
                    )?;
                    write_fields_comparison(
                        f,
                        "CompletionResponse",
                        expected_items,
                        actual_items,
                        0,
                    )?;
                }
            },
        };

        Ok(())
    }
}

#[derive(Debug, Error, PartialEq)]
pub struct DeclarationMismatchError {
    pub test_id: String,
    pub expected: GotoDeclarationResponse,
    pub actual: GotoDeclarationResponse,
}

impl std::fmt::Display for DeclarationMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect GotoDeclaration response:",
            self.test_id
        )?;
        write_fields_comparison(f, "GotoDeclaration", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq)]
pub struct DefinitionMismatchError {
    pub test_id: String,
    pub expected: GotoDefinitionResponse,
    pub actual: GotoDefinitionResponse,
}

impl std::fmt::Display for DefinitionMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect GotoDefinition response:",
            self.test_id
        )?;
        write_fields_comparison(f, "GotoDefinition", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct DiagnosticMismatchError {
    pub test_id: String,
    pub expected: Vec<Diagnostic>,
    pub actual: Vec<Diagnostic>,
}

impl std::fmt::Display for DiagnosticMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect Diagnostic response:", self.test_id)?;
        write_fields_comparison(f, "Diagnostics", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct DocumentHighlightMismatchError {
    pub test_id: String,
    pub expected: Vec<DocumentHighlight>,
    pub actual: Vec<DocumentHighlight>,
}

impl std::fmt::Display for DocumentHighlightMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Document Highlight response:",
            self.test_id
        )?;
        write_fields_comparison(f, "Document Highlight", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct DocumentLinkMismatchError {
    pub test_id: String,
    pub expected: Vec<DocumentLink>,
    pub actual: Vec<DocumentLink>,
}

impl std::fmt::Display for DocumentLinkMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Document Link response:",
            self.test_id
        )?;
        write_fields_comparison(f, "Document Link", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq)]
pub struct DocumentSymbolMismatchError {
    pub test_id: String,
    pub expected: DocumentSymbolResponse,
    pub actual: DocumentSymbolResponse,
}

impl std::fmt::Display for DocumentSymbolMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Document Symbol response:",
            self.test_id
        )?;
        write_fields_comparison(f, "Document Symbols", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FormattingResult {
    /// Check if the file's formatted state matches the expected contents
    EndState(String),
    /// Check if the server's response matches the exected edits
    Response(Vec<TextEdit>),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct FormattingMismatchError {
    pub test_id: String,
    pub expected: FormattingResult,
    pub actual: FormattingResult,
}

impl std::fmt::Display for FormattingMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect Formatting response:", self.test_id)?;
        match (&self.expected, &self.actual) {
            (
                FormattingResult::Response(expected_edits),
                FormattingResult::Response(actual_edits),
            ) => {
                write_fields_comparison(f, "TextEdit", expected_edits, actual_edits, 0)?;
            }
            (
                FormattingResult::EndState(expected_end_state),
                FormattingResult::EndState(actual_end_state),
            ) => {
                write_fields_comparison(f, "EndState", expected_end_state, actual_end_state, 0)?;
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct HoverMismatchError {
    pub test_id: String,
    pub expected: Hover,
    pub actual: Hover,
}

impl std::fmt::Display for HoverMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect Hover response:", self.test_id)?;
        write_fields_comparison(f, "Hover", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq)]
pub struct ImplementationMismatchError {
    pub test_id: String,
    pub expected: GotoImplementationResponse,
    pub actual: GotoImplementationResponse,
}

impl std::fmt::Display for ImplementationMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Implementation response:",
            self.test_id
        )?;
        write_fields_comparison(f, "Implementation", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq)]
pub struct IncomingCallsMismatchError {
    pub test_id: String,
    pub expected: Vec<CallHierarchyIncomingCall>,
    pub actual: Vec<CallHierarchyIncomingCall>,
}

impl std::fmt::Display for IncomingCallsMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect IncomingCalls response:",
            self.test_id
        )?;
        write_fields_comparison(f, "Implementation", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq)]
pub struct OutgoingCallsMismatchError {
    pub test_id: String,
    pub expected: Vec<CallHierarchyOutgoingCall>,
    pub actual: Vec<CallHierarchyOutgoingCall>,
}

impl std::fmt::Display for OutgoingCallsMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect OutgoingCalls response:",
            self.test_id
        )?;
        write_fields_comparison(f, "Implementation", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq)]
pub struct PrepareCallHierachyMismatchError {
    pub test_id: String,
    pub expected: Vec<CallHierarchyItem>,
    pub actual: Vec<CallHierarchyItem>,
}

impl std::fmt::Display for PrepareCallHierachyMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Prepare Call Hierarchy response:",
            self.test_id
        )?;
        write_fields_comparison(f, "Prepare Call Hierarchy", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct ReferencesMismatchError {
    pub test_id: String,
    pub expected: Vec<Location>,
    pub actual: Vec<Location>,
}

impl std::fmt::Display for ReferencesMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect References response:", self.test_id)?;
        write_fields_comparison(f, "Location", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct RenameMismatchError {
    pub test_id: String,
    pub expected: WorkspaceEdit,
    pub actual: WorkspaceEdit,
}

impl std::fmt::Display for RenameMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect Rename response:", self.test_id)?;
        write_fields_comparison(f, "WorkspaceEdit", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq)]
pub struct TypeDefinitionMismatchError {
    pub test_id: String,
    pub expected: GotoTypeDefinitionResponse,
    pub actual: GotoTypeDefinitionResponse,
}

impl std::fmt::Display for TypeDefinitionMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect GotoTypeDefinition response:",
            self.test_id
        )?;
        write_fields_comparison(f, "GotoTypeDefinition", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}

pub(crate) trait Empty {
    fn is_empty() -> bool {
        false
    }
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct EmptyResult {}

impl Empty for EmptyResult {
    fn is_empty() -> bool {
        true
    }
}

impl Empty for CompletionResponse {}
impl Empty for DocumentSymbolResponse {}
impl Empty for FormattingResult {}
impl Empty for GotoDefinitionResponse {}
impl Empty for Hover {}
impl Empty for String {}
impl Empty for Vec<CallHierarchyItem> {}
impl Empty for Vec<Diagnostic> {}
impl Empty for Vec<DocumentHighlight> {}
impl Empty for Vec<DocumentLink> {}
impl Empty for Vec<CallHierarchyIncomingCall> {}
impl Empty for Vec<CallHierarchyOutgoingCall> {}
impl Empty for Vec<Location> {}
impl Empty for Vec<TextEdit> {}
impl Empty for WorkspaceEdit {}

/// Cleans a given `Uri` object of any information internal to the case
///
/// # Examples
///
/// `file:///tmp/lspresso-shot/<test-id>/src/foo.rs` -> `foo.rs`
fn clean_uri(uri: &Uri, test_case: &TestCase) -> TestResult<Uri> {
    let test_case_root = test_case
        .get_source_file_path("") // "/tmp/lspresso-shot/<test-id>/src/"
        .map_err(|e| TestError::IO(test_case.test_id.clone(), e.to_string()))?
        .to_str()
        .unwrap()
        .to_string();
    let path = uri.path().to_string();
    let cleaned = path.strip_prefix(&test_case_root).unwrap_or(&path);
    Ok(Uri::from_str(cleaned).unwrap())
}

pub(crate) trait CleanResponse
where
    Self: Sized,
{
    /// Cleans a given resonse object of any Uri information related to the test case
    #[allow(unused_variables, unused_mut)]
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        Ok(self)
    }
}

impl CleanResponse for Vec<CallHierarchyItem> {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        for item in &mut self {
            item.uri = clean_uri(&item.uri, test_case)?;
        }
        Ok(self)
    }
}
impl CleanResponse for EmptyResult {}
impl CleanResponse for CompletionResponse {}
impl CleanResponse for DocumentSymbolResponse {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        match &mut self {
            Self::Flat(syms) => {
                for sym in syms {
                    sym.location.uri = clean_uri(&sym.location.uri, test_case)?;
                }
            }
            Self::Nested(_) => {}
        }
        Ok(self)
    }
}
impl CleanResponse for FormattingResult {}
impl CleanResponse for GotoDefinitionResponse {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        match &mut self {
            Self::Scalar(location) => {
                location.uri = clean_uri(&location.uri, test_case)?;
            }
            Self::Array(locs) => {
                for loc in locs {
                    loc.uri = clean_uri(&loc.uri, test_case)?;
                }
            }
            Self::Link(links) => {
                for link in links {
                    link.target_uri = clean_uri(&link.target_uri, test_case)?;
                }
            }
        }
        Ok(self)
    }
}
impl CleanResponse for Hover {}
impl CleanResponse for String {}
impl CleanResponse for Vec<Diagnostic> {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        for diagnostic in &mut self {
            if let Some(info) = diagnostic.related_information.as_mut() {
                for related in info {
                    related.location.uri = clean_uri(&related.location.uri, test_case)?;
                }
            }
        }
        Ok(self)
    }
}
impl CleanResponse for Vec<CallHierarchyIncomingCall> {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        for call in &mut self {
            call.from.uri = clean_uri(&call.from.uri, test_case)?;
        }
        Ok(self)
    }
}
impl CleanResponse for Vec<CallHierarchyOutgoingCall> {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        for call in &mut self {
            call.to.uri = clean_uri(&call.to.uri, test_case)?;
        }
        Ok(self)
    }
}
impl CleanResponse for Vec<DocumentHighlight> {}
impl CleanResponse for Vec<DocumentLink> {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        for link in &mut self {
            if let Some(ref mut uri) = link.target {
                *uri = clean_uri(uri, test_case)?;
            }
        }
        Ok(self)
    }
}
impl CleanResponse for Vec<Location> {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        for loc in &mut self {
            loc.uri = clean_uri(&loc.uri, test_case)?;
        }
        Ok(self)
    }
}
impl CleanResponse for Vec<TextEdit> {}
impl CleanResponse for WorkspaceEdit {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        if let Some(ref mut changes) = self.changes {
            let mut new_changes = HashMap::new();
            for (uri, edits) in changes.drain() {
                let cleaned_uri = clean_uri(&uri, test_case)?;
                new_changes.insert(cleaned_uri, edits);
            }
            *changes = new_changes;
        }
        match self.document_changes {
            Some(DocumentChanges::Edits(ref mut edits)) => {
                for edit in edits {
                    edit.text_document.uri = clean_uri(&edit.text_document.uri, test_case)?;
                }
            }
            Some(DocumentChanges::Operations(ref mut ops)) => {
                for op in ops {
                    match op {
                        DocumentChangeOperation::Op(ref mut op) => match op {
                            ResourceOp::Create(ref mut create) => {
                                create.uri = clean_uri(&create.uri, test_case)?;
                            }
                            ResourceOp::Rename(ref mut rename) => {
                                rename.old_uri = clean_uri(&rename.old_uri, test_case)?;
                                rename.new_uri = clean_uri(&rename.new_uri, test_case)?;
                            }
                            ResourceOp::Delete(ref mut delete) => {
                                delete.uri = clean_uri(&delete.uri, test_case)?;
                            }
                        },
                        DocumentChangeOperation::Edit(edit) => {
                            edit.text_document.uri = clean_uri(&edit.text_document.uri, test_case)?;
                        }
                    }
                }
            }
            None => {}
        }
        Ok(self)
    }
}
