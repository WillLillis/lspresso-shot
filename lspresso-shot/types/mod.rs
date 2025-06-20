pub mod call_hierarchy;
pub mod code_action;
pub mod code_lens;
pub mod color_presentation;
pub(crate) mod compare;
pub mod completion;
pub mod definition;
pub mod diagnostic;
pub mod document_color;
pub mod document_highlight;
pub mod document_link;
pub mod document_symbol;
pub mod folding_range;
pub mod formatting;
pub mod hover;
pub mod inlay_hint;
pub mod linked_editing_range;
pub mod moniker;
pub mod references;
pub mod rename;
pub mod selection_range;
pub mod semantic_tokens;
pub mod signature_help;
pub mod type_hierarchy;
pub mod workspace_symbol;

use crate::init_dot_lua::{LuaReplacement, get_init_dot_lua};

use std::{
    env::temp_dir,
    fs,
    num::NonZeroU32,
    path::{Path, PathBuf},
    str::FromStr as _,
    time::Duration,
};

use compare::write_fields_comparison;
use lsp_types::{LSPAny, Position, Uri};
use rand::distr::Distribution as _;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Specifies the type of test to run
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TestType {
    /// Test `textDocument/codeAction` requests
    CodeAction,
    /// Test `codeAction/resolve` requests
    CodeActionResolve,
    /// Test `textDocument/codeLens` requests
    CodeLens,
    /// Test `codeLens/resolve` requests
    CodeLensResolve,
    /// Test `textDocument/colorPresentation` requests
    ColorPresentation,
    /// Test `textDocument/completion` requests
    Completion,
    /// Test `completionItem/resolve` requests
    CompletionResolve,
    /// Test `textDocument/declaration` requests
    Declaration,
    /// Test `textDocument/definition` requests
    Definition,
    /// Test 'textDocument/diagnostic' requests
    Diagnostic,
    /// Test `textDocument/documentColor` requests
    DocumentColor,
    /// Test `textDocument/documentHighlight` requests
    DocumentHighlight,
    /// Test `textDocument/documentLink` requests
    DocumentLink,
    /// Test `documentLink/resolve` requests
    DocumentLinkResolve,
    /// Test `textDocument/documentSymbol` requests
    DocumentSymbol,
    /// Test `textDocument/foldingRange` requests
    FoldingRange,
    /// Test `textDocument/formatting` requests
    Formatting,
    /// Test `textDocument/hover` requests
    Hover,
    /// Test `textDocument/implementations` requests
    Implementation,
    /// Test `callHierarchy/incomingCalls` requests
    IncomingCalls,
    /// Test `textDocument/inlayHint` requests
    InlayHint,
    /// Test `textDocument/linkedEditingRange` requests
    LinkedEditingRange,
    /// Test `textDocument/moniker` requests
    Moniker,
    /// Test `textDocument/onTypeFormatting` requests
    OnTypeFormatting,
    /// Test `callHierarchy/outgoingCalls` requests
    OutgoingCalls,
    /// Test `textDocument/prepareCallHierarchy` requests
    PrepareCallHierarchy,
    /// Test `textDocument/prepareRename` requests
    PrepareRename,
    /// Test `textDocument/prepareTypeHierarchy` requests
    PrepareTypeHierarchy,
    /// Test `textDocument/publishDiagnostics` requests
    PublishDiagnostics,
    /// Test `textDocument/rangeFormatting` requests
    RangeFormatting,
    /// Test `textDocument/references` requests
    References,
    /// Test `textDocument/rename` requests
    Rename,
    /// Test `textDocument/selectionRange` requests
    SelectionRange,
    /// Test `textDocument/semanticTokens/full` requests
    SemanticTokensFull,
    /// Test `textDocument/semanticTokens/full/delta` requests
    SemanticTokensFullDelta,
    /// Test `textDocument/semanticTokens/range` requests
    SemanticTokensRange,
    /// Test `textDocument/signatureHelp` requests
    SignatureHelp,
    /// Test `textDocument/typeDefinition` requests
    TypeDefinition,
    /// Test `workspace/diagnostic` requests
    WorkspaceDiagnostic,
    /// Test `workspace/executeCommand` requests
    WorkspaceExecuteCommand,
    /// Test `workspace/symbol` requests
    WorkspaceSymbol,
    /// Test `workspaceSymbol/resolve` requests
    WorkspaceSymbolResolve,
    /// Test `workspace/willCreateFiles` requests
    WorkspaceWillCreateFiles,
    /// Test `workspace/willDeleteFiles` requests
    WorkspaceWillDeleteFiles,
    /// Test `workspace/willRenameFiles` requests
    WorkspaceWillRenameFiles,
}

