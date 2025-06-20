use std::str::FromStr;

use anyhow::Result;
use log::{error, info};
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    CallHierarchyIncomingCallsParams, CallHierarchyOutgoingCallsParams, CallHierarchyPrepareParams,
    CodeAction, CodeActionParams, CodeLens, CodeLensParams, ColorPresentationParams,
    CompletionItem, CompletionParams, CreateFilesParams, DeleteFilesParams, DocumentColorParams,
    DocumentDiagnosticParams, DocumentFormattingParams, DocumentHighlightParams, DocumentLink,
    DocumentLinkParams, DocumentOnTypeFormattingParams, DocumentRangeFormattingParams,
    DocumentSymbolParams, ExecuteCommandParams, FoldingRangeParams, GotoDefinitionParams,
    HoverParams, InlayHintParams, LinkedEditingRangeParams, MonikerParams, OneOf, ReferenceParams,
    RenameFilesParams, RenameParams, SelectionRangeParams, SemanticTokensDeltaParams,
    SemanticTokensParams, SemanticTokensRangeParams, ServerCapabilities, SignatureHelpParams,
    TextDocumentPositionParams, TypeHierarchyPrepareParams, Uri, WorkspaceDiagnosticParams,
    WorkspaceSymbol, WorkspaceSymbolParams,
    notification::{DidOpenTextDocument, Notification as _, PublishDiagnostics},
    request::{
        CallHierarchyIncomingCalls, CallHierarchyOutgoingCalls, CallHierarchyPrepare,
        CodeActionRequest, CodeActionResolveRequest, CodeLensRequest, CodeLensResolve,
        ColorPresentationRequest, Completion, DocumentColor, DocumentDiagnosticRequest,
        DocumentHighlightRequest, DocumentLinkRequest, DocumentLinkResolve, DocumentSymbolRequest,
        ExecuteCommand, FoldingRangeRequest, Formatting, GotoDeclaration, GotoDeclarationParams,
        GotoDefinition, GotoImplementation, GotoImplementationParams, GotoTypeDefinition,
        GotoTypeDefinitionParams, HoverRequest, InlayHintRequest, LinkedEditingRange,
        MonikerRequest, OnTypeFormatting, PrepareRenameRequest, RangeFormatting, References,
        Rename, Request as _, ResolveCompletionItem, SelectionRangeRequest,
        SemanticTokensFullDeltaRequest, SemanticTokensFullRequest, SemanticTokensRangeRequest,
        SignatureHelpRequest, TypeHierarchyPrepare, WillCreateFiles, WillDeleteFiles,
        WillRenameFiles, WorkspaceDiagnosticRequest, WorkspaceSymbolRequest,
        WorkspaceSymbolResolve,
    },
};

use crate::{
    get_root_test_path, receive_response_num,
    responses::{
        get_code_action_resolve_response, get_code_action_response, get_code_lens_resolve_response,
        get_code_lens_response, get_color_presentation_response, get_completion_resolve_response,
        get_completion_response, get_declaration_response, get_definition_response,
        get_diagnostic_response, get_document_color_response, get_document_highlight_response,
        get_document_link_resolve_response, get_document_link_response,
        get_document_symbol_response, get_execute_command_response, get_folding_range_response,
        get_formatting_range_response, get_formatting_response, get_hover_response,
        get_implementation_response, get_incoming_calls_response, get_inlay_hint_response,
        get_linked_editing_range_response, get_moniker_response, get_on_type_formatting_response,
        get_outgoing_calls_response, get_prepare_call_hierachy_response,
        get_prepare_rename_response, get_prepare_type_hierachy_response,
        get_publish_diagnostics_response, get_references_response, get_rename_response,
        get_selection_range_response, get_semantic_tokens_full_delta_response,
        get_semantic_tokens_full_response, get_semantic_tokens_range_response,
        get_signature_help_response, get_type_definition_response,
        get_workspace_diagnostics_response, get_workspace_symbol_resolve_response,
        get_workspace_symbol_response, get_workspace_will_create_files_response,
        get_workspace_will_delete_files_response,
    },
};

