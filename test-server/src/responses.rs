use std::{collections::HashMap, str::FromStr};

use lsp_types::{
    request::{GotoDeclarationResponse, GotoImplementationResponse, GotoTypeDefinitionResponse},
    CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall, ChangeAnnotation,
    CodeDescription, CompletionItem, CompletionItemKind, CompletionItemLabelDetails,
    CompletionList, CompletionResponse, Diagnostic, DiagnosticRelatedInformation, DocumentChanges,
    DocumentHighlight, DocumentHighlightKind, DocumentLink, DocumentSymbol, DocumentSymbolResponse,
    Documentation, GotoDefinitionResponse, Hover, HoverContents, LanguageString, Location,
    LocationLink, MarkedString, MarkupContent, MarkupKind, Position, PublishDiagnosticsParams,
    Range, SymbolInformation, SymbolKind, SymbolTag, TextDocumentEdit, TextEdit, Uri,
    WorkspaceEdit,
};

use crate::get_dummy_source_path;

/// For use with `test_document_highlight`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_document_highlight_response(response_num: u32) -> Option<Vec<DocumentHighlight>> {
    let item1 = DocumentHighlight {
        range: Range {
            start: Position::new(1, 2),
            end: Position::new(3, 4),
        },
        kind: None,
    };
    let item2 = DocumentHighlight {
        range: Range {
            start: Position::new(5, 6),
            end: Position::new(7, 8),
        },
        kind: Some(DocumentHighlightKind::TEXT),
    };
    match response_num {
        0 => Some(vec![]),
        1 => Some(vec![item1]),
        2 => Some(vec![item2]),
        3 => Some(vec![item1, item2]),
        _ => None,
    }
}

/// For use with `test_document_highlight`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_document_link_response(response_num: u32) -> Option<Vec<DocumentLink>> {
    let item1 = DocumentLink {
        range: Range {
            start: Position::new(1, 2),
            end: Position::new(3, 4),
        },
        target: None,
        tooltip: None,
        data: None,
    };
    let item2 = DocumentLink {
        range: Range {
            start: Position::new(1, 2),
            end: Position::new(3, 4),
        },
        target: Some(Uri::from_str(&get_dummy_source_path()).unwrap()),
        tooltip: None,
        data: None,
    };
    let item3 = DocumentLink {
        range: Range {
            start: Position::new(1, 2),
            end: Position::new(3, 4),
        },
        target: Some(Uri::from_str(&get_dummy_source_path()).unwrap()),
        tooltip: Some("tooltip".to_string()),
        data: None,
    };
    match response_num {
        0 => Some(vec![]),
        1 => Some(vec![item1]),
        2 => Some(vec![item2]),
        3 => Some(vec![item3]),
        4 => Some(vec![item1, item2, item3]),
        _ => None,
    }
}

/// For use with `test_document_symbol`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_document_symbol_response(response_num: u32) -> Option<DocumentSymbolResponse> {
    #[allow(deprecated)]
    match response_num {
        0 => Some(DocumentSymbolResponse::Flat(vec![])),
        1 => Some(DocumentSymbolResponse::Nested(vec![])),
        2 => Some(DocumentSymbolResponse::Flat(vec![SymbolInformation {
            name: "symbol name 1".to_string(),
            kind: SymbolKind::FILE,
            tags: None,
            deprecated: None,
            location: Location {
                uri: Uri::from_str(&get_dummy_source_path()).unwrap(),
                range: Range {
                    start: Position::new(0, 1),
                    end: Position::new(2, 3),
                },
            },
            container_name: Some("container name 1".to_string()),
        }])),
        3 => Some(DocumentSymbolResponse::Nested(vec![DocumentSymbol {
            name: "symbol name 2".to_string(),
            detail: Some("detail".to_string()),
            kind: SymbolKind::FUNCTION,
            tags: Some(vec![SymbolTag::DEPRECATED]),
            deprecated: Some(true),
            range: Range {
                start: Position::new(4, 5),
                end: Position::new(6, 7),
            },
            selection_range: Range {
                start: Position::new(5, 6),
                end: Position::new(7, 8),
            },
            children: Some(vec![]),
        }])),
        _ => None,
    }
}

