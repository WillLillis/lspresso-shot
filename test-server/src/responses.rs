use std::{collections::HashMap, str::FromStr};

use lsp_types::{
    request::{GotoDeclarationResponse, GotoImplementationResponse, GotoTypeDefinitionResponse},
    CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall, ChangeAnnotation,
    CodeDescription, CodeLens, CompletionItem, CompletionItemKind, CompletionItemLabelDetails,
    CompletionList, CompletionResponse, Diagnostic, DiagnosticRelatedInformation, DocumentChanges,
    DocumentDiagnosticReport, DocumentDiagnosticReportKind, DocumentHighlight,
    DocumentHighlightKind, DocumentLink, DocumentSymbol, DocumentSymbolResponse, Documentation,
    FoldingRange, FoldingRangeKind, FullDocumentDiagnosticReport, GotoDefinitionResponse, Hover,
    HoverContents, LanguageString, Location, LocationLink, MarkedString, MarkupContent, MarkupKind,
    Moniker, MonikerKind, ParameterInformation, ParameterLabel, Position, PublishDiagnosticsParams,
    Range, RelatedFullDocumentDiagnosticReport, SelectionRange, SemanticToken, SemanticTokens,
    SemanticTokensDelta, SemanticTokensEdit, SemanticTokensFullDeltaResult,
    SemanticTokensPartialResult, SemanticTokensRangeResult, SemanticTokensResult, SignatureHelp,
    SignatureInformation, SymbolInformation, SymbolKind, SymbolTag, TextDocumentEdit, TextEdit,
    UnchangedDocumentDiagnosticReport, UniquenessLevel, Uri, WorkspaceDiagnosticReport,
    WorkspaceDocumentDiagnosticReport, WorkspaceEdit, WorkspaceFullDocumentDiagnosticReport,
    WorkspaceUnchangedDocumentDiagnosticReport,
};

use crate::get_dummy_source_path;

/// For use with `test_document_highlight`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_document_highlight_response(
    response_num: u32,
    uri: &Uri,
) -> Option<Vec<DocumentHighlight>> {
    _ = uri;
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

/// For use with `test_document_link`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_document_link_response(response_num: u32, uri: &Uri) -> Option<Vec<DocumentLink>> {
    _ = uri;
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

/// For use with `test_document_link_resolve`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_document_link_resolve_response(response_num: u32, uri: &Uri) -> Option<DocumentLink> {
    _ = uri;
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
        0 => Some(item1),
        1 => Some(item2),
        2 => Some(item3),
        _ => None,
    }
}

