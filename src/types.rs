use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::{env::temp_dir, fmt::Write};

use anstyle::{AnsiColor, Color, Style};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::init_dot_lua::{get_init_dot_lua, InitType};

// TODO: Don't special case printing of strings if they're a single/multiline
// Just stick them on the line below the field name

// Common Types

/// Describes a test case to be passed to be used in an lspresso-shot test.
///
/// - `source_path`: gives the test proejct-relative path for the file to be opened
///    in Neovim.
/// - `source_contents`: the contents of the source file to be opened by Neovim.
/// - `cursor_pos`: the position of the cursor within `source_contents` when the
///    lsp request being tested is executed.
/// - `other_files`: other files to be placed in the mock directory (e.g. other source
///    files, server configuration, etc.).
/// - `cleanup`: whether to delete the temporary directory on test completion.
#[derive(Debug, Clone)]
pub struct TestCase {
    pub source_path: PathBuf,
    pub source_contents: String,
    pub cursor_pos: Option<CursorPosition>,
    pub other_files: Vec<(PathBuf, String)>,
    pub cleanup: bool,
}
// TODO: Add some sort of `from_path` method for `TestCase`. Allows a user to point
// to a file or directory and automatically convert it into a `TestCase` instance.
// We need to be *very* careful in the case of directories, as the size could blow
// up easily. Might be smart to set some sort of limit on total capacity and return an
// error if converting a path would exceed it. What should this upper bound be?

impl TestCase {
    pub fn new<P: Into<PathBuf>>(source_path: P, source_contents: &str) -> Self {
        Self {
            source_path: source_path.into(),
            source_contents: source_contents.to_string(),
            cursor_pos: None,
            other_files: Vec::new(),
            cleanup: false,
        }
    }