fn cast_req<R>(req: Request) -> Result<(RequestId, R::Params)>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    match req.extract(R::METHOD) {
        Ok(value) => Ok(value),
        Err(e) => Err(anyhow::anyhow!("Error: {e}")),
    }
}

fn cast_notif<R>(notif: Notification) -> Result<R::Params>
where
    R: lsp_types::notification::Notification,
    R::Params: serde::de::DeserializeOwned,
{
    match notif.extract(R::METHOD) {
        Ok(value) => Ok(value),
        Err(e) => Err(anyhow::anyhow!("Error: {e}")),
    }
}

fn send_req_resp<R>(id: RequestId, resp: Option<R>, connection: &Connection) -> Result<()>
where
    R: serde::ser::Serialize + std::fmt::Debug,
{
    info!("Sending response for request {id}: {resp:#?}");
    let result = serde_json::to_value(resp).unwrap();
    let result = Response {
        id,
        result: Some(result),
        error: None,
    };
    Ok(connection.sender.send(Message::Response(result))?)
}

/// Handles `Notification`s from the lsp client.
///
/// # Errors
///
/// Returns errors from any of the handler functions. The majority of error sources
/// are failures to send a response via `connection`.
///
/// # Panics
///
/// Panics if JSON encoding of a response fails or if a json request fails to cast
/// into its equivalent in-memory struct.
pub fn handle_notification(notif: Notification, connection: &Connection) -> Result<()> {
    match notif.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let did_open_params = cast_notif::<DidOpenTextDocument>(notif)?;
            info!(
                "Received `{}` notification: {did_open_params:?}",
                DidOpenTextDocument::METHOD
            );
            send_diagnostic_resp(&did_open_params.text_document.uri, connection)?;
        }
        method => error!("Unimplemented notification method: {method:?}\n{notif:?}"),
    }
    Ok(())
}

/// Sends a `textDocument/publishDiagnostic` notification to the client.
///
/// # Errors
///
/// Returns `Err` if sending the notification fails.
///
/// # Panics
///
/// Panics if serialization of `PublishDiagnosticsParams` fails.
pub fn send_diagnostic_resp(uri: &Uri, connection: &Connection) -> Result<()> {
    let Some(root_path) = get_root_test_path(uri) else {
        error!(
            "Failed to retrieve root path from provided uri: {}",
            uri.as_str()
        );
        return Ok(());
    };
    let response_num = receive_response_num(&root_path)?;
    info!("response_num: {response_num}");
    let Some(publish_params) = get_publish_diagnostics_response(response_num, uri) else {
        error!("Invalid response number: {response_num}");
        return Ok(());
    };
    info!("Sending diagnostics: {publish_params:?}");
    let result = serde_json::to_value(&publish_params).unwrap();

    let notif = Notification {
        method: PublishDiagnostics::METHOD.to_string(),
        params: result,
    };

    Ok(connection.sender.send(Message::Notification(notif))?)
}

macro_rules! handle_request {
    ($request_type:ty, $resp_getter:expr, $req:expr, $connection:expr, $extract_uri:expr) => {{
        let (id, params) = cast_req::<$request_type>($req).expect(concat!(
            "Failed to cast `",
            stringify!($request_type),
            "` request"
        ));
        info!(
            "Received `{}` request ({id}): {params:?}",
            <$request_type>::METHOD
        );
        let uri = $extract_uri(params);
        let Some(root_path) = get_root_test_path(&uri) else {
            error!(
                "Failed to retrieve root path from provided uri: {}",
                uri.as_str()
            );
            return Ok(());
        };
        let response_num = receive_response_num(&root_path)?;
        info!("response_num: {response_num}");

        let resp = $resp_getter(response_num, &uri);
        send_req_resp(id, resp, $connection)
    }};
}

