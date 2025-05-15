use lsp_types::CodeLens;

use super::CleanResponse;

impl CleanResponse for CodeLens {}
impl CleanResponse for Vec<CodeLens> {}
