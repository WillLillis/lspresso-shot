use std::fs;
use std::path::{Path, PathBuf};
use std::{env::temp_dir, fmt::Write};

use anstyle::{AnsiColor, Color, Style};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::init_dot_lua::{get_init_dot_lua, InitType};

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
    pub cursor_pos: CursorPosition,
    pub other_files: Vec<(PathBuf, String)>,
    pub cleanup: bool,
}
// TODO: Add some sort of `from_path` method for `TestCase`. Allows a user to point
// to a file or directory and automatically convert it into a `TestCase` instance.
// We need to be *very* careful in the case of directories, as the size could blow
// up easily. Might be smart to set some sort of limit on total capacity and return an
// error if converting a path would exceed it. What should this upper bound be?

impl TestCase {
    pub fn new<P: Into<PathBuf>>(
        source_path: P,
        source_contents: &str,
        cursor_pos: CursorPosition,
    ) -> Self {
        Self {
            source_path: source_path.into(),
            source_contents: source_contents.to_string(),
            cursor_pos,
            other_files: Vec::new(),
            cleanup: false,
        }
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
        if self.cursor_pos.line == 0 {
            Err(TestSetupError::InvalidCursorPosition)?;
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
            )
            .replace("CURSOR_LINE", &self.cursor_pos.line.to_string())
            .replace("CURSOR_COLUMN", &self.cursor_pos.column.to_string());
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

#[derive(Debug, Copy, Clone)]
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
    #[error("Cursor line position is 1-based")]
    InvalidCursorPosition,
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
