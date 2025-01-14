use std::path::PathBuf;

use anstyle::{AnsiColor, Color, Style};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
    pub fn source_file<P: Into<PathBuf>>(mut self, path: P, contents: &str) -> Self {
        self.source_path = path.into();
        self.source_contents = contents.to_string();
        self
    }

    /// Add an additional file to the test case
    pub fn other_file<P: Into<PathBuf>>(mut self, path: P, contents: &str) -> Self {
        self.other_files.push((path.into(), contents.to_string()));
        self
    }

    /// Change whether the temporary directory is cleaned up on test completion
    pub fn cleanup(mut self, cleanup: bool) -> Self {
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
}

#[derive(Debug, Copy, Clone)]
pub struct CursorPosition {
    pub line: usize,
    pub column: usize,
}

impl CursorPosition {
    pub fn new(line: usize, column: usize) -> Self {
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

impl From<std::io::Error> for HoverTestError {
    fn from(value: std::io::Error) -> Self {
        HoverTestError::IO(value.to_string())
    }
}

// Hover types
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct HoverResult {
    pub kind: String, // TODO: turn this into an enum? What are the possible values?
    pub value: String,
}

pub type HoverTestResult<T> = Result<T, HoverTestError>;

#[derive(Debug, Error)]
pub enum HoverTestError {
    #[error(transparent)]
    Mismatch(#[from] HoverMismatchError),
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

impl std::fmt::Display for HoverMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.expected.kind != self.actual.kind {
            let color = Some(AnsiColor::Red);
            writeln!(f, "{}", paint(color, "kind:"))?;
            writeln!(
                f,
                "    Expected: '{}'\n    Got:      '{}'",
                self.expected.kind, self.actual.kind
            )?;
        } else {
            let color = Some(AnsiColor::Green);
            writeln!(f, "{}'{}'", paint(color, "kind: "), self.expected.kind)?;
        }

        if self.expected.value != self.actual.value {
            let color = Some(AnsiColor::Red);
            writeln!(f, "{}", paint(color, "value:"))?;
            writeln!(
                f,
                "    Expected: '{}'\n    Got:      '{}'",
                self.expected.value, self.actual.value
            )?;
        } else {
            let color = Some(AnsiColor::Green);
            writeln!(f, "{}\n\"{}\"", paint(color, "value:"), self.expected.value)?;
        }

        Ok(())
    }
}

fn paint(color: Option<impl Into<Color>>, text: &str) -> String {
    let style = Style::new().fg_color(color.map(Into::into));
    format!("{style}{text}{style:#}")
}

// TODO:
// Diagnostic Types
// #[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
// pub struct DiagnosticResult {
//     // NOTE: do we need this if we're only opening one file?
//     pub bufnr: u32,
//     pub col: u32,
//     pub end_col: u32,
//     pub end_lnum: u32,
//     pub lnum: u32,
//     pub message: String,
//     pub severity: DiagnosticSeverity,
// }
//
// #[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
// pub enum DiagnosticSeverity {
//     #[serde(rename = "error")]
//     Error,
//     #[serde(rename = "warn")]
//     Warn,
//     #[serde(rename = "info")]
//     Info,
//     #[serde(rename = "hint")]
//     Hint,
// }
//
// impl From<u32> for DiagnosticSeverity {
//     fn from(value: u32) -> Self {
//         match value {
//             1 => Self::Error,
//             2 => Self::Warn,
//             3 => Self::Info,
//             4 => Self::Hint,
//             v => panic!("Invalid diagnostic severity: {v}"),
//         }
//     }
// }
