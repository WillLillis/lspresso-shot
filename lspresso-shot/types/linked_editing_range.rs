use lsp_types::LinkedEditingRanges;

use super::{ApproximateEq, CleanResponse};

impl CleanResponse for LinkedEditingRanges {}

impl ApproximateEq for LinkedEditingRanges {}