/// For use with `test_completion`.
#[must_use]
pub fn get_completion_response(response_num: u32) -> Option<CompletionResponse> {
    let item1 = CompletionItem {
        label: "label1".to_string(),
        label_details: Some(CompletionItemLabelDetails {
            detail: Some("detail1".to_string()),
            description: Some("description1".to_string()),
        }),
        kind: Some(CompletionItemKind::TEXT),
        detail: Some("detail1".to_string()),
        documentation: Some(Documentation::String("doc string1".to_string())),
        deprecated: Some(false),
        preselect: Some(true),
        sort_text: Some("sort text1".to_string()),
        filter_text: Some("filter_text1".to_string()),
        insert_text: Some("insert_text1".to_string()),
        insert_text_format: None,
        insert_text_mode: None,
        text_edit: None,
        additional_text_edits: None,
        command: None,
        commit_characters: None,
        data: None,
        tags: None,
    };
    let item2 = CompletionItem {
        label: "label2".to_string(),
        label_details: Some(CompletionItemLabelDetails {
            detail: Some("detail2".to_string()),
            description: Some("description2".to_string()),
        }),
        kind: Some(CompletionItemKind::TEXT),
        detail: Some("detail2".to_string()),
        documentation: Some(Documentation::String("doc string2".to_string())),
        deprecated: Some(false),
        preselect: Some(true),
        sort_text: Some("sort text2".to_string()),
        filter_text: Some("filter_text2".to_string()),
        insert_text: Some("insert_text2".to_string()),
        insert_text_format: None,
        insert_text_mode: None,
        text_edit: None,
        additional_text_edits: None,
        command: None,
        commit_characters: None,
        data: None,
        tags: None,
    };
    match response_num {
        0 => Some(CompletionResponse::List(CompletionList {
            is_incomplete: true,
            items: vec![],
        })),
        1 => Some(CompletionResponse::List(CompletionList {
            is_incomplete: true,
            items: vec![item1],
        })),
        2 => Some(CompletionResponse::List(CompletionList {
            is_incomplete: true,
            items: vec![item1, item2],
        })),
        3 => Some(CompletionResponse::List(CompletionList {
            is_incomplete: false,
            items: vec![item1],
        })),
        4 => Some(CompletionResponse::List(CompletionList {
            is_incomplete: false,
            items: vec![item1, item2],
        })),
        5 => Some(CompletionResponse::Array(vec![])),
        6 => Some(CompletionResponse::Array(vec![item1])),
        7 => Some(CompletionResponse::Array(vec![item1, item2])),
        _ => None,
    }
}

/// For use with `test_hover`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_hover_response(response_num: u32) -> Option<Hover> {
    match response_num {
        0 => Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(
                "Scalar Marked String\nLine two".to_string(),
            )),
            range: Some(Range {
                start: Position::new(1, 2),
                end: Position::new(3, 4),
            }),
        }),
        1 => Some(Hover {
            contents: HoverContents::Scalar(MarkedString::LanguageString(LanguageString {
                language: "dummy-lang".to_string(),
                value: "dummy-val\nLine two".to_string(),
            })),
            range: Some(Range {
                start: Position::new(5, 6),
                end: Position::new(7, 8),
            }),
        }),
        2 => Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "Markup Content\nMore content".to_string(),
            }),
            range: Some(Range {
                start: Position::new(9, 10),
                end: Position::new(11, 12),
            }),
        }),
        3 => Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::PlainText,
                value: "Plain Content\nPlain Jane".to_string(),
            }),
            range: Some(Range {
                start: Position::new(13, 14),
                end: Position::new(15, 16),
            }),
        }),
        // BUG: https://github.com/serde-rs/json/issues/1244
        // 4 => Some(Hover {
        //     contents: HoverContents::Array(vec![
        //         MarkedString::String("Array Marked String 1".to_string()),
        //         MarkedString::String("Array Marked String 2".to_string()),
        //     ]),
        //     range: Some(Range {
        //         start: Position::new(13, 14),
        //         end: Position::new(15, 16),
        //     }),
        // }),
        4 => Some(Hover {
            contents: HoverContents::Array(vec![]),
            range: Some(Range {
                start: Position::new(17, 18),
                end: Position::new(19, 20),
            }),
        }),
        5 => Some(Hover {
            contents: HoverContents::Array(vec![
                MarkedString::String("Array Marked String 1\nExtra".to_string()),
                MarkedString::String("Array Marked String 2\nExtra extra".to_string()),
                MarkedString::String("Array Marked String 3\nJust kidding".to_string()),
            ]),
            range: Some(Range {
                start: Position::new(21, 22),
                end: Position::new(23, 24),
            }),
        }),
        6 => Some(Hover {
            contents: HoverContents::Array(vec![
                MarkedString::LanguageString(LanguageString {
                    language: "dummy-lang".to_string(),
                    value: "dummy-val1\nDon't crash".to_string(),
                }),
                MarkedString::LanguageString(LanguageString {
                    language: "dummy-lang".to_string(),
                    value: "dummy-val2\nSeriously".to_string(),
                }),
            ]),
            range: Some(Range {
                start: Position::new(13, 14),
                end: Position::new(15, 16),
            }),
        }),
        _ => None,
    }
}

