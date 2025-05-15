use lsp_types::{CompletionItem, CompletionResponse};

use super::CleanResponse;

impl CleanResponse for CompletionResponse {}
impl CleanResponse for CompletionItem {}