    /// Set the cursor position in the source file
    #[must_use]
    pub const fn cursor_pos(mut self, cursor_pos: Option<CursorPosition>) -> Self {
        self.cursor_pos = cursor_pos;
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

    /// Validate the data contained within `self`
    pub fn validate(&self) -> Result<(), TestSetupError> {
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
    pub fn get_lspresso_dir(test_id: &str) -> std::io::Result<PathBuf> {
        let mut tmp_dir = temp_dir();
        tmp_dir.push("lspresso-shot");
        tmp_dir.push(test_id);
        fs::create_dir_all(&tmp_dir)?;
        Ok(tmp_dir)
    }

    /// Returns the path to the result file for test `test_id`,
    /// creating parent directories along the way
    ///
    /// /tmp/lspresso-shot/`test_id`/results.toml
    pub fn get_results_file_path(test_id: &str) -> std::io::Result<PathBuf> {
        let mut lspresso_dir = Self::get_lspresso_dir(test_id)?;
        fs::create_dir_all(&lspresso_dir)?;
        lspresso_dir.push("results.toml");
        Ok(lspresso_dir)
    }

    /// Returns the path to a source file for test `test_id`,
    /// creating parent directories along the way
    ///
    /// /tmp/lspresso-shot/`test_id`/src/`file_path`
    pub fn get_source_file_path(test_id: &str, file_path: &Path) -> std::io::Result<PathBuf> {
        let mut lspresso_dir = Self::get_lspresso_dir(test_id)?;
        lspresso_dir.push("src");
        fs::create_dir_all(&lspresso_dir)?;
        lspresso_dir.push(file_path);
        Ok(lspresso_dir)
    }

    /// Returns the path to a source file for test `test_id`,
    /// creating parent directories along the way
    ///
    /// /tmp/lspresso-shot/`test_id`/init.lua
    pub fn get_init_lua_file_path(test_id: &str) -> std::io::Result<PathBuf> {
        let mut lspresso_dir = Self::get_lspresso_dir(test_id)?;
        fs::create_dir_all(&lspresso_dir)?;
        lspresso_dir.push("init.lua");
        Ok(lspresso_dir)
    }

    /// Returns the path to the error file for test `test_id`,
    /// creating parent directories along the way. Any errors
    /// encounted by the config's lua code will be recorded here
    ///
    /// /tmp/lspresso-shot/`test_id`/init.lua
    pub fn get_error_file_path(test_id: &str) -> std::io::Result<PathBuf> {
        let mut lspresso_dir = Self::get_lspresso_dir(test_id)?;
        fs::create_dir_all(&lspresso_dir)?;
        lspresso_dir.push("error.txt");
        Ok(lspresso_dir)
    }

    /// Creates a test directory for `test_id` based on `self`
    pub fn create_test(
        &self,
        test_id: &str,
        executable_path: &Path,
        test_type: InitType,
    ) -> TestResult<PathBuf> {
        if let Some(cursor_pos) = self.cursor_pos {
            if cursor_pos.line == 0 {
                Err(TestSetupError::InvalidCursorPosition(
                    "Cursor line position is 1-based".to_string(),
                ))?;
            }
        }
        let results_file_path = Self::get_results_file_path(test_id)?;
        let init_dot_lua_path = Self::get_init_lua_file_path(test_id)?;
        let root_path = Self::get_lspresso_dir(test_id)?;
        let error_path = Self::get_error_file_path(test_id)?;
        let extension = self
            .source_path
            .extension()
            .ok_or_else(|| {
                TestSetupError::MissingFileExtension(self.source_path.to_string_lossy().to_string())
            })?
            .to_str()
            .ok_or_else(|| {
                TestSetupError::InvalidFileExtension(self.source_path.to_string_lossy().to_string())
            })?;

        {
            let nvim_config = get_init_dot_lua(
                test_type,
                &root_path,
                &results_file_path,
                &error_path,
                executable_path,
                extension,
                self.cursor_pos,
            );
            fs::File::create(&init_dot_lua_path)?;
            fs::write(&init_dot_lua_path, &nvim_config)?;
        }

        let source_path = Self::get_source_file_path(test_id, &self.source_path)?;
        // Source file paths should always have a parent directory
        fs::create_dir_all(source_path.parent().unwrap())?;
        fs::File::create(&source_path)?;
        fs::write(&source_path, &self.source_contents)?;

        for (path, contents) in &self.other_files {
            let source_file_path = Self::get_source_file_path(test_id, path)?;
            // Source file paths should always have a parent directory
            fs::create_dir_all(source_file_path.parent().unwrap())?;
            fs::File::create(&source_file_path)?;
            fs::write(&source_file_path, contents)?;
        }
        Ok(source_path)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: usize,
    pub column: usize,
}

impl CursorPosition {
    #[must_use]
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Debug, Error)]
pub enum TestSetupError {
    #[error("Source file \"{0}\" must have an extension")]
    MissingFileExtension(String),
    #[error("The extension of source file \"{0}\" is invalid")]
    InvalidFileExtension(String),
    #[error("Source file path \"{0}\" is invalid")]
    InvalidFilePath(String),
    #[error("{0}")]
    InvalidCursorPosition(String),
}

impl From<std::io::Error> for TestError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value.to_string())
    }
}

// Hover types
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct HoverResult {
    // TODO: turn this into an enum? What are the possible values besides 'markdown'?
    pub kind: String,
    pub value: String,
}

pub type TestResult<T> = Result<T, TestError>;

#[derive(Debug, Error)]
pub enum TestError {
    #[error(transparent)]
    HoverMismatch(#[from] HoverMismatchError),
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

#[derive(Debug, Error)]
pub struct HoverMismatchError {
    pub expected: HoverResult,
    pub actual: HoverResult,
}

const GREEN: Option<Color> = Some(anstyle::Color::Ansi(AnsiColor::Green));
const RED: Option<Color> = Some(anstyle::Color::Ansi(AnsiColor::Red));

// TODO: Just make this write directly?
fn render_field_comparison<T: Eq + std::fmt::Display>(
    field_name: &str,
    expected: Option<&T>,
    actual: Option<&T>,
) -> Result<String, std::fmt::Error> {
    let mut out = String::new();
    match (expected, actual) {
        (None, None) => writeln!(&mut out, "{field_name}: nil")?,
        (Some(expected), Some(actual)) => {
            let rendered_expected = expected.to_string();
            let rendered_actual = actual.to_string();
            let separate_lines =
                rendered_expected.lines().count() > 1 || rendered_actual.lines().count() > 1;
            if expected != actual {
                writeln!(&mut out, "{}:", paint(RED, field_name))?;
                if separate_lines {
                    writeln!(
                        &mut out,
                        "    Expected:\n'{expected}'\n    Got:\n'{actual}'",
                    )?;
                } else {
                    writeln!(
                        &mut out,
                        "    Expected: '{expected}'\n    Got:      '{actual}'",
                    )?;
                }
            } else {
                let rendered_field = expected.to_string();
                let separator = if rendered_field.lines().count() > 1
                    || matches!(rendered_field.chars().last(), Some('\n'))
                {
                    "\n"
                } else {
                    " "
                };
                writeln!(
                    &mut out,
                    "{}:{separator}'{}'",
                    paint(GREEN, field_name),
                    expected
                )?;
            }
        }
        (Some(expected), None) => {
            writeln!(&mut out, "{}:", paint(RED, field_name))?;
            writeln!(&mut out, "    Expected: '{expected}'\n    Got:      nil",)?;
        }
        (None, Some(actual)) => {
            writeln!(&mut out, "{}:", paint(RED, field_name))?;
            writeln!(&mut out, "    Expected: nil\n    Got:      '{actual}'",)?;
        }
    }

    Ok(out)
}

impl std::fmt::Display for HoverMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            render_field_comparison("kind", Some(&self.expected.kind), Some(&self.actual.kind))?
        )?;