/// For use with `test_document_symbol`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_document_symbol_response(
    response_num: u32,
    uri: &Uri,
) -> Option<DocumentSymbolResponse> {
    _ = uri;
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

/// For use with `test_code_lens`.
#[must_use]
pub fn get_code_lens_response(response_num: u32, uri: &Uri) -> Option<Vec<CodeLens>> {
    _ = uri;
    let item1 = CodeLens {
        range: Range {
            start: Position::new(1, 2),
            end: Position::new(3, 4),
        },
        command: None,
        data: None,
    };
    let item2 = CodeLens {
        range: Range {
            start: Position::new(5, 6),
            end: Position::new(7, 8),
        },
        command: Some(lsp_types::Command {
            title: "title".to_string(),
            command: "command".to_string(),
            arguments: None,
        }),
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

/// For use with `test_code_lens_resolve`.
#[must_use]
pub fn get_code_lens_resolve_response(response_num: u32, uri: &Uri) -> Option<CodeLens> {
    _ = uri;
    let item1 = CodeLens {
        range: Range {
            start: Position::new(1, 2),
            end: Position::new(3, 4),
        },
        command: None,
        data: None,
    };
    let item2 = CodeLens {
        range: Range {
            start: Position::new(5, 6),
            end: Position::new(7, 8),
        },
        command: Some(lsp_types::Command {
            title: "title".to_string(),
            command: "command".to_string(),
            arguments: None,
        }),
        data: None,
    };
    match response_num {
        0 => Some(item1),
        1 => Some(item2),
        _ => None,
    }
}

/// For use with `test_completion`.
#[must_use]
pub fn get_completion_response(response_num: u32, uri: &Uri) -> Option<CompletionResponse> {
    _ = uri;
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

/// For use with `test_completion_resolve`.
#[must_use]
pub fn get_completion_resolve_response(response_num: u32, uri: &Uri) -> Option<CompletionItem> {
    _ = uri;
    match response_num {
        0 => Some(CompletionItem {
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
        }),
        1 => Some(CompletionItem {
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
        }),
        _ => None,
    }
}

/// For use with `test_hover`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_hover_response(response_num: u32, uri: &Uri) -> Option<Hover> {
    _ = uri;
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
pub fn get_implementation_response(
    response_num: u32,
    uri: &Uri,
) -> Option<GotoImplementationResponse> {
    get_definition_response(response_num, uri)
}

/// For use with `test_incoming_calls`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_incoming_calls_response(
    response_num: u32,
    uri: &Uri,
) -> Option<Vec<CallHierarchyIncomingCall>> {
    _ = uri;
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

/// For use with `test_moniker`.
#[must_use]
pub fn get_moniker_response(response_num: u32, uri: &Uri) -> Option<Vec<Moniker>> {
    _ = uri;
    let item1 = Moniker {
        scheme: "scheme1".to_string(),
        identifier: "identifier1".to_string(),
        unique: UniquenessLevel::Document,
        kind: Some(MonikerKind::Export),
    };
    let item2 = Moniker {
        scheme: "scheme2".to_string(),
        identifier: "identifier2".to_string(),
        unique: UniquenessLevel::Project,
        kind: Some(MonikerKind::Import),
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
pub fn get_outgoing_calls_response(
    response_num: u32,
    uri: &Uri,
) -> Option<Vec<CallHierarchyOutgoingCall>> {
    _ = uri;
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
pub fn get_prepare_call_hierachy_response(
    response_num: u32,
    uri: &Uri,
) -> Option<Vec<CallHierarchyItem>> {
    _ = uri;
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

/// For use with `test_diagnostic`.
#[must_use]
#[allow(clippy::missing_panics_doc, clippy::too_many_lines)]
pub fn get_diagnostic_response(response_num: u32, uri: &Uri) -> Option<DocumentDiagnosticReport> {
    let item1 = Diagnostic {
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
                uri: Uri::from_str(&get_dummy_source_path()).unwrap(),
                range: Range {
                    start: Position::new(5, 6),
                    end: Position::new(7, 8),
                },
            },
            message: "related message".to_string(),
        }]),
        data: None,
    };
    let item2 = Diagnostic {
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
                uri: Uri::from_str(&get_dummy_source_path()).unwrap(),
                range: Range {
                    start: Position::new(5, 6),
                    end: Position::new(7, 8),
                },
            },
            message: "related message".to_string(),
        }]),
        data: None,
    };
    let mut related_documents = HashMap::new();
    related_documents.insert(
        uri.clone(),
        DocumentDiagnosticReportKind::Full(FullDocumentDiagnosticReport {
            result_id: None,
            items: vec![item1.clone()],
        }),
    );

    match response_num {
        0 => Some(DocumentDiagnosticReport::Full(
            RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items: vec![],
                },
            },
        )),
        1 => Some(DocumentDiagnosticReport::Full(
            RelatedFullDocumentDiagnosticReport {
                related_documents: Some(related_documents.clone()),
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items: vec![],
                },
            },
        )),
        2 => Some(DocumentDiagnosticReport::Full(
            RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items: vec![item1],
                },
            },
        )),
        3 => Some(DocumentDiagnosticReport::Full(
            RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items: vec![item2],
                },
            },
        )),
        4 => Some(DocumentDiagnosticReport::Full(
            RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items: vec![item1, item2],
                },
            },
        )),
        _ => None,
    }
}

