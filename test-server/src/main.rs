use anyhow::Result;
use log::{error, info};
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    notification::{DidOpenTextDocument, Notification as _, PublishDiagnostics},
    request::{
        DocumentDiagnosticRequest, Formatting, GotoDefinition, References, Rename, Request as _,
    },
    DiagnosticOptions, DiagnosticServerCapabilities, InitializeParams, OneOf, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, Uri, WorkDoneProgressOptions,
};
use test_server::responses::{
    get_definition_response, get_diagnostics_response, get_formatting_response,
    get_references_response, get_rename_response,
};

/// Entry point of the lsp server. Connects to the client and enters the main loop
///
/// # Errors
///
/// Returns `Err` if the server fails to connect to the lsp client
///
/// # Panics
///
/// Panics if JSON serialization of the server capabilities fails
pub fn main() -> Result<()> {
    flexi_logger::Logger::try_with_str("info")?.start()?;
    info!("Starting test-server");
    let (connection, _io_threads) = Connection::stdio();

    // Setup capabilities
    let definition_provider = Some(OneOf::Left(true));
    let diagnostic_provider = Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
        identifier: Some("test_server".to_string()),
        inter_file_dependencies: true,
        workspace_diagnostics: true,
        work_done_progress_options: WorkDoneProgressOptions {
            work_done_progress: None,
        },
    }));
    let document_formatting_provider = Some(OneOf::Left(true));
    let references_provider = Some(OneOf::Left(true));
    let rename_provider = Some(OneOf::Left(true));
    // TODO: May need to revisit this later to test other sync kinds, i.e. incremental
    let text_document_sync = Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL));

    let capabilities = ServerCapabilities {
        definition_provider,
        diagnostic_provider,
        document_formatting_provider,
        references_provider,
        rename_provider,
        text_document_sync,
        ..ServerCapabilities::default()
    };
    let server_capabilities = serde_json::to_value(capabilities).unwrap();
    let initialization_params = connection.initialize(server_capabilities)?;

    // TODO: We can get the project's root directory here. If we need to communciate
    // something to the server outside of the request, we can drop in some config file
    // to read

    let params: InitializeParams = serde_json::from_value(initialization_params).unwrap();
    info!("Client initialization params: {:?}", params);

    main_loop(&connection)?;

    // HACK: the `writer` thread of `connection` hangs on joining more often than
    // not. Need to investigate this further, but for now just skipping the join
    // (and thus allowing the process to exit) is fine
    // _io_threads.join()?;

    info!("Shutting down test-server");
    Ok(())
}

fn main_loop(connection: &Connection) -> Result<()> {
    info!("Starting main loop...");
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    info!("Recieved shutdown request");
                    return Ok(());
                }
                handle_request(req, connection)?;
            }
            Message::Notification(notif) => handle_notification(notif, connection)?,
            Message::Response(_resp) => {
                // unimplemented!();
            }
        }
    }
    Ok(())
}

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
/// into its equivalent in memory struct
#[allow(clippy::too_many_lines)]
pub fn handle_request(req: Request, connection: &Connection) -> Result<()> {
    match req.method.as_str() {
        References::METHOD => {
            let (id, params) =
                cast_req::<References>(req).expect("Failed to cast References request");
            info!(
                "Received `{}` request ({id}): {params:?}",
                References::METHOD
            );
            let response_num = params.text_document_position.position.line;
            info!("response_num: {response_num}");
            let Some(resp) = get_references_response(response_num) else {
                error!("Invalid response number: {response_num}");
                return Ok(());
            };
            let result = serde_json::to_value(&resp).unwrap();

            let result = Response {
                id,
                result: Some(result),
                error: None,
            };
            return Ok(connection.sender.send(Message::Response(result))?);
        }
        Formatting::METHOD => {
            let (id, params) =
                cast_req::<Formatting>(req).expect("Failed to cast Formatting request");
            info!(
                "Received `{}` request ({id}): {params:?}",
                Formatting::METHOD
            );
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
            let result = serde_json::to_value(&resp).unwrap();

            let result = Response {
                id,
                result: Some(result),
                error: None,
            };
            return Ok(connection.sender.send(Message::Response(result))?);
        }
        Rename::METHOD => {
            let (id, params) = cast_req::<Rename>(req).expect("Failed to cast Rename request");
            info!("Received `{}` request ({id}): {params:?}", Rename::METHOD);
            // `response_num` passed via `params.new_name`
            let Ok(response_num) = params.new_name.parse() else {
                error!(
                    "Failed to parse `new_name` as `response_num`: {}",
                    params.new_name
                );
                return Ok(());
            };
            info!("response_num: {response_num}");
            let Some(resp) = get_rename_response(response_num) else {
                error!("Invalid response number: {response_num}");
                return Ok(());
            };
            let result = serde_json::to_value(&resp).unwrap();

            let result = Response {
                id,
                result: Some(result),
                error: None,
            };
            return Ok(connection.sender.send(Message::Response(result))?);
        }
        GotoDefinition::METHOD => {
            let (id, params) =
                cast_req::<GotoDefinition>(req).expect("Failed to cast GotoDefinition request");
            info!(
                "Received `{}` request ({id}): {params:?}",
                GotoDefinition::METHOD
            );
            let response_num = params.text_document_position_params.position.line;
            info!("response_num: {response_num}");
            let Some(resp) = get_definition_response(response_num) else {
                error!("Invalid response number: {response_num}");
                return Ok(());
            };
            let result = serde_json::to_value(&resp).unwrap();

            let result = Response {
                id,
                result: Some(result),
                error: None,
            };
            return Ok(connection.sender.send(Message::Response(result))?);
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
        method => error!("Unimplemented request format: {method:?}\n{req:?}"),
    }

    Ok(())
}

fn handle_notification(notif: Notification, connection: &Connection) -> Result<()> {
    match notif.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let did_open_params = cast_notif::<DidOpenTextDocument>(notif)?;
            info!(
                "Received `{}` notification: {did_open_params:?}",
                DidOpenTextDocument::METHOD
            );
            send_diagnostic_resp(&did_open_params.text_document.uri, connection)?;
        }
        method => error!("Unimplemented notification format: {method:?}\n{notif:?}"),
    }
    Ok(())
}

// TODO: Move this once we refactor
fn send_diagnostic_resp(uri: &Uri, connection: &Connection) -> Result<()> {
    let publish_params = get_diagnostics_response(uri);
    let result = serde_json::to_value(&publish_params).unwrap();

    let notif = Notification {
        method: PublishDiagnostics::METHOD.to_string(),
        params: result,
    };

    Ok(connection.sender.send(Message::Notification(notif))?)
}
