use anyhow::Result;
use log::info;
use lsp_server::{Connection, Message};
use lsp_types::{
    DiagnosticOptions, DiagnosticServerCapabilities, InitializeParams, OneOf, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, WorkDoneProgressOptions,
};
use test_server::handle::{handle_notification, handle_request};

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