        write!(
            f,
            "{}",
            render_field_comparison(
                "value",
                Some(&self.expected.value),
                Some(&self.actual.value)
            )?
        )?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub struct DiagnosticMismatchError {
    pub expected: DiagnosticResult,
    pub actual: DiagnosticResult,
}

// TODO: Do something clever with closures/macros to reduce duplicate code
impl std::fmt::Display for DiagnosticMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, (expected, actual)) in self
            .expected
            .diagnostics
            .iter()
            .zip(self.actual.diagnostics.iter())
            .enumerate()
        {
            writeln!(f, "Diagnostic {i}:")?;
            write!(
                f,
                "{}",
                render_field_comparison(
                    "start_line",
                    Some(&expected.start_line),
                    Some(&actual.start_line)
                )?
            )?;
            write!(
                f,
                "{}",
                render_field_comparison(
                    "start_character",
                    Some(&expected.start_character),
                    Some(&actual.start_character)
                )?
            )?;
            write!(
                f,
                "{}",
                render_field_comparison(
                    "end_line",
                    expected.end_line.as_ref(),
                    actual.end_line.as_ref()
                )?
            )?;
            write!(
                f,
                "{}",
                render_field_comparison(
                    "end_character",
                    expected.end_character.as_ref(),
                    actual.end_character.as_ref()
                )?
            )?;
            write!(
                f,
                "{}",
                render_field_comparison("message", Some(&expected.message), Some(&actual.message))?
            )?;
            write!(
                f,
                "{}",
                render_field_comparison(
                    "severity",
                    expected.severity.as_ref(),
                    actual.severity.as_ref()
                )?
            )?;
            write!(f, "\n\n")?;
        }

        let render_diagnostic =
            |f: &mut std::fmt::Formatter<'_>, diagnostic: &DiagnosticInfo| -> std::fmt::Result {
                writeln!(
                    f,
                    "{}: '{}'",
                    paint(RED, "start_line"),
                    diagnostic.start_line
                )?;
                writeln!(
                    f,
                    "{}: '{}'",
                    paint(RED, "start_character"),
                    diagnostic.start_character
                )?;
                writeln!(
                    f,
                    "{}: '{}'",
                    paint(RED, "end_line"),
                    diagnostic
                        .end_line
                        .map_or("'nil'".to_string(), |c| c.to_string())
                )?;
                writeln!(
                    f,
                    "{}: '{}'",
                    paint(RED, "end_character"),
                    diagnostic
                        .end_character
                        .map_or("'nil'".to_string(), |c| c.to_string())
                )?;
                writeln!(f, "{}:\n'{}'", paint(RED, "message"), diagnostic.message)?;
                writeln!(
                    f,
                    "{}: '{}'",
                    paint(RED, "severity"),
                    diagnostic
                        .severity
                        .map_or("'nil'".to_string(), |s| s.to_string())
                )?;
                write!(f, "\n\n")?;
                Ok(())
            };

        let expected_len = self.expected.diagnostics.len();
        let actual_len = self.actual.diagnostics.len();
        match expected_len.cmp(&actual_len) {
            std::cmp::Ordering::Equal => {}
            std::cmp::Ordering::Less => {
                for (i, diagnostic) in self
                    .actual
                    .diagnostics
                    .iter()
                    .enumerate()
                    .skip(expected_len)
                {
                    writeln!(f, "Diagnostic {i}:")?;
                    writeln!(f, "Expected: nil")?;
                    writeln!(f, "Got:")?;
                    render_diagnostic(f, diagnostic)?;
                }
            }
            std::cmp::Ordering::Greater => {
                for (i, diagnostic) in self
                    .expected
                    .diagnostics
                    .iter()
                    .enumerate()
                    .skip(actual_len)
                {
                    writeln!(f, "Diagnostic {i}:")?;
                    writeln!(f, "Expected:")?;
                    render_diagnostic(f, diagnostic)?;
                    writeln!(f, "Got: nil")?;
                }
            }
        };

        Ok(())
    }
}

