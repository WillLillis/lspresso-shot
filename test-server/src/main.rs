use std::{
    path::{Path, PathBuf},
    str::FromStr as _,
};

use test_server::handle::{handle_notification, handle_request};

use anyhow::{anyhow, Result};
use log::{error, info};
use lsp_server::{Connection, Message};
use lsp_types::{InitializeParams, ServerCapabilities};

fn get_capabilities(path: &Path) -> Result<ServerCapabilities> {
    let capabilities_json = std::fs::read_to_string(path)?;
    let capabilities: ServerCapabilities = serde_json::from_str(&capabilities_json)?;

    Ok(capabilities)
}

// Yoinked from asm-lsp
/// Attempts to find the project's root directory given its `InitializeParams`
// 1. if we have workspace folders, then iterate through them and assign the first valid one to
//    the root path
// 2. If we don't have worksace folders or none of them is a valid path, check the (deprecated)
//    root_uri field
// 3. If both workspace folders and root_uri didn't provide a path, check the (deprecated)
//    root_path field
fn get_project_root(params: &InitializeParams) -> Option<PathBuf> {
    // First check workspace folders
    if let Some(folders) = &params.workspace_folders {
        // If there's multiple, just visit in order until we find a valid folder
        for folder in folders {
            let Ok(parsed) = PathBuf::from_str(folder.uri.path().as_str());
            if let Ok(parsed_path) = parsed.canonicalize() {
                info!("Detected project root: {}", parsed_path.display());
                return Some(parsed_path);
            }
        }
    }

    // If workspace folders weren't set or came up empty, we check the root_uri
    #[allow(deprecated)]
    if let Some(root_uri) = &params.root_uri {
        let Ok(parsed) = PathBuf::from_str(root_uri.path().as_str());
        if let Ok(parsed_path) = parsed.canonicalize() {
            info!("Detected project root: {}", parsed_path.display());
            return Some(parsed_path);
        }
    }

    // If both `workspace_folders` and `root_uri` weren't set or came up empty, we check the root_path
    #[allow(deprecated)]
    if let Some(root_path) = &params.root_path {
        let Ok(parsed) = PathBuf::from_str(root_path.as_str());
        if let Ok(parsed_path) = parsed.canonicalize() {
            info!("Detected project root: {}", parsed_path.display());
            return Some(parsed_path);
        }
    }

    error!("Failed to detect project root");
    None
}

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

    info!("Initializing test-server");
    let (id, init_params) = connection.initialize_start()?;
    let init_params: InitializeParams = serde_json::from_value(init_params).unwrap();
    info!("Client initialization params: {init_params:?}");
    let Some(root_path) = get_project_root(&init_params) else {
        return Err(anyhow!("Failed to detect project root"));
    };
    // Invariant: The `src` directory passed to the test server as the root path
    // should always be contained within an lspresso-shot test case directory
    let mut capabilities_path = root_path.parent().unwrap().to_path_buf();
    capabilities_path.push("capabilities.json");
    let server_capabilities = get_capabilities(&capabilities_path)?;
    info!("Server capabilities: {server_capabilities:?}");
    let initialize_data = serde_json::json!({
        "capabilities": server_capabilities,
        "serverInfo": {
            "name": "test-server",
            "version": "0.1.0",
        },
    });
    connection.initialize_finish(id, initialize_data)?;
    info!("Initialization complete");

    main_loop(&connection, &server_capabilities)?;

    // HACK: the `writer` thread of `connection` hangs on joining more often than
    // not. Need to investigate this further, but for now just skipping the join
    // (and thus allowing the process to exit) is fine
    // _io_threads.join()?;

    info!("Shutting down test-server");
    Ok(())
}

/// The test server's main loop.
fn main_loop(connection: &Connection, capabilities: &ServerCapabilities) -> Result<()> {
    info!("Starting main loop...");
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                handle_request(req, capabilities, connection)?;
            }
            Message::Notification(notif) => handle_notification(notif, connection)?,
            Message::Response(resp) => error!("Unimplemented response received: {resp:?}"),
        }
    }
    Ok(())
}
