use lsp_types::{CompletionItem, CompletionResponse};

use super::{ApproximateEq, CleanResponse};

impl CleanResponse for CompletionResponse {}
impl CleanResponse for CompletionItem {}

impl ApproximateEq for CompletionResponse {}
impl ApproximateEq for CompletionItem {}