fn paint(color: Option<impl Into<Color>>, text: &str) -> String {
    let style = Style::new().fg_color(color.map(Into::into));
    format!("{style}{text}{style:#}")
}

// Diagnostic Types
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticResult {
    pub diagnostics: Vec<DiagnosticInfo>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticInfo {
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: Option<u32>,
    pub end_character: Option<u32>,
    pub message: String,
    pub severity: Option<DiagnosticSeverity>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    #[serde(rename = "1")]
    Error,
    #[serde(rename = "2")]
    Warn,
    #[serde(rename = "3")]
    Info,
    #[serde(rename = "4")]
    Hint,
}

impl std::fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Error => "error",
                Self::Warn => "warn",
                Self::Info => "info",
                Self::Hint => "hint",
            }
        )?;
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
    Contains(HashSet<CompletionInfo>),
    /// Expect this exact set of completion results in a specific order
    Exact(Vec<CompletionInfo>),
}

impl CompletionResult {
    /// Compares the expected results in `self` to the `actual` results, respecting
    /// the intended behavior for each enum variant of `Self`
    ///
    /// Returns true if the two are considered equal, false otherwise
    #[must_use]
    pub fn compare_results(&self, actual: &Vec<CompletionInfo>) -> bool {
        match self {
            Self::LessThan(n_expected) => *n_expected > actual.len(),
            Self::Exactly(n_expected) => *n_expected == actual.len(),
            Self::MoreThan(n_expected) => *n_expected < actual.len(),
            Self::Contains(expected) => {
                let mut remaining = expected.clone();
                for result in actual {
                    if remaining.is_empty() {
                        return false;
                    }
                    if remaining.contains(result) {
                        remaining.remove(result);
                    }
                }
                remaining.is_empty()
            }
            Self::Exact(results) => {
                if results.len() != actual.len() {
                    return false;
                }

                for (expected, actual) in results.iter().zip(actual.iter()) {
                    if expected != actual {
                        return false;
                    }
                }
                true
            }
        }
    }
}

// NOTE: There are a *lot* of optional fields in the completion response item
// For now we'll add a few basic ones, and then come back to later
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct CompletionInfo {
    label: String,
    #[serde(flatten)]
    documentation: CompletionDocumentation,
    kind: CompletionItemKind,
}

impl std::fmt::Display for CompletionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "label: '{}'", self.label)?;
        writeln!(f, "documentation:\n'{}'", self.documentation)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct CompletionDocumentation {
    #[serde(rename = "documentation_kind")]
    kind: String,
    #[serde(rename = "documentation_value")]
    value: String,
}

impl std::fmt::Display for CompletionDocumentation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "kind: {}", self.kind)?;
        let separator =
            if self.value.lines().count() > 1 || matches!(self.value.chars().last(), Some('\n')) {
                "\n"
            } else {
                " "
            };
        writeln!(f, "value:{separator}'{}'", self.value)?;
        Ok(())
    }
}

// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#completionItemKind
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum CompletionItemKind {
    #[serde(rename = "1")]
    Text = 1,
    #[serde(rename = "2")]
    Method = 2,
    #[serde(rename = "3")]
    Function = 3,
    #[serde(rename = "4")]
    Constructor = 4,
    #[serde(rename = "5")]
    Field = 5,
    #[serde(rename = "6")]
    Variable = 6,
    #[serde(rename = "7")]
    Class = 7,
    #[serde(rename = "8")]
    Interface = 8,
    #[serde(rename = "9")]
    Module = 9,
    #[serde(rename = "10")]
    Property = 10,
    #[serde(rename = "11")]
    Unit = 11,
    #[serde(rename = "12")]
    Value = 12,
    #[serde(rename = "13")]
    Enum = 13,
    #[serde(rename = "14")]
    Keyword = 14,
    #[serde(rename = "15")]
    Snippet = 15,
    #[serde(rename = "16")]
    Color = 16,
    #[serde(rename = "17")]
    File = 17,
    #[serde(rename = "18")]
    Reference = 18,
    #[serde(rename = "19")]
    Folder = 19,
    #[serde(rename = "20")]
    EnumMember = 20,
    #[serde(rename = "21")]
    Constant = 21,
    #[serde(rename = "22")]
    Struct = 22,
    #[serde(rename = "23")]
    Event = 23,
    #[serde(rename = "24")]
    Operator = 24,
    #[serde(rename = "25")]
    TypeParameter = 25,
}

impl std::fmt::Display for CompletionItemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "{}",
            match self {
                Self::Text => "Text",
                Self::Method => "Method",
                Self::Function => "Function",
                Self::Constructor => "Constructor",
                Self::Field => "Field",
                Self::Variable => "Variable",
                Self::Class => "Class",
                Self::Interface => "Interface",
                Self::Module => "Module",
                Self::Property => "Property",
                Self::Unit => "Unit",
                Self::Value => "Value",
                Self::Enum => "Enum",
                Self::Keyword => "Keyword",
                Self::Snippet => "Snippet",
                Self::Color => "Color",
                Self::File => "File",
                Self::Reference => "Reference",
                Self::Folder => "Folder",
                Self::EnumMember => "EnumMember",
                Self::Constant => "Constant",
                Self::Struct => "Struct",
                Self::Event => "Event",
                Self::Operator => "Operator",
                Self::TypeParameter => "TypeParameter",
            }
        )?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub struct CompletionMismatchError {
    pub expected: CompletionResult,
    pub actual: Vec<CompletionInfo>,
}