/// Handles `Request`s from the lsp client.
///
/// # Errors
///
/// Returns errors from any of the handler functions. The majority of error sources
/// are failures to send a response via `connection`.
///
/// # Panics
///
/// Panics if JSON encoding of a response fails or if a json request fails to cast
/// into its equivalent in-memory struct.
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
pub fn handle_request(
    req: Request,
    _capabilities: &ServerCapabilities, // TODO: Use once we have more capabilities tested
    conn: &Connection,
) -> Result<()> {
    // TODO: Probably check capabilities here and do some progress reporting before
    // and after handling the request, maybe implement other behaviors
    match req.method.as_str() {
        CallHierarchyIncomingCalls::METHOD => {
            handle_request!(
                CallHierarchyIncomingCalls,
                get_incoming_calls_response,
                req,
                conn,
                |params: CallHierarchyIncomingCallsParams| -> Uri { params.item.uri }
            )?;
        }
        CallHierarchyOutgoingCalls::METHOD => {
            handle_request!(
                CallHierarchyOutgoingCalls,
                get_outgoing_calls_response,
                req,
                conn,
                |params: CallHierarchyOutgoingCallsParams| -> Uri { params.item.uri }
            )?;
        }
        CallHierarchyPrepare::METHOD => {
            handle_request!(
                CallHierarchyPrepare,
                get_prepare_call_hierachy_response,
                req,
                conn,
                |params: CallHierarchyPrepareParams| -> Uri {
                    params.text_document_position_params.text_document.uri
                }
            )?;
        }
        CodeActionRequest::METHOD => {
            handle_request!(
                CodeActionRequest,
                get_code_action_response,
                req,
                conn,
                |params: CodeActionParams| -> Uri { params.text_document.uri }
            )?;
        }
        CodeActionResolveRequest::METHOD => {
            handle_request!(
                CodeActionResolveRequest,
                get_code_action_resolve_response,
                req,
                conn,
                |params: CodeAction| -> Uri {
                    let data = params.data.unwrap();
                    let raw_uri = data.get("uri").unwrap().as_str().unwrap();
                    Uri::from_str(raw_uri).unwrap()
                }
            )?;
        }
        CodeLensRequest::METHOD => {
            handle_request!(
                CodeLensRequest,
                get_code_lens_response,
                req,
                conn,
                |params: CodeLensParams| -> Uri { params.text_document.uri }
            )?;
        }
        CodeLensResolve::METHOD => {
            handle_request!(
                CodeLensResolve,
                get_code_lens_resolve_response,
                req,
                conn,
                |params: CodeLens| -> Uri {
                    let data = params.data.unwrap();
                    let raw_uri = data.get("uri").unwrap().as_str().unwrap();
                    Uri::from_str(raw_uri).unwrap()
                }
            )?;
        }
        ColorPresentationRequest::METHOD => {
            handle_request!(
                ColorPresentationRequest,
                get_color_presentation_response,
                req,
                conn,
                |params: ColorPresentationParams| -> Uri { params.text_document.uri }
            )?;
        }
        Completion::METHOD => {
            handle_request!(
                Completion,
                get_completion_response,
                req,
                conn,
                |params: CompletionParams| -> Uri {
                    params.text_document_position.text_document.uri
                }
            )?;
        }
        ResolveCompletionItem::METHOD => {
            handle_request!(
                ResolveCompletionItem,
                get_completion_resolve_response,
                req,
                conn,
                |params: CompletionItem| -> Uri {
                    let data = params.data.unwrap();
                    let raw_uri = data.get("uri").unwrap().as_str().unwrap();
                    Uri::from_str(raw_uri).unwrap()
                }
            )?;
        }
        DocumentDiagnosticRequest::METHOD => {
            handle_request!(
                DocumentDiagnosticRequest,
                get_diagnostic_response,
                req,
                conn,
                |params: DocumentDiagnosticParams| -> Uri { params.text_document.uri }
            )?;
        }
        DocumentColor::METHOD => {
            handle_request!(
                DocumentColor,
                get_document_color_response,
                req,
                conn,
                |params: DocumentColorParams| -> Uri { params.text_document.uri }
            )?;
        }
        DocumentHighlightRequest::METHOD => {
            handle_request!(
                DocumentHighlightRequest,
                get_document_highlight_response,
                req,
                conn,
                |params: DocumentHighlightParams| -> Uri {
                    params.text_document_position_params.text_document.uri
                }
            )?;
        }
        DocumentLinkRequest::METHOD => {
            handle_request!(
                DocumentLinkRequest,
                get_document_link_response,
                req,
                conn,
                |params: DocumentLinkParams| -> Uri { params.text_document.uri }
            )?;
        }
        DocumentLinkResolve::METHOD => {
            handle_request!(
                DocumentLinkResolve,
                get_document_link_resolve_response,
                req,
                conn,
                |params: DocumentLink| -> Uri { params.target.unwrap() }
            )?;
        }
        DocumentSymbolRequest::METHOD => {
            handle_request!(
                DocumentSymbolRequest,
                get_document_symbol_response,
                req,
                conn,
                |params: DocumentSymbolParams| -> Uri { params.text_document.uri }
            )?;
        }
        ExecuteCommand::METHOD => {
            handle_request!(
                ExecuteCommand,
                get_execute_command_response,
                req,
                conn,
                |params: ExecuteCommandParams| -> Uri {
                    let raw_uri = params.arguments[0].as_str().unwrap();
                    Uri::from_str(raw_uri).unwrap()
                }
            )?;
        }
        FoldingRangeRequest::METHOD => {
            handle_request!(
                FoldingRangeRequest,
                get_folding_range_response,
                req,
                conn,
                |params: FoldingRangeParams| -> Uri { params.text_document.uri }
            )?;
        }
        Formatting::METHOD => {
            handle_request!(
                Formatting,
                get_formatting_response,
                req,
                conn,
                |params: DocumentFormattingParams| -> Uri { params.text_document.uri }
            )?;
        }
        RangeFormatting::METHOD => {
            handle_request!(
                RangeFormatting,
                get_formatting_range_response,
                req,
                conn,
                |params: DocumentRangeFormattingParams| -> Uri { params.text_document.uri }
            )?;
        }
        OnTypeFormatting::METHOD => {
            handle_request!(
                OnTypeFormatting,
                get_on_type_formatting_response,
                req,
                conn,
                |params: DocumentOnTypeFormattingParams| -> Uri {
                    params.text_document_position.text_document.uri
                }
            )?;
        }
        GotoDeclaration::METHOD => {
            handle_request!(
                GotoDeclaration,
                get_declaration_response,
                req,
                conn,
                |params: GotoDeclarationParams| -> Uri {
                    params.text_document_position_params.text_document.uri
                }
            )?;
        }
        GotoDefinition::METHOD => {
            handle_request!(
                GotoDefinition,
                get_definition_response,
                req,
                conn,
                |params: GotoDefinitionParams| -> Uri {
                    params.text_document_position_params.text_document.uri
                }
            )?;
        }
        GotoImplementation::METHOD => {
            handle_request!(
                GotoImplementation,
                get_implementation_response,
                req,
                conn,
                |params: GotoImplementationParams| -> Uri {
                    params.text_document_position_params.text_document.uri
                }
            )?;
        }
        GotoTypeDefinition::METHOD => {
            handle_request!(
                GotoTypeDefinition,
                get_type_definition_response,
                req,
                conn,
                |params: GotoTypeDefinitionParams| -> Uri {
                    params.text_document_position_params.text_document.uri
                }
            )?;
        }
        HoverRequest::METHOD => {
            handle_request!(
                HoverRequest,
                get_hover_response,
                req,
                conn,
                |params: HoverParams| -> Uri {
                    params.text_document_position_params.text_document.uri
                }
            )?;
        }
        InlayHintRequest::METHOD => {
            handle_request!(
                InlayHintRequest,
                get_inlay_hint_response,
                req,
                conn,
                |params: InlayHintParams| -> Uri { params.text_document.uri }
            )?;
        }
        LinkedEditingRange::METHOD => {
            handle_request!(
                LinkedEditingRange,
                get_linked_editing_range_response,
                req,
                conn,
                |params: LinkedEditingRangeParams| -> Uri {
                    params.text_document_position_params.text_document.uri
                }
            )?;
        }
        MonikerRequest::METHOD => {
            handle_request!(
                MonikerRequest,
                get_moniker_response,
                req,
                conn,
                |params: MonikerParams| -> Uri {
                    params.text_document_position_params.text_document.uri
                }
            )?;
        }
        References::METHOD => {
            handle_request!(
                References,
                get_references_response,
                req,
                conn,
                |params: ReferenceParams| -> Uri {
                    params.text_document_position.text_document.uri
                }
            )?;
        }
        Rename::METHOD => {
            handle_request!(
                Rename,
                get_rename_response,
                req,
                conn,
                |params: RenameParams| -> Uri { params.text_document_position.text_document.uri }
            )?;
        }
        PrepareRenameRequest::METHOD => {
            handle_request!(
                PrepareRenameRequest,
                get_prepare_rename_response,
                req,
                conn,
                |params: TextDocumentPositionParams| -> Uri { params.text_document.uri }
            )?;
        }
        SelectionRangeRequest::METHOD => {
            handle_request!(
                SelectionRangeRequest,
                get_selection_range_response,
                req,
                conn,
                |params: SelectionRangeParams| -> Uri { params.text_document.uri }
            )?;
        }
        SemanticTokensFullRequest::METHOD => {
            handle_request!(
                SemanticTokensFullRequest,
                get_semantic_tokens_full_response,
                req,
                conn,
                |params: SemanticTokensParams| -> Uri { params.text_document.uri }
            )?;
        }
        SemanticTokensFullDeltaRequest::METHOD => {
            handle_request!(
                SemanticTokensFullDeltaRequest,
                get_semantic_tokens_full_delta_response,
                req,
                conn,
                |params: SemanticTokensDeltaParams| -> Uri { params.text_document.uri }
            )?;
        }
        SemanticTokensRangeRequest::METHOD => {
            handle_request!(
                SemanticTokensRangeRequest,
                get_semantic_tokens_range_response,
                req,
                conn,
                |params: SemanticTokensRangeParams| -> Uri { params.text_document.uri }
            )?;
        }
        SignatureHelpRequest::METHOD => {
            handle_request!(
                SignatureHelpRequest,
                get_signature_help_response,
                req,
                conn,
                |params: SignatureHelpParams| -> Uri {
                    params.text_document_position_params.text_document.uri
                }
            )?;
        }
        TypeHierarchyPrepare::METHOD => {
            handle_request!(
                TypeHierarchyPrepare,
                get_prepare_type_hierachy_response,
                req,
                conn,
                |params: TypeHierarchyPrepareParams| -> Uri {
                    params.text_document_position_params.text_document.uri
                }
            )?;
        }
        WorkspaceDiagnosticRequest::METHOD => {
            handle_request!(
                WorkspaceDiagnosticRequest,
                get_workspace_diagnostics_response,
                req,
                conn,
                |params: WorkspaceDiagnosticParams| -> Uri {
                    let raw_uri = params.identifier.unwrap();
                    Uri::from_str(&raw_uri).unwrap()
                }
            )?;
        }
        WorkspaceSymbolRequest::METHOD => {
            handle_request!(
                WorkspaceSymbolRequest,
                get_workspace_symbol_response,
                req,
                conn,
                |params: WorkspaceSymbolParams| -> Uri { Uri::from_str(&params.query).unwrap() }
            )?;
        }
        WorkspaceSymbolResolve::METHOD => {
            handle_request!(
                WorkspaceSymbolResolve,
                get_workspace_symbol_resolve_response,
                req,
                conn,
                |params: WorkspaceSymbol| -> Uri {
                    match params.location {
                        OneOf::Left(location) => location.uri,
                        OneOf::Right(workspace_location) => workspace_location.uri,
                    }
                }
            )?;
        }
        WillCreateFiles::METHOD => {
            handle_request!(
                WillCreateFiles,
                get_workspace_will_create_files_response,
                req,
                conn,
                |params: CreateFilesParams| -> Uri { Uri::from_str(&params.files[0].uri).unwrap() }
            )?;
        }
        WillDeleteFiles::METHOD => {
            handle_request!(
                WillDeleteFiles,
                get_workspace_will_delete_files_response,
                req,
                conn,
                |params: DeleteFilesParams| -> Uri { Uri::from_str(&params.files[0].uri).unwrap() }
            )?;
        }
        WillRenameFiles::METHOD => {
            handle_request!(
                WillRenameFiles,
                get_workspace_will_create_files_response,
                req,
                conn,
                |params: RenameFilesParams| -> Uri {
                    Uri::from_str(&params.files[0].old_uri).unwrap()
                }
            )?;
        }
        method => error!("Unimplemented request method: {method:?}\n{req:?}"),
    }

    Ok(())
}
