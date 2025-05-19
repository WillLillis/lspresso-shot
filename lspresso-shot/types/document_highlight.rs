use lsp_types::DocumentHighlight;

use super::{ApproximateEq, CleanResponse};

impl CleanResponse for Vec<DocumentHighlight> {}

impl ApproximateEq for Vec<DocumentHighlight> {}
