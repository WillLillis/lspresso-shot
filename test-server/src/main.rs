use anyhow::Result;
use log::{error, info};
use lsp_server::{Connection, Message, Request, RequestId, Response};
use lsp_types::{
    request::{References, Request as _},
    InitializeParams, OneOf, ServerCapabilities,
};
use test_server::responses::get_references_response;

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
    let references_provider = Some(OneOf::Left(true));

    let capabilities = ServerCapabilities {
        references_provider,
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
            Message::Notification(_notif) => {
                unimplemented!();
                // handle_notification(notif, connection)?;
            }
            Message::Response(_resp) => {
                unimplemented!();
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

/// Handles `Request`s from the lsp client. By convention, the `response_num` value
/// specifying which pre-determined response to send back is taken from the first line
/// number in `params`, if available
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

            let result = Response {
                id,
                result: Some(result),
                error: None,
            };
            return Ok(connection.sender.send(Message::Response(result))?);
        }
        method => error!("Unimplemented request format: {method:?}\n{req:?}"),
    }

    Ok(())
}
