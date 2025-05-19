use lsp_types::CodeLens;

use super::{ApproximateEq, CleanResponse};

impl CleanResponse for CodeLens {}
impl CleanResponse for Vec<CodeLens> {}

impl ApproximateEq for CodeLens {}
impl ApproximateEq for Vec<CodeLens> {}
