use lsp_types::SelectionRange;

use super::{ApproximateEq, CleanResponse};

impl CleanResponse for Vec<SelectionRange> {}

impl ApproximateEq for Vec<SelectionRange> {}
