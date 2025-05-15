use lsp_types::{SemanticTokensFullDeltaResult, SemanticTokensRangeResult, SemanticTokensResult};

use super::CleanResponse;

impl CleanResponse for SemanticTokensResult {}
impl CleanResponse for SemanticTokensFullDeltaResult {}
impl CleanResponse for SemanticTokensRangeResult {}
