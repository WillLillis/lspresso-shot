use crate::types::{ServerStartType, TestCase, TestSetupError, TestSetupResult, TestType};

/// Construct the contents of an `init.lua` file to test an lsp request corresponding
/// to `test_type`.
pub fn get_init_dot_lua(
    test_case: &TestCase,
    test_type: TestType,
    custom_replacements: Option<&Vec<(&str, String)>>,
) -> TestSetupResult<String> {
    let results_file_path = test_case.get_results_file_path()?;
    let root_path = test_case.get_lspresso_dir()?;
    let error_path = test_case.get_error_file_path()?;
    let log_path = test_case.get_log_file_path()?;
    let empty_path = test_case.get_empty_file_path()?;
    let source_extension = test_case
        .source_file
        .path
        .extension()
        .ok_or_else(|| {
            // NOTE: use `.unwrap_or("*")` here somehow instead to cover files without extensions?
            TestSetupError::MissingFileExtension(
                test_case.source_file.path.to_string_lossy().to_string(),
            )
        })?
        .to_str()
        .ok_or_else(|| {
            TestSetupError::InvalidFileExtension(
                test_case.source_file.path.to_string_lossy().to_string(),
            )
        })?;
    // Start out with some utilities, adding the relevant filetype, attach logic,
    // and the relevant `check_progress_result` function to invoke our request at
    // the appropriate time
    let mut raw_init = format!(
        "{}{}{}",
        include_str!("lua_templates/helpers.lua"),
        get_attach_action(test_type),
        include_str!("lua_templates/attach.lua"),
    );
    // This is how we actually invoke the action to be tested
    match test_type {
        TestType::CodeLens
        | TestType::CodeLensResolve
        | TestType::Completion
        | TestType::Declaration
        | TestType::Definition
        | TestType::DocumentHighlight
        | TestType::DocumentLink
        | TestType::DocumentLinkResolve
        | TestType::DocumentSymbol
        | TestType::FoldingRange
        | TestType::Formatting
        | TestType::Hover
        | TestType::Implementation
        | TestType::IncomingCalls
        | TestType::OutgoingCalls
        | TestType::PrepareCallHierarchy
        | TestType::References
        | TestType::Rename
        | TestType::TypeDefinition => {
            raw_init = raw_init.replace("LSP_ACTION", &invoke_lsp_action(&test_case.start_type));
        }
        TestType::Diagnostic => {
            // Diagnostics are handled via an autocmd, no need to handle `$/progress`
            raw_init = raw_init.replace("LSP_ACTION", "");
            raw_init.push_str(include_str!("lua_templates/diagnostic_autocmd.lua"));
        }
    };

    if let Some(replacements) = custom_replacements {
        for (from, to) in replacements {
            raw_init = raw_init.replace(from, to);
        }
    }

    let set_cursor_position = test_case.cursor_pos.map_or_else(String::new, |cursor_pos| {
        format!(
            "position = {{ line = {}, character = {} }}",
            cursor_pos.line, cursor_pos.character
        )
    });
    let final_init = raw_init
        .replace("RESULTS_FILE", results_file_path.to_str().unwrap())
        .replace(
            "EXECUTABLE_PATH",
            test_case.executable_path.to_str().unwrap(),
        )
        .replace("ROOT_PATH", root_path.to_str().unwrap())
        .replace("ERROR_PATH", error_path.to_str().unwrap())
        .replace("LOG_PATH", log_path.to_str().unwrap())
        .replace("EMPTY_PATH", empty_path.to_str().unwrap())
        .replace("FILE_EXTENSION", source_extension)
        .replace("SET_CURSOR_POSITION", &set_cursor_position)
        .replace("COMMANDS", "") // clear out commands placeholder if they weren't set by `custom_replacements`
        .replace(
            "PROGRESS_THRESHOLD",
            &progress_threshold(&test_case.start_type),
        )
        .replace(
            "PARENT_PATH",
            test_case
                .get_source_file_path("")
                .unwrap()
                .to_str()
                .unwrap(),
        );

    Ok(final_init)
}

fn progress_threshold(start_type: &ServerStartType) -> String {
    match start_type {
        ServerStartType::Simple => "1".to_string(),
        ServerStartType::Progress(threshold, _) => threshold.to_string(),
    }
}

fn get_attach_action(test_type: TestType) -> String {
    match test_type {
        TestType::CodeLens => include_str!("lua_templates/code_lens_action.lua"),
        TestType::CodeLensResolve => include_str!("lua_templates/code_lens_resolve_action.lua"),
        TestType::Completion => include_str!("lua_templates/completion_action.lua"),
        TestType::Declaration => include_str!("lua_templates/declaration_action.lua"),
        TestType::Definition => include_str!("lua_templates/definition_action.lua"),
        TestType::Diagnostic => "\n-- NOTE: No `check_progress_result` function for diagnostics, instead handled by `DiagnosticChanged` autocmd\n",
        TestType::DocumentHighlight => include_str!("lua_templates/document_highlight_action.lua"),
        TestType::DocumentLink => include_str!("lua_templates/document_link_action.lua"),
        TestType::DocumentLinkResolve => include_str!("lua_templates/document_link_resolve_action.lua"),
        TestType::DocumentSymbol => include_str!("lua_templates/document_symbol_action.lua"),
        TestType::FoldingRange => include_str!("lua_templates/folding_range_action.lua"),
        TestType::Formatting => include_str!("lua_templates/formatting_action.lua"),
        TestType::Hover => include_str!("lua_templates/hover_action.lua"),
        TestType::Implementation => include_str!("lua_templates/implementation_action.lua"),
        TestType::IncomingCalls => include_str!("lua_templates/incoming_calls_action.lua"),
        TestType::OutgoingCalls => include_str!("lua_templates/outgoing_calls_action.lua"),
        TestType::PrepareCallHierarchy => include_str!("lua_templates/prepare_call_hierarchy_action.lua"),
        TestType::References => include_str!("lua_templates/references_action.lua"),
        TestType::Rename => include_str!("lua_templates/rename_action.lua"),
        TestType::TypeDefinition => include_str!("lua_templates/type_definition_action.lua"),
    }
    .to_string()
}

/// In the simple case, the action is invoked immediately. If a server employs
/// some sort of `$/progress` scheme, then we need to check each time the server
/// claims it's ready, respecting the user-set `progress_threshold`
fn invoke_lsp_action(start_type: &ServerStartType) -> String {
    match start_type {
        // Directly invoke the action. Note we unconditionally end the test after the first try
        ServerStartType::Simple => {
            format!("check_progress_result()\n{}vim.cmd('qa!')", " ".repeat(16))
        }
        // Hook into `$/progress` messages
        ServerStartType::Progress(_, token_name) => {
            format!(
                r#"vim.lsp.handlers["$/progress"] = function(_, result, _)
                    if client then
                        if result.value.kind == "end" and result.token == "{token_name}" then
                            client.initialized = true
                            check_progress_result()
                        end
                    end
                end"#
            )
        }
    }
}