impl std::fmt::Display for TestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::CodeAction => "textDocument/codeAction",
                Self::CodeActionResolve => "codeAction/resolve",
                Self::CodeLens => "textDocument/codeLens",
                Self::CodeLensResolve => "codeLens/resolve",
                Self::ColorPresentation => "textDocument/colorPresentation",
                Self::Completion => "textDocument/completion",
                Self::CompletionResolve => "completionItem/resolve",
                Self::Declaration => "textDocument/declaration",
                Self::Definition => "textDocument/definition",
                Self::Diagnostic => "textDocument/diagnostic",
                Self::DocumentColor => "textDocument/documentColor",
                Self::DocumentHighlight => "textDocument/documentHighlight",
                Self::DocumentLink => "textDocument/documentLink",
                Self::DocumentLinkResolve => "documentLink/resolve",
                Self::DocumentSymbol => "textDocument/documentSymbol",
                Self::FoldingRange => "textDocument/foldingRange",
                Self::Formatting => "textDocument/formatting",
                Self::Hover => "textDocument/hover",
                Self::Implementation => "textDocument/implementation",
                Self::IncomingCalls => "callHierarchy/incomingCalls",
                Self::InlayHint => "textDocument/inlayHint",
                Self::LinkedEditingRange => "textDocument/linkedEditingRange",
                Self::Moniker => "textDocument/moniker",
                Self::OnTypeFormatting => "textDocument/onTypeFormatting",
                Self::OutgoingCalls => "callHierarchy/outgoingCalls",
                Self::PrepareCallHierarchy => "textDocument/prepareCallHierarchy",
                Self::PrepareRename => "textDocument/prepareRename",
                Self::PrepareTypeHierarchy => "textDocument/prepareTypeHierarchy",
                Self::PublishDiagnostics => "textDocument/publishDiagnostics",
                Self::RangeFormatting => "textDocument/rangeFormatting",
                Self::References => "textDocument/references",
                Self::Rename => "textDocument/rename",
                Self::SelectionRange => "textDocument/selectionRange",
                Self::SemanticTokensFull => "textDocument/semanticTokens/full",
                Self::SemanticTokensFullDelta => "textDocument/semanticTokens/full/delta",
                Self::SemanticTokensRange => "textDocument/semanticTokens/range",
                Self::SignatureHelp => "textDocument/signatureHelp",
                Self::TypeDefinition => "textDocument/typeDefinition",
                Self::WorkspaceDiagnostic => "workspace/diagnostic",
                Self::WorkspaceExecuteCommand => "workspace/executeCommand",
                Self::WorkspaceSymbol => "workspace/symbol",
                Self::WorkspaceSymbolResolve => "workspaceSymbol/resolve",
                Self::WorkspaceWillCreateFiles => "workspace/willCreateFiles",
                Self::WorkspaceWillDeleteFiles => "workspace/willDeleteFiles",
                Self::WorkspaceWillRenameFiles => "workspace/willRenameFiles",
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
///   set by the user.
/// - `executable_path`: path to the language server's executable.
/// - `nvim_path`: path to/command for the Neovim executable. The default is "nvim".
/// - `source_file`: the source file to be opened by Neovim.
/// - `cursor_pos`: the position of the cursor within `source_contents` when the
///   lsp request being tested is executed.
/// - `other_files`: other files to be placed in the mock directory (e.g. other source
///   files, server configuration, etc.).
/// - `start_type`: indicates when the server is ready to service requests
/// - `timeout`: timeout for the test's run in Neovim. The default is 1000ms.
/// - `cleanup`: whether to delete the temporary directory on test completion.
#[derive(Debug, Clone)]
pub struct TestCase {
    pub test_id: String,
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
    pub fn new<P: Into<PathBuf>>(executable_path: P, source_file: TestFile) -> Self {
        let nvim_path = std::env::var("LSPRESSO_NVIM")
            .unwrap_or_else(|_| "nvim".into())
            .into();
        Self {
            test_id: Self::generate_test_id(),
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
        for TestFile { path, .. } in &self.other_files {
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

    /// Returns the path to the benchmark file for test `test_id`,
    /// creating parent directories along the way. Any benchmark
    /// measurements recorded by the lua code will be recorded here.
    ///
    /// `/tmp/lspresso-shot/<test_id>/measurements.txt`
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the the test directory can't be created
    pub fn get_benchmark_file_path(&self) -> std::io::Result<PathBuf> {
        let mut lspresso_dir = self.get_lspresso_dir()?;
        lspresso_dir.push("measurements.txt");
        Ok(lspresso_dir)
    }

    /// Gathers the benchmark results from the benchmark file
    ///
    /// # Errors
    ///
    /// Rerurns [`BenchmarkError`] if the benchmark file can't be read,
    ///
    /// # Panics
    ///
    /// Will panic if the benchmark file contains a line that cannot be parsed
    /// as a `u64`.
    pub fn get_benchmark_results(&self) -> Result<Vec<Duration>, BenchmarkError> {
        let file_path = self
            .get_benchmark_file_path()
            .map_err(|_| BenchmarkError::NoResults)?;
        let contents = fs::read_to_string(file_path).map_err(|_| BenchmarkError::NoResults)?;
        let results: Vec<Duration> = contents
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| {
                let time = line.parse::<u64>().unwrap();
                Duration::from_nanos(time) // `vim.uv.hrtime()` measures in ns
            })
            .collect();
        Ok(results)
    }

    /// Returns the path to the timeout file for test `test_id`,
    /// creating parent directories along the way. If the neovim
    /// instance exited because the timeout was exceeded, this
    /// file will be created as a marker.
    ///
    /// `/tmp/lspresso-shot/<test_id>/timeout`
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the the test directory can't be created
    pub fn get_timeout_file_path(&self) -> std::io::Result<PathBuf> {
        let mut lspresso_dir = self.get_lspresso_dir()?;
        lspresso_dir.push("timeout");
        Ok(lspresso_dir)
    }

    /// Indicates if the test case's neovim instance exited because
    /// the case's timeout was exceeded.
    #[must_use]
    pub fn did_exceed_timeout(&self) -> bool {
        let timeout_file = self.get_timeout_file_path();
        if let Ok(timeout_file) = timeout_file {
            return timeout_file.exists();
        }
        false
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
    pub(crate) fn create_test(
        &self,
        test_type: TestType,
        replacements: &mut Vec<LuaReplacement>,
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

#[derive(Debug, Clone, Copy)]
pub enum EndCondition {
    /// Run `n` iterations of the benchmark
    Count(u64),
    /// Run the benchmark until the specified duration has elapsed
    Time(Duration),
}

impl Default for EndCondition {
    fn default() -> Self {
        Self::Count(100)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BenchmarkConfig {
    /// Determines how the benchmark should end under normal conditions
    pub end_condition: EndCondition,
    /// Stop the benchmark on the first error encountered
    pub fail_fast: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            end_condition: EndCondition::default(),
            fail_fast: true,
        }
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
    /// (i.e. "rustAnalyzer/cachePriming").
    Progress(NonZeroU32, String),
}

/// Response type for cases where it's reasonable to either compare the server's
/// actual response, or the state of the buffer after the response is received
#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub enum StateOrResponse<T> {
    /// Check if the buffer's state matches after the response is received
    State(String),
    /// Check if the server's response matches
    Response(T),
}

impl<T> From<TestError<T>> for TestError<StateOrResponse<T>> {
    fn from(value: TestError<T>) -> Self {
        match value {
            TestError::ResponseMismatch(ResponseMismatchError {
                test_id,
                expected,
                actual,
            }) => {
                let expected = expected.map(StateOrResponse::Response);
                let actual = actual.map(StateOrResponse::Response);
                Self::ResponseMismatch(ResponseMismatchError {
                    test_id,
                    expected,
                    actual,
                })
            }
            TestError::TestSetup(e) => Self::TestSetup(e),
            TestError::TestExecution(e) => Self::TestExecution(e),
        }
    }
}

/// Converts a `TestResult<(), String>` or `TestResult<(), Vec<TextEdit>>` to `TestResult<(), FormattingResult>`.
/// This is necessary to satisfy the generic constraints introduced by `test_formatting` calling
/// `test_formatting_resp` and `test_formatting_state`.
///
/// Note that we can't implement this logic directly via the `From` trait because `TestResult` is just an alias
/// for `Result`, and thus a foreign type.
pub(crate) fn to_parent_err_type<C, P>(
    result: TestResult<(), C>,
) -> TestResult<(), StateOrResponse<P>>
where
    TestError<StateOrResponse<P>>: From<TestError<C>>,
{
    match result {
        Ok(()) => Ok(()),
        Err(e) => Err(e.into()),
    }
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
    #[error("{0}")]
    IO(String),
}

impl From<std::io::Error> for TestSetupError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value.to_string())
    }
}

macro_rules! type_name {
    ($t:ty) => {{
        let full_name = std::any::type_name::<$t>();
        full_name
            .rsplit("::")
            .next()
            .unwrap_or(full_name)
            .trim_end_matches(|c| c == '<' || c == '>')
    }};
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct ResponseMismatchError<T> {
    pub test_id: String,
    pub expected: Option<T>,
    pub actual: Option<T>,
}

// TODO: Add a `display` field to `ResponseMismatchError` to allow for different
// error displays. We can have the existing JSON-ish diffing logic, debug prints
// of `actual` and `expected`, or a JSON print of the two.
impl<T: Serialize> std::fmt::Display for ResponseMismatchError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Test {}: Incorrect {} response:",
            self.test_id,
            type_name!(T)
        )?;
        match (self.expected.as_ref(), self.actual.as_ref()) {
            (Some(_), Some(_)) => writeln!(f)?,
            (None, Some(_)) => writeln!(f, "Expected `None`, got `Some`")?,
            (Some(_), None) => writeln!(f, "Expected `Some`, got `None`")?,
            (None, None) => unreachable!(),
        }
        write_fields_comparison(f, "", &self.expected, &self.actual, 0)?;

        Ok(())
    }
}