/// For use with `test_publish_diagnostics`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_workspace_diagnostics_response(
    response_num: u32,
    uri: &Uri,
) -> Option<WorkspaceDiagnosticReport> {
    let subitem = Diagnostic {
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
    let item1 = WorkspaceDocumentDiagnosticReport::Full(WorkspaceFullDocumentDiagnosticReport {
        uri: uri.clone(),
        version: Some(0),
        full_document_diagnostic_report: FullDocumentDiagnosticReport {
            result_id: None,
            items: vec![subitem],
        },
    });
    let item2 =
        WorkspaceDocumentDiagnosticReport::Unchanged(WorkspaceUnchangedDocumentDiagnosticReport {
            uri: uri.clone(),
            version: Some(0),
            unchanged_document_diagnostic_report: UnchangedDocumentDiagnosticReport {
                result_id: "result_id".to_string(),
            },
        });
    match response_num {
        0 => Some(WorkspaceDiagnosticReport { items: vec![] }),
        1 => Some(WorkspaceDiagnosticReport { items: vec![item1] }),
        2 => Some(WorkspaceDiagnosticReport { items: vec![item2] }),
        3 => Some(WorkspaceDiagnosticReport {
            items: vec![item1, item2],
        }),
        _ => None,
    }
}

/// For use with `test_publish_diagnostics`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_publish_diagnostics_response(
    response_num: u32,
    uri: &Uri,
) -> Option<PublishDiagnosticsParams> {
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
pub fn get_declaration_response(response_num: u32, uri: &Uri) -> Option<GotoDeclarationResponse> {
    get_definition_response(response_num, uri)
}

/// For use with `test_definition`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_definition_response(response_num: u32, uri: &Uri) -> Option<GotoDefinitionResponse> {
    _ = uri;
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
pub fn get_rename_response(response_num: u32, uri: &Uri) -> Option<WorkspaceEdit> {
    _ = uri;
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
pub fn get_references_response(response_num: u32, uri: &Uri) -> Option<Vec<Location>> {
    _ = uri;
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

/// For use with `test_selection_range`.
#[must_use]
pub fn get_selection_range_response(response_num: u32, uri: &Uri) -> Option<Vec<SelectionRange>> {
    _ = uri;
    let item1 = SelectionRange {
        range: Range {
            start: Position::new(1, 2),
            end: Position::new(3, 4),
        },
        parent: None,
    };
    let item2 = SelectionRange {
        range: Range {
            start: Position::new(5, 6),
            end: Position::new(7, 8),
        },
        parent: Some(Box::new(item1.clone())),
    };
    match response_num {
        0 => Some(vec![]),
        1 => Some(vec![item1]),
        2 => Some(vec![item2]),
        3 => Some(vec![item1, item2]),
        _ => None,
    }
}

/// For use with `test_semantic_tokens_full`.
#[must_use]
pub fn get_semantic_tokens_full_response(
    response_num: u32,
    uri: &Uri,
) -> Option<SemanticTokensResult> {
    _ = uri;
    let item1 = SemanticToken {
        delta_line: 1,
        delta_start: 2,
        length: 3,
        token_type: 4,
        token_modifiers_bitset: 5,
    };
    let item2 = SemanticToken {
        delta_line: 5,
        delta_start: 7,
        length: 8,
        token_type: 9,
        token_modifiers_bitset: 10,
    };
    match response_num {
        0 => Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: vec![],
        })),
        1 => Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: Some("result_id_1".to_string()),
            data: vec![],
        })),
        2 => Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: vec![item1],
        })),
        3 => Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: Some("result_id_1".to_string()),
            data: vec![item1],
        })),
        4 => Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: vec![item2],
        })),
        5 => Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: Some("result_id_2".to_string()),
            data: vec![item2],
        })),
        6 => Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: vec![item1, item2],
        })),
        7 => Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: Some("result_id_3".to_string()),
            data: vec![item1, item2],
        })),
        8 => Some(SemanticTokensResult::Partial(SemanticTokensPartialResult {
            data: vec![],
        })),
        9 => Some(SemanticTokensResult::Partial(SemanticTokensPartialResult {
            data: vec![item1],
        })),
        10 => Some(SemanticTokensResult::Partial(SemanticTokensPartialResult {
            data: vec![item2],
        })),
        11 => Some(SemanticTokensResult::Partial(SemanticTokensPartialResult {
            data: vec![item1, item2],
        })),
        // NOTE: Because testing `textDocument/semanticTokens/full/delta` relies
        // on getting *some* response for its initial `textDocument/semanticTokens/full`
        // request, we send a valid response even though we don't explicitly test
        // for it
        100..200 => Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: Some("some_result_type".to_string()),
            data: vec![item1],
        })),
        _ => None,
    }
}

/// For use with `test_semantic_tokens_range`.
#[must_use]
pub fn get_semantic_tokens_range_response(
    response_num: u32,
    uri: &Uri,
) -> Option<SemanticTokensRangeResult> {
    _ = uri;
    let item1 = SemanticToken {
        delta_line: 1,
        delta_start: 2,
        length: 3,
        token_type: 4,
        token_modifiers_bitset: 5,
    };
    let item2 = SemanticToken {
        delta_line: 5,
        delta_start: 7,
        length: 8,
        token_type: 9,
        token_modifiers_bitset: 10,
    };
    match response_num {
        0 => Some(SemanticTokensRangeResult::Tokens(SemanticTokens {
            result_id: None,
            data: vec![],
        })),
        1 => Some(SemanticTokensRangeResult::Tokens(SemanticTokens {
            result_id: Some("result_id_1".to_string()),
            data: vec![],
        })),
        2 => Some(SemanticTokensRangeResult::Tokens(SemanticTokens {
            result_id: None,
            data: vec![item1],
        })),
        3 => Some(SemanticTokensRangeResult::Tokens(SemanticTokens {
            result_id: Some("result_id_2".to_string()),
            data: vec![item2],
        })),
        4 => Some(SemanticTokensRangeResult::Tokens(SemanticTokens {
            result_id: Some("result_id_3".to_string()),
            data: vec![item1, item2],
        })),
        5 => Some(SemanticTokensRangeResult::Partial(
            SemanticTokensPartialResult { data: vec![] },
        )),
        6 => Some(SemanticTokensRangeResult::Partial(
            SemanticTokensPartialResult { data: vec![item1] },
        )),
        7 => Some(SemanticTokensRangeResult::Partial(
            SemanticTokensPartialResult { data: vec![item2] },
        )),
        8 => Some(SemanticTokensRangeResult::Partial(
            SemanticTokensPartialResult {
                data: vec![item1, item2],
            },
        )),
        _ => None,
    }
}

