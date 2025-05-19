use lsp_types::{OneOf, SymbolInformation, WorkspaceSymbol, WorkspaceSymbolResponse};

use super::{ApproximateEq, CleanResponse, TestExecutionResult, clean_uri};

impl CleanResponse for WorkspaceSymbol {
    fn clean_response(mut self, test_case: &super::TestCase) -> TestExecutionResult<Self> {
        match &mut self.location {
            OneOf::Left(location) => {
                let uri = location.uri.clone();
                location.uri = clean_uri(&uri, test_case)?;
            }
            OneOf::Right(workspace_location) => {
                let uri = workspace_location.uri.clone();
                workspace_location.uri = clean_uri(&uri, test_case)?;
            }
        }
        Ok(self)
    }
}

impl CleanResponse for WorkspaceSymbolResponse {
    fn clean_response(mut self, test_case: &super::TestCase) -> TestExecutionResult<Self> {
        match &mut self {
            Self::Flat(symbols) => {
                for symbol in symbols.iter_mut() {
                    let uri = symbol.location.uri.clone();
                    symbol.location.uri = clean_uri(&uri, test_case)?;
                }
            }
            Self::Nested(symbols) => {
                for symbol in symbols.iter_mut() {
                    *symbol = symbol.clone().clean_response(test_case)?;
                }
            }
        }
        Ok(self)
    }
}

impl ApproximateEq for WorkspaceSymbolResponse {
    fn approx_eq(a: &Self, b: &Self) -> bool {
        match (a, b) {
            (Self::Nested(nested), Self::Flat(flat)) | (Self::Flat(flat), Self::Nested(nested)) => {
                if flat.len() != nested.len() {
                    return false;
                }
                !flat
                    .iter()
                    .zip(nested.iter())
                    .any(|(sym_info, workspace_sym)| {
                        !cmp_inner(sym_info, workspace_sym) // return true if *not* equal
                    })
            }
            _ => a == b,
        }
    }
}

fn cmp_inner(sym_info: &SymbolInformation, workspace_sym: &WorkspaceSymbol) -> bool {
    // The two are structurally identical in their JSON representations iff:
    //   - `sym_info.deprecated` is `None`
    //   - `workspace_sym.location` is the `OneOf::Left(Location)` variant
    //   - `workspace_sym.data` is `None`
    #[allow(deprecated)]
    if sym_info.deprecated.is_some() {
        return false;
    }
    if workspace_sym.data.is_some() {
        return false;
    }
    if let OneOf::Left(location) = &workspace_sym.location {
        if sym_info.location != *location {
            return false;
        }
    } else {
        return false;
    }

    // If we've confirmed the two are *structurally* identical, compare the
    // remaining common fields
    if sym_info.name != workspace_sym.name {
        return false;
    }
    if sym_info.kind != workspace_sym.kind {
        return false;
    }
    if sym_info.tags != workspace_sym.tags {
        return false;
    }
    if sym_info.container_name != workspace_sym.container_name {
        return false;
    }

    true
}
