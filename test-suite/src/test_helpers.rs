use lspresso_shot::types::TestFile;

#[must_use]
pub fn cargo_dot_toml() -> TestFile {
    TestFile::new(
        "Cargo.toml",
        r#"
[package]
name = "test"
version = "0.1.0"
edition = "2021"

[dependencies]

[[bin]]
name = "test"
path = "src/main.rs""#,
    )
}
