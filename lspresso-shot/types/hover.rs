use lsp_types::Hover;

use super::{ApproximateEq, CleanResponse};

impl CleanResponse for Hover {}

impl ApproximateEq for Hover {}
