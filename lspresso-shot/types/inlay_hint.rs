use lsp_types::InlayHint;

use super::{ApproximateEq, CleanResponse};

impl CleanResponse for Vec<InlayHint> {}

impl ApproximateEq for Vec<InlayHint> {}