/// For use with `test_implementation`.
///
/// Since `textDocument/definition` and `textDocument/implementation` have the same
/// response, this just wraps `get_definition_response`.
#[must_use]
pub fn get_implementation_response(response_num: u32) -> Option<GotoImplementationResponse> {
    get_definition_response(response_num)
}

/// For use with `test_incoming_calls`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_incoming_calls_response(response_num: u32) -> Option<Vec<CallHierarchyIncomingCall>> {
    let item1 = CallHierarchyIncomingCall {
        from: CallHierarchyItem {
            name: "name1".to_string(),
            kind: SymbolKind::FILE,
            tags: None,
            detail: Some("detail1".to_string()),
            uri: Uri::from_str(&get_dummy_source_path()).unwrap(),
            range: Range {
                start: Position::new(1, 2),
                end: Position::new(3, 4),
            },
            selection_range: Range {
                start: Position::new(5, 6),
                end: Position::new(7, 8),
            },
            data: None,
        },
        from_ranges: vec![Range {
            start: Position::new(9, 10),
            end: Position::new(11, 12),
        }],
    };
    let item2 = CallHierarchyIncomingCall {
        from: CallHierarchyItem {
            name: "name2".to_string(),
            kind: SymbolKind::FILE,
            tags: None,
            detail: Some("detail2".to_string()),
            uri: Uri::from_str(&get_dummy_source_path()).unwrap(),
            range: Range {
                start: Position::new(1, 2),
                end: Position::new(3, 4),
            },
            selection_range: Range {
                start: Position::new(5, 6),
                end: Position::new(7, 8),
            },
            data: None,
        },
        from_ranges: vec![Range {
            start: Position::new(9, 10),
            end: Position::new(11, 12),
        }],
    };
    match response_num {
        0 => Some(vec![]),
        1 => Some(vec![item1]),
        2 => Some(vec![item2]),
        3 => Some(vec![item1, item2]),
        _ => None,
    }
}

/// For use with `test_outgoing_calls`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_outgoing_calls_response(response_num: u32) -> Option<Vec<CallHierarchyOutgoingCall>> {
    let item1 = CallHierarchyOutgoingCall {
        to: CallHierarchyItem {
            name: "name1".to_string(),
            kind: SymbolKind::FILE,
            tags: None,
            detail: Some("detail1".to_string()),
            uri: Uri::from_str(&get_dummy_source_path()).unwrap(),
            range: Range {
                start: Position::new(1, 2),
                end: Position::new(3, 4),
            },
            selection_range: Range {
                start: Position::new(5, 6),
                end: Position::new(7, 8),
            },
            data: None,
        },
        from_ranges: vec![Range {
            start: Position::new(9, 10),
            end: Position::new(11, 12),
        }],
    };
    let item2 = CallHierarchyOutgoingCall {
        to: CallHierarchyItem {
            name: "name2".to_string(),
            kind: SymbolKind::FILE,
            tags: None,
            detail: Some("detail2".to_string()),
            uri: Uri::from_str(&get_dummy_source_path()).unwrap(),
            range: Range {
                start: Position::new(1, 2),
                end: Position::new(3, 4),
            },
            selection_range: Range {
                start: Position::new(5, 6),
                end: Position::new(7, 8),
            },
            data: None,
        },
        from_ranges: vec![Range {
            start: Position::new(9, 10),
            end: Position::new(11, 12),
        }],
    };
    match response_num {
        0 => Some(vec![]),
        1 => Some(vec![item1]),
        2 => Some(vec![item2]),
        3 => Some(vec![item1, item2]),
        _ => None,
    }
}

/// For use with `test_prepare_call_hierarchy`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_prepare_call_hierachy_response(response_num: u32) -> Option<Vec<CallHierarchyItem>> {
    let item1 = CallHierarchyItem {
        name: "name1".to_string(),
        kind: SymbolKind::FILE,
        tags: None,
        detail: Some("detail1".to_string()),
        uri: Uri::from_str(&get_dummy_source_path()).unwrap(),
        range: Range {
            start: Position::new(1, 2),
            end: Position::new(3, 4),
        },
        selection_range: Range {
            start: Position::new(5, 6),
            end: Position::new(7, 8),
        },
        data: None,
    };
    let item2 = CallHierarchyItem {
        name: "name2".to_string(),
        kind: SymbolKind::FILE,
        tags: None,
        detail: Some("detail2\nmore details".to_string()),
        uri: Uri::from_str(&get_dummy_source_path()).unwrap(),
        range: Range {
            start: Position::new(9, 10),
            end: Position::new(11, 12),
        },
        selection_range: Range {
            start: Position::new(13, 14),
            end: Position::new(15, 16),
        },
        data: None,
    };
    match response_num {
        0 => Some(vec![]),
        1 => Some(vec![item1]),
        2 => Some(vec![item2]),
        3 => Some(vec![item1, item2]),
        _ => None,
    }
}

