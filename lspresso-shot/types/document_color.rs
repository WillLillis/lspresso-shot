use lsp_types::ColorInformation;

use super::{ApproximateEq, CleanResponse};

impl CleanResponse for Vec<ColorInformation> {}

impl ApproximateEq for Vec<ColorInformation> {}
