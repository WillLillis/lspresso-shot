use lsp_types::Moniker;

use super::{ApproximateEq, CleanResponse};

impl CleanResponse for Vec<Moniker> {}

impl ApproximateEq for Vec<Moniker> {}
