use anyhow::Result;
use log::{error, info};
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    notification::{DidOpenTextDocument, Notification as _, PublishDiagnostics},
    request::{
        CallHierarchyIncomingCalls, CallHierarchyPrepare, Completion, DocumentDiagnosticRequest,
        DocumentSymbolRequest, Formatting, GotoDeclaration, GotoDeclarationParams, GotoDefinition,
        GotoImplementation, GotoImplementationParams, GotoTypeDefinition, GotoTypeDefinitionParams,
        HoverRequest, References, Rename, Request as _,
    },
    CallHierarchyIncomingCallsParams, CallHierarchyPrepareParams, CompletionParams,
    DocumentFormattingParams, DocumentSymbolParams, GotoDefinitionParams, HoverParams,
    ReferenceParams, RenameParams, ServerCapabilities, Uri,
};

use crate::{
    get_root_test_path, receive_response_num,
    responses::{
        get_completion_response, get_declaration_response, get_definition_response,
        get_diagnostics_response, get_document_symbol_response, get_formatting_response,
        get_hover_response, get_implementation_response, get_incoming_calls_response,
        get_prepare_call_hierachy_response, get_references_response, get_rename_response,
        get_type_definition_response,
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
    let Some(publish_params) = get_diagnostics_response(response_num, uri) else {
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
    ($request_type:ty, $handler:expr, $req:expr, $connection:expr) => {
        let (id, params) = cast_req::<$request_type>($req).expect(concat!(
            "Failed to cast `",
            stringify!($request_type),
            "` request"
        ));
        info!(
            "Received `{}` request ({id}): {params:?}",
            <$request_type>::METHOD
        );
        $handler(id, &params, $connection)?;
    };
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
pub fn handle_request(
    req: Request,
    _capabilities: &ServerCapabilities, // TODO: Use once we have more capabilities tested
    conn: &Connection,
) -> Result<()> {
    match req.method.as_str() {
        CallHierarchyIncomingCalls::METHOD => {
            handle_request!(CallHierarchyIncomingCalls, incoming_calls, req, conn);
        }
        CallHierarchyPrepare::METHOD => {
            handle_request!(CallHierarchyPrepare, prepare_call_hierarchy, req, conn);
        }
        Completion::METHOD => {
            handle_request!(Completion, handle_completion, req, conn);
        }
        DocumentDiagnosticRequest::METHOD => {
            let (id, params) = cast_req::<DocumentDiagnosticRequest>(req)
                .expect("Failed to cast `DocumentDiagnosticRequest` request");
            info!(
                "Received `{}` request ({id}): {params:?}",
                DocumentDiagnosticRequest::METHOD
            );
            send_diagnostic_resp(&params.text_document.uri, conn)?;
        }
        DocumentSymbolRequest::METHOD => {
            handle_request!(DocumentSymbolRequest, document_symbol, req, conn);
        }
        Formatting::METHOD => {
            handle_request!(Formatting, formatting, req, conn);
        }
        GotoDeclaration::METHOD => {
            handle_request!(GotoDeclaration, declaration, req, conn);
        }
        GotoDefinition::METHOD => {
            handle_request!(GotoDefinition, definition, req, conn);
        }
        GotoImplementation::METHOD => {
            handle_request!(GotoImplementation, implementation, req, conn);
        }
        GotoTypeDefinition::METHOD => {
            handle_request!(GotoTypeDefinition, type_definition, req, conn);
        }
        HoverRequest::METHOD => {
            handle_request!(HoverRequest, hover, req, conn);
        }
        References::METHOD => {
            handle_request!(References, references, req, conn);
        }
        Rename::METHOD => {
            handle_request!(Rename, rename, req, conn);
        }
        method => error!("Unimplemented request method: {method:?}\n{req:?}"),
    }

    Ok(())
}

// TODO: Pull out common handler logic into a macro
// This will get a little more complicated once we start checking capabilities
// at runtime, but I think that should be doable with a closure parameter

/// Sends response to a `textDocument/completion` request.
///
/// # Errors
///
/// Returns `Err` if receiving the response nummber from the test case or sending
/// the response to  the server fails.
fn handle_completion(
    id: RequestId,
    params: &CompletionParams,
    connection: &Connection,
) -> Result<()> {
    let uri = &params.text_document_position.text_document.uri;
    let Some(root_path) = get_root_test_path(uri) else {
        error!(
            "Failed to retrieve root path from provided uri: {}",
            uri.as_str()
        );
        return Ok(());
    };
    let response_num = receive_response_num(&root_path)?;

    info!("response_num: {response_num}");
    let resp = get_completion_response(response_num);
    send_req_resp(id, resp, connection)
}

/// Sends response to a `textDocument/declaration` request.
///
/// # Errors
///
/// Returns `Err` if receiving the response nummber from the test case or sending
/// the response to  the server fails.
fn declaration(
    id: RequestId,
    params: &GotoDeclarationParams,
    connection: &Connection,
) -> Result<()> {
    let uri = &params.text_document_position_params.text_document.uri;
    let Some(root_path) = get_root_test_path(uri) else {
        error!(
            "Failed to retrieve root path from provided uri: {}",
            uri.as_str()
        );
        return Ok(());
    };
    let response_num = receive_response_num(&root_path)?;
    info!("response_num: {response_num}");
    let resp = get_declaration_response(response_num);
    send_req_resp(id, resp, connection)
}

/// Sends response to a `textDocument/definition` request.
///
/// # Errors
///
/// Returns `Err` if receiving the response nummber from the test case or sending
/// the response to  the server fails.
fn definition(id: RequestId, params: &GotoDefinitionParams, connection: &Connection) -> Result<()> {
    let uri = &params.text_document_position_params.text_document.uri;
    let Some(root_path) = get_root_test_path(uri) else {
        error!(
            "Failed to retrieve root path from provided uri: {}",
            uri.as_str()
        );
        return Ok(());
    };
    let response_num = receive_response_num(&root_path)?;
    info!("response_num: {response_num}");
    let resp = get_definition_response(response_num);
    send_req_resp(id, resp, connection)
}

/// Sends response to a `textDocument/documentSymbol` request.
///
/// # Errors
///
/// Returns `Err` if receiving the response nummber from the test case or sending
/// the response to  the server fails.
fn document_symbol(
    id: RequestId,
    params: &DocumentSymbolParams,
    connection: &Connection,
) -> Result<()> {
    let uri = &params.text_document.uri;
    let Some(root_path) = get_root_test_path(uri) else {
        error!(
            "Failed to retrieve root path from provided uri: {}",
            uri.as_str()
        );
        return Ok(());
    };
    let response_num = receive_response_num(&root_path)?;
    info!("response_num: {response_num}");
    let resp = get_document_symbol_response(response_num);
    send_req_resp(id, resp, connection)
}

/// Sends response to a `textDocument/formatting` request.
///
/// # Errors
///
/// Returns `Err` if receiving the response nummber from the test case or sending
/// the response to  the server fails.
fn formatting(
    id: RequestId,
    params: &DocumentFormattingParams,
    connection: &Connection,
) -> Result<()> {
    let uri = &params.text_document.uri;
    let Some(root_path) = get_root_test_path(uri) else {
        error!(
            "Failed to retrieve root path from provided uri: {}",
            uri.as_str()
        );
        return Ok(());
    };
    let response_num = receive_response_num(&root_path)?;

    info!("response_num: {response_num}");
    let resp = get_formatting_response(response_num);
    send_req_resp(id, resp, connection)
}

/// Sends response to a `textDocument/hover` request.
///
/// # Errors
///
/// Returns `Err` if receiving the response nummber from the test case or sending
/// the response to  the server fails.
fn hover(
    id: RequestId,
    params: &HoverParams,
    // capabilities: &ServerCapabilities, // TODO: Once we add more capabilities coverage
    connection: &Connection,
) -> Result<()> {
    let uri = &params.text_document_position_params.text_document.uri;
    let Some(root_path) = get_root_test_path(uri) else {
        error!(
            "Failed to retrieve root path from provided uri: {}",
            uri.as_str()
        );
        return Ok(());
    };
    let response_num = receive_response_num(&root_path)?;

    // let is_progress = matches!(
    //     capabilities.hover_provider,
    //     Some(HoverProviderCapability::Options(HoverOptions {
    //         work_done_progress_options: WorkDoneProgressOptions {
    //             work_done_progress: Some(true),
    //         },
    //     }))
    // );
    // if is_progress {
    //     // TODO: Send a few mock progress responses before sending the data
    // }

    info!("response_num: {response_num}");
    let resp = get_hover_response(response_num);
    send_req_resp(id, resp, connection)?;

    // if is_progress {
    //     // TODO: Send a progress done messages here
    // }
    Ok(())
}

/// Sends response to a `textDocument/implementation` request.
///
/// # Errors
///
/// Returns `Err` if receiving the response nummber from the test case or sending
/// the response to  the server fails.
fn implementation(
    id: RequestId,
    params: &GotoImplementationParams,
    connection: &Connection,
) -> Result<()> {
    let uri = &params.text_document_position_params.text_document.uri;
    let Some(root_path) = get_root_test_path(uri) else {
        error!(
            "Failed to retrieve root path from provided uri: {}",
            uri.as_str()
        );
        return Ok(());
    };
    let response_num = receive_response_num(&root_path)?;
    info!("response_num: {response_num}");
    let resp = get_implementation_response(response_num);
    send_req_resp(id, resp, connection)
}

/// Sends response to a `callHierarchy/incomingCalls` request.
///
/// # Errors
///
/// Returns `Err` if receiving the response nummber from the test case or sending
/// the response to  the server fails.
fn incoming_calls(
    id: RequestId,
    params: &CallHierarchyIncomingCallsParams,
    connection: &Connection,
) -> Result<()> {
    let uri = &params.item.uri;
    let Some(root_path) = get_root_test_path(uri) else {
        error!(
            "Failed to retrieve root path from provided uri: {}",
            uri.as_str()
        );
        return Ok(());
    };
    let response_num = receive_response_num(&root_path)?;
    info!("response_num: {response_num}");
    let resp = get_incoming_calls_response(response_num);
    send_req_resp(id, resp, connection)
}

/// Sends response to a `textDocument/prepareCallHierarchy` request.
///
/// # Errors
///
/// Returns `Err` if receiving the response nummber from the test case or sending
/// the response to  the server fails.
fn prepare_call_hierarchy(
    id: RequestId,
    params: &CallHierarchyPrepareParams,
    connection: &Connection,
) -> Result<()> {
    let uri = &params.text_document_position_params.text_document.uri;
    let Some(root_path) = get_root_test_path(uri) else {
        error!(
            "Failed to retrieve root path from provided uri: {}",
            uri.as_str()
        );
        return Ok(());
    };
    let response_num = receive_response_num(&root_path)?;
    info!("response_num: {response_num}");
    let resp = get_prepare_call_hierachy_response(response_num);
    send_req_resp(id, resp, connection)
}

/// Sends a `textDocument/references` response to the client.
///
/// # Errors
///
/// Returns `Err` if receiving the response nummber from the test case or sending
/// the response to  the server fails.
fn references(id: RequestId, params: &ReferenceParams, connection: &Connection) -> Result<()> {
    let uri = &params.text_document_position.text_document.uri;
    let Some(root_path) = get_root_test_path(uri) else {
        error!(
            "Failed to retrieve root path from provided uri: {}",
            uri.as_str()
        );
        return Ok(());
    };
    let response_num = receive_response_num(&root_path)?;
    info!("response_num: {response_num}");
    let resp = get_references_response(response_num);
    send_req_resp(id, resp, connection)
}

/// Sends response to a `textDocument/rename` request.
///
/// # Errors
///
/// Returns `Err` if receiving the response nummber from the test case or sending
/// the response to  the server fails.
fn rename(id: RequestId, params: &RenameParams, connection: &Connection) -> Result<()> {
    let uri = &params.text_document_position.text_document.uri;
    let Some(root_path) = get_root_test_path(uri) else {
        error!(
            "Failed to retrieve root path from provided uri: {}",
            uri.as_str()
        );
        return Ok(());
    };
    let response_num = receive_response_num(&root_path)?;
    info!("response_num: {response_num}");
    let resp = get_rename_response(response_num);
    send_req_resp(id, resp, connection)
}

/// Sends response to a `textDocument/typeDefinition` request.
///
/// # Errors
///
/// Returns `Err` if receiving the response nummber from the test case or sending
/// the response to  the server fails.
fn type_definition(
    id: RequestId,
    params: &GotoTypeDefinitionParams,
    connection: &Connection,
) -> Result<()> {
    let uri = &params.text_document_position_params.text_document.uri;
    let Some(root_path) = get_root_test_path(uri) else {
        error!(
            "Failed to retrieve root path from provided uri: {}",
            uri.as_str()
        );
        return Ok(());
    };
    let response_num = receive_response_num(&root_path)?;
    info!("response_num: {response_num}");
    let resp = get_type_definition_response(response_num);
    send_req_resp(id, resp, connection)
}
