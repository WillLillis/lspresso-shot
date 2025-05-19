use lsp_types::FoldingRange;

use super::{ApproximateEq, CleanResponse};

impl CleanResponse for Vec<FoldingRange> {}

impl ApproximateEq for Vec<FoldingRange> {}
