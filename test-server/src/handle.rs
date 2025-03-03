use anyhow::Result;
use log::{error, info};
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    notification::{DidOpenTextDocument, Notification as _, PublishDiagnostics},
    request::{
        Completion, DocumentDiagnosticRequest, DocumentSymbolRequest, Formatting, GotoDefinition,
        HoverRequest, References, Rename, Request as _,
    },
    CompletionParams, DocumentFormattingParams, DocumentSymbolParams, GotoDefinitionParams,
    HoverParams, ReferenceParams, RenameParams, Uri,
};

use crate::{
    get_root_test_path, receive_response_num,
    responses::{
        get_completion_response, get_definition_response, get_diagnostics_response,
        get_document_symbol_response, get_formatting_response, get_hover_response,
        get_references_response, get_rename_response,
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

fn send_req_resp<R>(id: RequestId, resp: R, connection: &Connection) -> Result<()>
where
    R: serde::ser::Serialize,
{
    let result = serde_json::to_value(&resp).unwrap();
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
/// are failures to send a response via `connection`
///
/// # Panics
///
/// Panics if JSON encoding of a response fails or if a json request fails to cast
/// into its equivalent in-memory struct
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

/// Handles `Request`s from the lsp client.
///
/// By convention, the `response_num` value specifying which pre-determined response
/// to send back is taken from the first line number in `params`, if available.
/// Data passed via other means will be specified in a comment
///
/// # Errors
///
/// Returns errors from any of the handler functions. The majority of error sources
/// are failures to send a response via `connection`
///
/// # Panics
///
/// Panics if JSON encoding of a response fails or if a json request fails to cast
/// into its equivalent in-memory struct
pub fn handle_request(req: Request, connection: &Connection) -> Result<()> {
    match req.method.as_str() {
        References::METHOD => {
            let (id, params) =
                cast_req::<References>(req).expect("Failed to cast References request");
            info!(
                "Received `{}` request ({id}): {params:?}",
                References::METHOD
            );
            return handle_references(id, &params, connection);
        }
        Formatting::METHOD => {
            let (id, params) =
                cast_req::<Formatting>(req).expect("Failed to cast Formatting request");
            info!(
                "Received `{}` request ({id}): {params:?}",
                Formatting::METHOD
            );
            return handle_formatting(id, &params, connection);
        }
        Rename::METHOD => {
            let (id, params) = cast_req::<Rename>(req).expect("Failed to cast Rename request");
            info!("Received `{}` request ({id}): {params:?}", Rename::METHOD);
            return handle_rename(id, &params, connection);
        }
        GotoDefinition::METHOD => {
            let (id, params) =
                cast_req::<GotoDefinition>(req).expect("Failed to cast GotoDefinition request");
            info!(
                "Received `{}` request ({id}): {params:?}",
                GotoDefinition::METHOD
            );
            handle_definition(id, &params, connection)?;
        }
        DocumentDiagnosticRequest::METHOD => {
            let (id, params) = cast_req::<DocumentDiagnosticRequest>(req)
                .expect("Failed to cast DocumentDiagnosticRequest request");
            info!(
                "Received `{}` request ({id}): {params:?}",
                DocumentDiagnosticRequest::METHOD
            );
            send_diagnostic_resp(&params.text_document.uri, connection)?;
        }
        HoverRequest::METHOD => {
            let (id, params) =
                cast_req::<HoverRequest>(req).expect("Failed to cast `HoverRequest` request");
            info!(
                "Received `{}` request ({id}): {params:?}",
                HoverRequest::METHOD
            );
            handle_hover(id, &params, connection)?;
        }
        Completion::METHOD => {
            let (id, params) =
                cast_req::<Completion>(req).expect("Failed to cast `Completion` request");
            info!(
                "Received `{}` request ({id}): {params:?}",
                Completion::METHOD
            );
            handle_completion(id, &params, connection)?;
        }
        DocumentSymbolRequest::METHOD => {
            let (id, params) = cast_req::<DocumentSymbolRequest>(req)
                .expect("Failed to cast `Completion` request");
            info!(
                "Received `{}` request ({id}): {params:?}",
                Completion::METHOD
            );
            handle_document_symbol(id, &params, connection)?;
        }
        method => error!("Unimplemented request method: {method:?}\n{req:?}"),
    }

    Ok(())
}

/// Sends response to a `textDocument/completion` request
///
/// # Errors
///
/// Returns `Err` if sending the response fails.
///
/// # Panics
///
/// Panics if serialization of `params` fails.
fn handle_document_symbol(
    id: RequestId,
    params: &DocumentSymbolParams,
    connection: &Connection,
) -> Result<()> {
    let Some(root_path) = get_root_test_path(&params.text_document.uri) else {
        error!(
            "Failed to retrieve root path from provided uri: {}",
            params.text_document.uri.as_str()
        );
        return Ok(());
    };
    let response_num = receive_response_num(&root_path)?;
    info!("response_num: {response_num}");
    let Some(resp) = get_document_symbol_response(response_num) else {
        error!("Invalid response number: {response_num}");
        return Ok(());
    };
    send_req_resp(id, resp, connection)
}

/// Sends response to a `textDocument/completion` request
///
/// # Errors
///
/// Returns `Err` if sending the response fails.
///
/// # Panics
///
/// Panics if serialization of `params` fails.
fn handle_completion(
    id: RequestId,
    params: &CompletionParams,
    connection: &Connection,
) -> Result<()> {
    let response_num = params.text_document_position.position.line;
    info!("response_num: {response_num}");
    let Some(resp) = get_completion_response(response_num) else {
        error!("Invalid response number: {response_num}");
        return Ok(());
    };
    send_req_resp(id, resp, connection)
}

/// Sends response to a `textDocument/hover` request
///
/// # Errors
///
/// Returns `Err` if sending the response fails.
///
/// # Panics
///
/// Panics if serialization of `params` fails.
fn handle_hover(id: RequestId, params: &HoverParams, connection: &Connection) -> Result<()> {
    let response_num = params.text_document_position_params.position.line;
    info!("response_num: {response_num}");
    let Some(resp) = get_hover_response(response_num) else {
        error!("Invalid response number: {response_num}");
        return Ok(());
    };
    send_req_resp(id, resp, connection)
}

/// Sends response to a `textDocument/definition` request
///
/// # Errors
///
/// Returns `Err` if sending the response fails.
///
/// # Panics
///
/// Panics if serialization of `params` fails.
fn handle_definition(
    id: RequestId,
    params: &GotoDefinitionParams,
    connection: &Connection,
) -> Result<()> {
    let response_num = params.text_document_position_params.position.line;
    info!("response_num: {response_num}");
    let Some(resp) = get_definition_response(response_num) else {
        error!("Invalid response number: {response_num}");
        return Ok(());
    };
    send_req_resp(id, resp, connection)
}

/// Sends response to a `textDocument/rename` request
///
/// # Errors
///
/// Returns `Err` if sending the response fails.
///
/// # Panics
///
/// Panics if serialization of `params` fails.
fn handle_rename(id: RequestId, params: &RenameParams, connection: &Connection) -> Result<()> {
    // `response_num` passed via `params.new_name`
    let Ok(response_num) = params.new_name.parse() else {
        error!(
            "Failed to parse `new_name` as `response_num`: {}",
            params.new_name
        );
        return Ok(());
    };
    let Some(resp) = get_rename_response(response_num) else {
        error!("Invalid response number: {response_num}");
        return Ok(());
    };
    send_req_resp(id, resp, connection)
}

/// Sends response to a `textDocument/formatting` request
///
/// # Errors
///
/// Returns `Err` if sending the response fails.
///
/// # Panics
///
/// Panics if serialization of `params` fails.
fn handle_formatting(
    id: RequestId,
    params: &DocumentFormattingParams,
    connection: &Connection,
) -> Result<()> {
    // `response_num` passed via `params.options.tab_size`
    let response_num = params.options.tab_size;
    info!("response_num: {response_num}");
    let resp = get_formatting_response(response_num).map_or_else(
        || {
            // In this case, we wish to test `FormattingResponse::EndState`
            // Send a  reply with no edits (so the start and end state of
            // the file matches) to the client so it knows we got the request
            // and proceeds with the comparison
            info!("Sending response for `FormattingResponse::EndState`");
            Vec::new()
        },
        |resp| {
            info!("Sending response for `FormattingResponse::Response`");
            resp
        },
    );
    send_req_resp(id, resp, connection)
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
    let publish_params = get_diagnostics_response(uri);
    let result = serde_json::to_value(&publish_params).unwrap();

    let notif = Notification {
        method: PublishDiagnostics::METHOD.to_string(),
        params: result,
    };

    Ok(connection.sender.send(Message::Notification(notif))?)
}

/// Sends a `textDocument/references` response to the client.
///
/// # Errors
///
/// Retruns `Err` if sending the response fails.
///
/// # Panics
///
/// Panics if serialization of `Vec<Location>` fails.
fn handle_references(
    id: RequestId,
    params: &ReferenceParams,
    connection: &Connection,
) -> Result<()> {
    let response_num = params.text_document_position.position.line;
    info!("response_num: {response_num}");
    let Some(resp) = get_references_response(response_num) else {
        error!("Invalid response number: {response_num}");
        return Ok(());
    };
    send_req_resp(id, resp, connection)
}