/// For use with `test_semantic_tokens_full_delta`.
///
/// Response numbers start at 100 for comaptibility with `test_semantic_tokens_full_response`
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn get_semantic_tokens_full_delta_response(
    response_num: u32,
    uri: &Uri,
) -> Option<SemanticTokensFullDeltaResult> {
    _ = uri;
    let token1 = SemanticToken {
        delta_line: 1,
        delta_start: 2,
        length: 3,
        token_type: 4,
        token_modifiers_bitset: 5,
    };
    let token2 = SemanticToken {
        delta_line: 1,
        delta_start: 2,
        length: 3,
        token_type: 4,
        token_modifiers_bitset: 5,
    };
    let semantic_tokens1 = SemanticTokens {
        result_id: None,
        data: vec![],
    };
    let semantic_tokens2 = SemanticTokens {
        result_id: Some("result_id_1a".to_string()),
        data: vec![],
    };
    let semantic_tokens3 = SemanticTokens {
        result_id: Some("result_id_1a".to_string()),
        data: vec![token1],
    };
    let semantic_tokens4 = SemanticTokens {
        result_id: Some("result_id_1a".to_string()),
        data: vec![token2],
    };
    let semantic_tokens5 = SemanticTokens {
        result_id: Some("result_id_1a".to_string()),
        data: vec![token1, token2],
    };
    let edits1 = SemanticTokensEdit {
        start: 1,
        delete_count: 2,
        data: None,
    };
    let edits2 = SemanticTokensEdit {
        start: 1,
        delete_count: 2,
        data: Some(vec![]),
    };
    let edits3 = SemanticTokensEdit {
        start: 1,
        delete_count: 2,
        data: Some(vec![token1]),
    };
    let edits4 = SemanticTokensEdit {
        start: 1,
        delete_count: 2,
        data: Some(vec![token2]),
    };
    let edits5 = SemanticTokensEdit {
        start: 1,
        delete_count: 2,
        data: Some(vec![token1, token2]),
    };
    match response_num {
        100 => Some(SemanticTokensFullDeltaResult::Tokens(semantic_tokens1)),
        101 => Some(SemanticTokensFullDeltaResult::Tokens(semantic_tokens2)),
        102 => Some(SemanticTokensFullDeltaResult::Tokens(semantic_tokens3)),
        103 => Some(SemanticTokensFullDeltaResult::Tokens(semantic_tokens4)),
        104 => Some(SemanticTokensFullDeltaResult::Tokens(semantic_tokens5)),
        105 => Some(SemanticTokensFullDeltaResult::TokensDelta(
            SemanticTokensDelta {
                result_id: None,
                edits: vec![],
            },
        )),
        106 => Some(SemanticTokensFullDeltaResult::TokensDelta(
            SemanticTokensDelta {
                result_id: Some("result_id_1b".to_string()),
                edits: vec![],
            },
        )),
        107 => Some(SemanticTokensFullDeltaResult::TokensDelta(
            SemanticTokensDelta {
                result_id: Some("result_id_2b".to_string()),
                edits: vec![SemanticTokensEdit {
                    start: 1,
                    delete_count: 2,
                    data: None,
                }],
            },
        )),
        108 => Some(SemanticTokensFullDeltaResult::TokensDelta(
            SemanticTokensDelta {
                result_id: Some("result_id_3b".to_string()),
                edits: vec![SemanticTokensEdit {
                    start: 1,
                    delete_count: 2,
                    data: None,
                }],
            },
        )),
        109 => Some(SemanticTokensFullDeltaResult::TokensDelta(
            SemanticTokensDelta {
                result_id: Some("result_id_4b".to_string()),
                edits: vec![SemanticTokensEdit {
                    start: 3,
                    delete_count: 4,
                    data: Some(vec![]),
                }],
            },
        )),
        110 => Some(SemanticTokensFullDeltaResult::TokensDelta(
            SemanticTokensDelta {
                result_id: Some("result_id_5b".to_string()),
                edits: vec![SemanticTokensEdit {
                    start: 5,
                    delete_count: 6,
                    data: Some(vec![token1]),
                }],
            },
        )),
        111 => Some(SemanticTokensFullDeltaResult::TokensDelta(
            SemanticTokensDelta {
                result_id: Some("result_id_6b".to_string()),
                edits: vec![SemanticTokensEdit {
                    start: 7,
                    delete_count: 8,
                    data: Some(vec![token2]),
                }],
            },
        )),
        112 => Some(SemanticTokensFullDeltaResult::TokensDelta(
            SemanticTokensDelta {
                result_id: Some("result_id_7b".to_string()),
                edits: vec![SemanticTokensEdit {
                    start: 8,
                    delete_count: 9,
                    data: Some(vec![token1, token2]),
                }],
            },
        )),
        113 => Some(SemanticTokensFullDeltaResult::PartialTokensDelta { edits: vec![] }),
        114 => Some(SemanticTokensFullDeltaResult::PartialTokensDelta {
            edits: vec![edits1],
        }),
        115 => Some(SemanticTokensFullDeltaResult::PartialTokensDelta {
            edits: vec![edits2],
        }),
        116 => Some(SemanticTokensFullDeltaResult::PartialTokensDelta {
            edits: vec![edits3],
        }),
        117 => Some(SemanticTokensFullDeltaResult::PartialTokensDelta {
            edits: vec![edits4],
        }),
        118 => Some(SemanticTokensFullDeltaResult::PartialTokensDelta {
            edits: vec![edits5],
        }),
        // This is getting ridiculous...
        119 => Some(SemanticTokensFullDeltaResult::PartialTokensDelta {
            edits: vec![edits1, edits2, edits3, edits4, edits5],
        }),
        _ => None,
    }
}