/// For use with `test_diagnostics`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_diagnostics_response(response_num: u32, uri: &Uri) -> Option<PublishDiagnosticsParams> {
    let item = Diagnostic {
        range: Range {
            start: Position::new(1, 2),
            end: Position::new(3, 4),
        },
        severity: Some(lsp_types::DiagnosticSeverity::ERROR),
        code: None,
        code_description: Some(CodeDescription {
            href: Uri::from_str(&get_dummy_source_path()).unwrap(),
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
    };
    match response_num {
        0 => Some(PublishDiagnosticsParams {
            uri: uri.clone(),
            diagnostics: vec![],
            version: None,
        }),
        1 => Some(PublishDiagnosticsParams {
            uri: uri.clone(),
            diagnostics: vec![item],
            version: None,
        }),
        2 => Some(PublishDiagnosticsParams {
            uri: uri.clone(),
            diagnostics: vec![item.clone(), item],
            version: None,
        }),
        _ => None,
    }
}

/// For use with `test_declaration`.
///
/// Since `textDocument/definition` and `textDocument/declaration` have the same response,
/// this just wraps `get_definition_response`.
#[must_use]
pub fn get_declaration_response(response_num: u32) -> Option<GotoDeclarationResponse> {
    get_definition_response(response_num)
}

/// For use with `test_definition`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_definition_response(response_num: u32) -> Option<GotoDefinitionResponse> {
    let location_item = Location {
        uri: Uri::from_str(&get_dummy_source_path()).unwrap(),
        range: Range {
            start: Position::new(1, 2),
            end: Position::new(3, 4),
        },
    };
    let link_item = LocationLink {
        target_uri: Uri::from_str(&get_dummy_source_path()).unwrap(),
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
    };
    match response_num {
        0 => Some(GotoDefinitionResponse::Array(vec![])),
        1 => Some(GotoDefinitionResponse::Array(vec![location_item])),
        2 => Some(GotoDefinitionResponse::Array(vec![
            location_item.clone(),
            location_item,
        ])),
        3 => Some(GotoDefinitionResponse::Link(vec![])),
        4 => Some(GotoDefinitionResponse::Link(vec![link_item])),
        5 => Some(GotoDefinitionResponse::Link(vec![
            link_item.clone(),
            link_item,
        ])),
        6 => Some(GotoDefinitionResponse::Scalar(location_item)),
        _ => None,
    }
}

/// For use with `test_rename`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_rename_response(response_num: u32) -> Option<WorkspaceEdit> {
    match response_num {
        0 => Some(WorkspaceEdit {
            changes: Some(HashMap::new()),
            document_changes: None,
            change_annotations: None,
        }),
        1 => {
            let mut changes = HashMap::new();
            changes.insert(
                Uri::from_str(&get_dummy_source_path()).unwrap(),
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
                    uri: Uri::from_str(&get_dummy_source_path()).unwrap(),
                    version: None,
                },
                edits: Vec::new(),
            }])),
            change_annotations: None,
        }),
        3 => Some(WorkspaceEdit {
            changes: None,
            document_changes: Some(DocumentChanges::Edits(vec![])),
            change_annotations: None,
        }),
        4 => {
            let mut changes = HashMap::new();
            changes.insert(
                get_dummy_source_path(),
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
        5 => Some(WorkspaceEdit {
            changes: None,
            document_changes: None,
            change_annotations: Some(HashMap::new()),
        }),
        _ => None,
    }
}

/// For use with `test_references`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_references_response(response_num: u32) -> Option<Vec<Location>> {
    let uri = Uri::from_str(&get_dummy_source_path()).unwrap();
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

/// For use with `test_declaration`.
///
/// Since `textDocument/definition` and `textDocument/typeDefinition` have the same
/// response, this just wraps `get_definition_response`.
#[must_use]
pub fn get_type_definition_response(response_num: u32) -> Option<GotoTypeDefinitionResponse> {
    get_definition_response(response_num)
}

/// For use with `test_formatting`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_formatting_response(response_num: u32) -> Option<Vec<TextEdit>> {
    match response_num {
        // NOTE: The dummy tests rely on a `response_num` of 0 to return an empty edit response
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