impl std::fmt::Display for CompletionMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.expected {
            CompletionResult::LessThan(n_results) => writeln!(
                f,
                "{}",
                &paint(
                    RED,
                    &format!(
                        "Expected less than {n_results} completion results, got {}",
                        self.actual.len()
                    )
                ),
            )?,
            CompletionResult::Exactly(n_results) => writeln!(
                f,
                "{}",
                &paint(
                    RED,
                    &format!(
                        "Expected exactly {n_results} completion results, got {}",
                        self.actual.len()
                    )
                )
            )?,
            CompletionResult::MoreThan(n_results) => writeln!(
                f,
                "{}",
                &paint(
                    RED,
                    &format!(
                        "Expected more than {n_results} completion results, got {}",
                        self.actual.len()
                    ),
                ),
            )?,
            CompletionResult::Contains(results) => {
                let mut remaining = results.clone();
                let mut remaining_count: Option<usize> = None;
                for (i, result) in self.actual.iter().enumerate() {
                    if remaining.is_empty() {
                        remaining_count = Some(i + 1);
                        break;
                    }
                    if remaining.contains(result) {
                        remaining.remove(result);
                    }
                }
                if !remaining.is_empty() {
                    writeln!(
                        f,
                        "{}",
                        &paint(
                            RED,
                            "Didn't recieve all of the expected completion results:"
                        )
                    )?;
                    for result in remaining {
                        writeln!(f, "Completion Result:\n{result}\n",)?;
                    }
                } else if let Some(count) = remaining_count {
                    writeln!(f, "{}", paint(RED, "Got unexpected completion results:"))?;
                    for result in self.actual.iter().skip(count) {
                        writeln!(f, "Completion Result:\n{result}\n",)?;
                    }
                }
            }
            CompletionResult::Exact(results) => {
                let render_diff = |f: &mut std::fmt::Formatter<'_>,
                                   expected: &CompletionInfo,
                                   actual: &CompletionInfo|
                 -> std::fmt::Result {
                    write!(
                        f,
                        "{}",
                        render_field_comparison(
                            "label",
                            Some(&expected.label),
                            Some(&actual.label)
                        )?
                    )?;
                    write!(
                        f,
                        "{}",
                        render_field_comparison(
                            "documentation",
                            Some(&expected.documentation),
                            Some(&actual.documentation)
                        )?
                    )?;
                    write!(
                        f,
                        "{}",
                        render_field_comparison("kind", Some(&expected.kind), Some(&actual.kind))?
                    )?;

                    Ok(())
                };
                let render_completion = |f: &mut std::fmt::Formatter<'_>,
                                         completion: &CompletionInfo|
                 -> std::fmt::Result {
                    write!(f, "label: '{}'", paint(RED, &completion.label.to_string()))?;
                    write!(
                        f,
                        "documentation:\n'{}'",
                        paint(RED, &format!("{}", completion.documentation))
                    )?;
                    write!(f, "kind: '{}'", paint(RED, &format!("{}", completion.kind)))?;

                    Ok(())
                };
                for (expected, actual) in results.iter().zip(self.actual.iter()) {
                    if expected != actual {
                        render_diff(f, expected, actual)?;
                    }
                }
                let expected_len = results.len();
                let actual_len = self.actual.len();
                match expected_len.cmp(&actual_len) {
                    std::cmp::Ordering::Equal => {}
                    std::cmp::Ordering::Less => {
                        for (i, completion) in self.actual.iter().enumerate().skip(expected_len) {
                            writeln!(f, "Completion {i}:")?;
                            writeln!(f, "Expected: nil")?;
                            writeln!(f, "Got:")?;
                            render_completion(f, completion)?;
                        }
                    }
                    std::cmp::Ordering::Greater => {
                        for (i, completion) in results.iter().enumerate().skip(actual_len) {
                            writeln!(f, "Completion {i}:")?;
                            writeln!(f, "Expected:")?;
                            render_completion(f, completion)?;
                            writeln!(f, "Got: nil")?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

// Definition Types
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct DefinitionResult {
    pub start_pos: CursorPosition,
    pub end_pos: Option<CursorPosition>,
    // this is actually returned as a uri ("file:/./..."), but the uri crate's
    // type doesn't support serde. As such, we'll just grab the path
    pub path: PathBuf,
}

impl std::fmt::Display for DefinitionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "start_line: '{}'", self.start_pos.line)?;
        writeln!(f, "start_column: '{}'", self.start_pos.column)?;
        if let Some(end) = self.end_pos {
            writeln!(f, "end_line: '{}'", end.line)?;
            writeln!(f, "end_column: '{}'", end.column)?;
        } else {
            writeln!(f, "end_line: 'nil'")?;
            writeln!(f, "end_column: 'nil'")?;
        }
        writeln!(f, "path: '{}'", self.path.display())?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub struct DefinitionMismatchError {
    pub expected: DefinitionResult,
    pub actual: DefinitionResult,
}

impl std::fmt::Display for DefinitionMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "{}",
            render_field_comparison(
                "start_line",
                Some(&self.expected.start_pos.line),
                Some(&self.actual.start_pos.line),
            )?
        )?;
        writeln!(
            f,
            "{}",
            render_field_comparison(
                "start_column",
                Some(&self.expected.start_pos.column),
                Some(&self.actual.start_pos.column),
            )?
        )?;
        let (expected_end_line, expected_end_column) = self
            .expected
            .end_pos
            .map_or((None, None), |p| (Some(p.line), Some(p.column)));
        let (actual_end_line, actual_end_column) = self
            .actual
            .end_pos
            .map_or((None, None), |p| (Some(p.line), Some(p.column)));
        writeln!(
            f,
            "{}",
            render_field_comparison(
                "end_line",
                expected_end_line.as_ref(),
                actual_end_line.as_ref(),
            )?
        )?;
        writeln!(
            f,
            "{}",
            render_field_comparison(
                "end_column",
                expected_end_column.as_ref(),
                actual_end_column.as_ref(),
            )?
        )?;
        writeln!(
            f,
            "{}",
            render_field_comparison(
                "path",
                Some(&self.expected.path.to_string_lossy()),
                Some(&self.actual.path.to_string_lossy()),
            )?
        )?;

        Ok(())
    }
}
