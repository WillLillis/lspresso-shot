use lsp_types::SignatureHelp;

use super::{ApproximateEq, CleanResponse};

impl CleanResponse for SignatureHelp {}

impl ApproximateEq for SignatureHelp {}