pub type TestResult<T, V> = Result<T, TestError<V>>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TestError<T> {
    #[allow(clippy::result_large_err)]
    #[error(transparent)]
    ResponseMismatch(#[from] ResponseMismatchError<T>),
    #[error(transparent)]
    TestExecution(#[from] TestExecutionError),
    #[error(transparent)]
    TestSetup(#[from] TestSetupError),
}

pub type TestExecutionResult<T> = Result<T, TestExecutionError>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TestExecutionError {
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

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BenchmarkError {
    #[error("Failed to gather benchmark results")]
    NoResults,
    #[error(transparent)]
    TestExecution(#[from] TestExecutionError),
    #[error(transparent)]
    TestSetup(#[from] TestSetupError),
}

/// Cleans a given `Uri` object of any information internal to the case
///
/// # Examples
///
/// `file:///tmp/lspresso-shot/<test-id>/src/foo.rs` -> `foo.rs`
///
/// # Errors
///
/// Returns `TestError::IO` on failure to get the root source file path from
/// `test_case`, or `TestSetupError::InvalidFilePath` if the root source file path
/// cannot be converted betwen a `Uri` and a `String`
pub fn clean_uri(uri: &Uri, test_case: &TestCase) -> TestExecutionResult<Uri> {
    let root = test_case
        .get_source_file_path("") // "/tmp/lspresso-shot/<test-id>/src/"
        .map_err(|e| TestExecutionError::IO(test_case.test_id.clone(), e.to_string()))?;
    let test_case_root = root
        .to_str()
        .ok_or_else(|| TestSetupError::InvalidFilePath(format!("{}", root.display())))?
        .to_string();
    let path = uri.path().to_string();
    let cleaned = path.strip_prefix(&test_case_root).unwrap_or(&path);
    Ok(Uri::from_str(cleaned).map_err(|_| TestSetupError::InvalidFilePath(path))?)
}

pub trait CleanResponse
where
    Self: Sized,
{
    /// Cleans a given resonse object of any Uri information related to the test case
    #[allow(unused_variables, unused_mut, clippy::missing_errors_doc)]
    fn clean_response(mut self, test_case: &TestCase) -> TestExecutionResult<Self> {
        Ok(self)
    }
}

impl CleanResponse for String {}
impl CleanResponse for LSPAny {}

/// This trait implements a comparison method that accounts for issues w.r.t. JSON
/// serialization/deserialization of types used in the LSP protocol.
pub trait ApproximateEq
where
    Self: Serialize + PartialEq,
{
    fn approx_eq(a: &Self, b: &Self) -> bool {
        a == b
    }
}

impl ApproximateEq for String {}
impl ApproximateEq for LSPAny {}
