use std::{collections::HashMap, str::FromStr};

use lsp_types::{
    ChangeAnnotation, CodeDescription, Diagnostic, DiagnosticRelatedInformation, DocumentChanges,
    GotoDefinitionResponse, Location, LocationLink, Position, PublishDiagnosticsParams, Range,
    TextDocumentEdit, TextEdit, Uri, WorkspaceEdit,
};

use crate::get_source_path;

// TODO: Figure out way to publish different diagnostics
/// For use with `test_diagnostics`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_diagnostics_response(uri: &Uri) -> PublishDiagnosticsParams {
    PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics: vec![Diagnostic {
            range: Range {
                start: Position::new(1, 2),
                end: Position::new(3, 4),
            },
            severity: Some(lsp_types::DiagnosticSeverity::ERROR),
            code: None,
            code_description: Some(CodeDescription {
                href: Uri::from_str(&get_source_path()).unwrap(),
            }),
            source: None,
            message: "message".to_string(),
            tags: None,
            related_information: Some(vec![DiagnosticRelatedInformation {
                location: Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position::new(5, 6),
                        end: Position::new(7, 8),
                    },
                },
                message: "related message".to_string(),
            }]),
            data: None,
        }],
        version: None,
    }
}

/// For use with `test_definition`.
/// Returns a different `Vec<GotoDefinitionResponse>` based on `response_num`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_definition_response(response_num: u32) -> Option<GotoDefinitionResponse> {
    match response_num {
        0 => Some(GotoDefinitionResponse::Scalar(Location {
            uri: Uri::from_str(&get_source_path()).unwrap(),
            range: Range {
                start: Position::new(1, 2),
                end: Position::new(3, 4),
            },
        })),
        1 => Some(GotoDefinitionResponse::Array(vec![Location {
            uri: Uri::from_str(&get_source_path()).unwrap(),
            range: Range {
                start: Position::new(1, 2),
                end: Position::new(3, 4),
            },
        }])),
        2 => Some(GotoDefinitionResponse::Link(vec![LocationLink {
            target_uri: Uri::from_str(&get_source_path()).unwrap(),
            target_range: Range {
                start: Position::new(1, 2),
                end: Position::new(3, 4),
            },
            target_selection_range: Range {
                start: Position::new(5, 6),
                end: Position::new(7, 8),
            },
            origin_selection_range: Some(Range {
                start: Position::new(9, 10),
                end: Position::new(11, 12),
            }),
        }])),
        _ => None,
    }
}

/// For use with `test_rename`.
/// Returns a different `Vec<WorkspaceEdit>` based on `response_num`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_rename_response(response_num: u32) -> Option<WorkspaceEdit> {
    match response_num {
        0 => Some(WorkspaceEdit {
            changes: None,
            document_changes: None,
            change_annotations: None,
        }),
        1 => {
            let mut changes = HashMap::new();
            changes.insert(
                Uri::from_str(&get_source_path()).unwrap(),
                vec![TextEdit {
                    range: Range {
                        start: Position::new(1, 2),
                        end: Position::new(3, 4),
                    },
                    new_text: "new_text".to_string(),
                }],
            );
            Some(WorkspaceEdit {
                changes: Some(changes),
                document_changes: None,
                change_annotations: None,
            })
        }
        2 => Some(WorkspaceEdit {
            changes: None,
            document_changes: Some(DocumentChanges::Edits(vec![TextDocumentEdit {
                text_document: lsp_types::OptionalVersionedTextDocumentIdentifier {
                    uri: Uri::from_str(&get_source_path()).unwrap(),
                    version: None,
                },
                edits: Vec::new(),
            }])),
            change_annotations: None,
        }),
        3 => {
            let mut changes = HashMap::new();
            changes.insert(
                get_source_path(),
                ChangeAnnotation {
                    label: "label".to_string(),
                    needs_confirmation: None,
                    description: None,
                },
            );
            Some(WorkspaceEdit {
                changes: None,
                document_changes: None,
                change_annotations: Some(changes),
            })
        }
        _ => None,
    }
}

/// For use with `test_references`.
/// Returns a different `Vec<Location>` based on `response_num`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_references_response(response_num: u32) -> Option<Vec<Location>> {
    let uri = Uri::from_str(&get_source_path()).unwrap();
    match response_num {
        0 => Some(vec![]),
        1 => Some(vec![Location {
            uri,
            range: Range {
                start: Position::new(1, 2),
                end: Position::new(3, 4),
            },
        }]),
        2 => Some(vec![
            Location {
                uri: uri.clone(),
                range: Range {
                    start: Position::new(1, 2),
                    end: Position::new(3, 4),
                },
            },
            Location {
                uri,
                range: Range {
                    start: Position::new(5, 6),
                    end: Position::new(7, 8),
                },
            },
        ]),
        3 => Some(vec![
            Location {
                uri: uri.clone(),
                range: Range {
                    start: Position::new(1, 2),
                    end: Position::new(3, 4),
                },
            },
            Location {
                uri: uri.clone(),
                range: Range {
                    start: Position::new(5, 6),
                    end: Position::new(7, 8),
                },
            },
            Location {
                uri,
                range: Range {
                    start: Position::new(9, 10),
                    end: Position::new(11, 12),
                },
            },
        ]),
        _ => None,
    }
}

/// For use with `test_formatting`.
/// Returns a different `Vec<TextEdit>` based on `response_num`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_formatting_response(response_num: u32) -> Option<Vec<TextEdit>> {
    match response_num {
        0 => Some(vec![]),
        1 => Some(vec![TextEdit {
            range: Range {
                start: Position::new(1, 2),
                end: Position::new(3, 4),
            },
            new_text: "new_text 1".to_string(),
        }]),
        2 => Some(vec![
            TextEdit {
                range: Range {
                    start: Position::new(1, 2),
                    end: Position::new(3, 4),
                },
                new_text: "new_text 1".to_string(),
            },
            TextEdit {
                range: Range {
                    start: Position::new(5, 6),
                    end: Position::new(7, 8),
                },
                new_text: "new_text 2".to_string(),
            },
        ]),
        3 => Some(vec![
            TextEdit {
                range: Range {
                    start: Position::new(1, 2),
                    end: Position::new(3, 4),
                },
                new_text: "new_text 1".to_string(),
            },
            TextEdit {
                range: Range {
                    start: Position::new(5, 6),
                    end: Position::new(7, 8),
                },
                new_text: "new_text 2".to_string(),
            },
            TextEdit {
                range: Range {
                    start: Position::new(9, 10),
                    end: Position::new(11, 12),
                },
                new_text: "new_text 3".to_string(),
            },
        ]),
        _ => None,
    }
}
