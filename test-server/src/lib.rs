use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use lsp_types::{ServerCapabilities, Uri};

pub mod handle;
pub mod responses;

/// Returns the path to the test server executable
#[allow(clippy::missing_panics_doc)]
#[must_use]
pub fn get_dummy_server_path() -> PathBuf {
    let mut proj_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    proj_dir.push("target");
    proj_dir.push("debug");
    proj_dir.push("test-server");

    proj_dir
}

/// Returns `main.dummy`
#[must_use]
pub fn get_source_path() -> String {
    "main.dummy".to_string()
}

/// Given a `URI` pointing to *some* file within an lspresso-shot
/// test directory, returns the test directory's root path
///
/// For example, "/tmp/lspresso-shot/5382805252853875543/src/main.dummy"
/// would get transformed into /tmp/lspresso-shot/5382805252853875543/"
///
/// Since we want to avoid circular dependencies, this is a bit
/// of a hack rather than using functionality from the lib itself
pub fn get_root_test_path(uri: &Uri) -> Option<PathBuf> {
    let lspresso = "lspresso-shot";
    let uri_str = uri.path().to_string();
    let mut lspresso_idx = uri_str.find(lspresso)?;
    lspresso_idx += lspresso.len() + 1; // +1 to account for path separator
    let end_idx = uri_str
        .chars()
        .enumerate()
        .skip(lspresso_idx)
        .find(|(_, c)| !c.is_ascii_digit())
        .map(|(i, _)| i)?;

    let uri: String = uri_str.chars().take(end_idx).collect();
    Some(uri.into())
}

// NOTE: This (and `send_capabilities`) could also be accomplished by adding the file
// as an "other file" with path `../<filename>` to the test case, but this seems
// a bit brittle and much less explicit.
/// Writes `response_num` to `path/RESPONSE_NUM.txt`
///
/// # Errors
///
/// Will return `std::io::Error` if writing the file fails
pub fn send_response_num(response_num: u32, path: &Path) -> std::io::Result<()> {
    let mut path = path.to_path_buf();
    path.push("RESPONSE_NUM.txt");

    std::fs::write(path, response_num.to_string())
}

/// Serialized `capabilities` to JSON and writes them to `path/capabilities.json`
///
/// # Errors
///
/// Will return `std::io::Error` if writing the file fails
///
/// # Panics
///
/// Will panic if serialization of `capabilities` fails
pub fn send_capabiltiies(capabilities: &ServerCapabilities, path: &Path) -> std::io::Result<()> {
    let mut path = path.to_path_buf();
    path.push("capabilities.json");
    let capabilities_json =
        serde_json::to_string_pretty(capabilities).expect("Failed to serialize capabilities");

    std::fs::write(path, capabilities_json)
}

/// Reads a response number from `path/RESPONSE_NUM.txt`
///
/// # Errors
///
/// Will return `Err` if reading or parsing the file fails
pub fn receive_response_num(path: &Path) -> Result<u32> {
    let mut path = path.to_path_buf();
    path.push("RESPONSE_NUM.txt");
    let response_str = std::fs::read_to_string(path)?;

    match response_str.parse::<u32>() {
        Ok(num) => Ok(num),
        Err(e) => Err(anyhow!("Failed to parse response num contents -- {e}")),
    }
}
