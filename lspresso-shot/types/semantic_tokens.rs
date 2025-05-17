use lsp_types::{SemanticTokensFullDeltaResult, SemanticTokensRangeResult, SemanticTokensResult};

use super::{ApproximateEq, CleanResponse};

impl CleanResponse for SemanticTokensResult {}
impl CleanResponse for SemanticTokensFullDeltaResult {}
impl CleanResponse for SemanticTokensRangeResult {}

impl ApproximateEq for SemanticTokensResult {
    fn approx_eq(a: &Self, b: &Self) -> bool {
        match (a, b) {
            (Self::Tokens(tokens), Self::Partial(partial))
            | (Self::Partial(partial), Self::Tokens(tokens)) => {
                tokens.result_id.is_none() && tokens.data.eq(&partial.data)
            }
            _ => a == b,
        }
    }
}

impl ApproximateEq for SemanticTokensFullDeltaResult {
    fn approx_eq(a: &Self, b: &Self) -> bool {
        match (a, b) {
            (Self::Tokens(tokens), Self::TokensDelta(delta))
            | (Self::TokensDelta(delta), Self::Tokens(tokens)) => {
                tokens.result_id.is_none() && tokens.data.is_empty() && delta.edits.is_empty()
            }
            (Self::Tokens(tokens), Self::PartialTokensDelta { edits })
            | (Self::PartialTokensDelta { edits }, Self::Tokens(tokens)) => {
                tokens.result_id.is_none() && tokens.data.is_empty() && edits.is_empty()
            }
            (Self::TokensDelta(delta), Self::PartialTokensDelta { edits })
            | (Self::PartialTokensDelta { edits }, Self::TokensDelta(delta)) => {
                delta.result_id.is_none() && delta.edits.is_empty() && edits.is_empty()
            }
            _ => a == b,
        }
    }
}

impl ApproximateEq for SemanticTokensRangeResult {
    fn approx_eq(a: &Self, b: &Self) -> bool {
        match (a, b) {
            (Self::Tokens(tokens), Self::Partial(partial))
            | (Self::Partial(partial), Self::Tokens(tokens)) => {
                tokens.result_id.is_none() && tokens.data.eq(&partial.data)
            }
            _ => a == b,
        }
    }
}
