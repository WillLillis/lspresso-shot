use lsp_types::ColorPresentation;

use super::{ApproximateEq, CleanResponse};

impl CleanResponse for Vec<ColorPresentation> {}

impl ApproximateEq for Vec<ColorPresentation> {}