/// For use with `test_semantic_tokens_range`.
#[must_use]
pub fn get_signature_help_response(response_num: u32, uri: &Uri) -> Option<SignatureHelp> {
    _ = uri;
    match response_num {
        0 => Some(SignatureHelp {
            signatures: vec![],
            active_signature: None,
            active_parameter: None,
        }),
        1 => Some(SignatureHelp {
            signatures: vec![SignatureInformation {
                label: "label1".to_string(),
                documentation: None,
                parameters: None,
                active_parameter: None,
            }],
            active_signature: None,
            active_parameter: None,
        }),
        2 => Some(SignatureHelp {
            signatures: vec![SignatureInformation {
                label: "label2".to_string(),
                documentation: Some(Documentation::String("string documentation".to_string())),
                parameters: Some(vec![]),
                active_parameter: Some(0),
            }],
            active_signature: None,
            active_parameter: None,
        }),
        3 => Some(SignatureHelp {
            signatures: vec![SignatureInformation {
                label: "label3".to_string(),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "markdown documentation".to_string(),
                })),
                parameters: Some(vec![ParameterInformation {
                    label: ParameterLabel::Simple("label".to_string()),
                    documentation: Some(Documentation::String("string documentation".to_string())),
                }]),
                active_parameter: Some(0),
            }],
            active_signature: None,
            active_parameter: None,
        }),
        _ => None,
    }
}

/// For use with `test_declaration`.
///
/// Since `textDocument/definition` and `textDocument/typeDefinition` have the same
/// response, this just wraps `get_definition_response`.
#[must_use]
pub fn get_type_definition_response(
    response_num: u32,
    uri: &Uri,
) -> Option<GotoTypeDefinitionResponse> {
    get_definition_response(response_num, uri)
}

/// For use with `test_folding_range`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_folding_range_response(response_num: u32, uri: &Uri) -> Option<Vec<FoldingRange>> {
    _ = uri;
    let item1 = FoldingRange {
        start_line: 0,
        start_character: None,
        end_line: 1,
        end_character: None,
        kind: None,
        collapsed_text: None,
    };
    let item2 = FoldingRange {
        start_line: 2,
        start_character: Some(3),
        end_line: 4,
        end_character: Some(5),
        kind: None,
        collapsed_text: None,
    };
    let item3 = FoldingRange {
        start_line: 6,
        start_character: Some(7),
        end_line: 8,
        end_character: Some(8),
        kind: Some(FoldingRangeKind::Comment),
        collapsed_text: Some("collapsed text".to_string()),
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

/// For use with `test_formatting`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_formatting_response(response_num: u32, uri: &Uri) -> Option<Vec<TextEdit>> {
    _ = uri;
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
