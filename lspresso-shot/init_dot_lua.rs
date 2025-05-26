use lsp_types::{Position, Range};
use std::fmt::Write;

use crate::types::{ServerStartType, TestCase, TestSetupError, TestSetupResult, TestType};

/// Construct the contents of an `init.lua` file to test an lsp request corresponding
/// to `test_type`.
pub fn get_init_dot_lua(
    test_case: &TestCase,
    test_type: TestType,
    replacements: &mut Vec<LuaReplacement>,
) -> TestSetupResult<String> {
    replacements.extend(get_standard_replacements(test_case, test_type)?);
    let mut raw_init = include_str!("lua_templates/helpers.lua").to_string();
    raw_init.push_str(match test_type {
        TestType::PublishDiagnostics => include_str!("lua_templates/diagnostic_autocmd.lua"),
        TestType::Formatting | TestType::WorkspaceExecuteCommand => {
            include_str!("lua_templates/state_or_response_action.lua")
        }
        TestType::SemanticTokensFullDelta => {
            include_str!("lua_templates/semantic_tokens_full_delta_action.lua")
        }
        _ => include_str!("lua_templates/request_action.lua"),
    });
    raw_init.push_str(include_str!("lua_templates/attach.lua"));
    // This is how we get neovim to actually invoke the action to be tested
    raw_init = match test_type {
        // Diagnostics are handled via an autocmd, no need to hook into `$/progress`
        TestType::PublishDiagnostics => raw_init.replace("LSP_ACTION", ""),
        _ => raw_init.replace("LSP_ACTION", &invoke_lsp_action(&test_case.start_type)),
    };
    let replacement_set = LuaDocumentReplacement::new(replacements);
    let final_init = replacement_set.fill_document(raw_init);

    Ok(final_init)
}

/// Replacements common to all/nearly all test types.
fn get_standard_replacements(
    test_case: &TestCase,
    test_type: TestType,
) -> TestSetupResult<Vec<LuaReplacement>> {
    let mut replacements = Vec::with_capacity(14);
    let results_file_path = test_case.get_results_file_path()?;
    let root_path = test_case.get_lspresso_dir()?;
    let error_path = test_case.get_error_file_path()?;
    let log_path = test_case.get_log_file_path()?;
    let empty_path = test_case.get_empty_file_path()?;
    let benchmark_path = test_case.get_benchmark_file_path()?;
    let source_extension = test_case
        .source_file
        .path
        .extension()
        .ok_or_else(|| {
            // TODO: use `.unwrap_or("*")` here somehow instead to cover files without extensions?
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
    replacements.push(LuaReplacement::Other {
        from: "REQUEST_METHOD",
        to: test_type.to_string(),
    });
    replacements.push(LuaReplacement::Other {
        from: "RESULTS_FILE",
        to: results_file_path.to_str().unwrap().to_string(),
    });
    replacements.push(LuaReplacement::Other {
        from: "EXECUTABLE_PATH",
        to: test_case.executable_path.to_str().unwrap().to_string(),
    });
    replacements.push(LuaReplacement::Other {
        from: "ROOT_PATH",
        to: root_path.to_str().unwrap().to_string(),
    });
    replacements.push(LuaReplacement::Other {
        from: "ERROR_PATH",
        to: error_path.to_str().unwrap().to_string(),
    });
    replacements.push(LuaReplacement::Other {
        from: "LOG_PATH",
        to: log_path.to_str().unwrap().to_string(),
    });
    replacements.push(LuaReplacement::Other {
        from: "EMPTY_PATH",
        to: empty_path.to_str().unwrap().to_string(),
    });
    replacements.push(LuaReplacement::Other {
        from: "BENCHMARK_PATH",
        to: benchmark_path.to_str().unwrap().to_string(),
    });
    replacements.push(LuaReplacement::Other {
        from: "FILE_EXTENSION",
        to: source_extension.to_string(),
    });
    replacements.push(LuaReplacement::Other {
        from: "COMMANDS",
        to: String::new(),
    });
    replacements.push(LuaReplacement::Other {
        from: "PROGRESS_THRESHOLD",
        to: progress_threshold(&test_case.start_type),
    });
    replacements.push(LuaReplacement::Other {
        from: "PARENT_PATH",
        to: test_case
            .get_source_file_path("")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string(),
    });
    replacements.push(LuaReplacement::Other {
        from: "TIMEOUT_PATH",
        to: test_case
            .get_timeout_file_path()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string(),
    });
    replacements.push(LuaReplacement::Other {
        from: "TIMEOUT_MS",
        to: test_case.timeout.as_millis().to_string(),
    });
    Ok(replacements)
}

fn progress_threshold(start_type: &ServerStartType) -> String {
    match start_type {
        ServerStartType::Simple => "1".to_string(),
        ServerStartType::Progress(threshold, _) => threshold.to_string(),
    }
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

/// The type of replacement to be made in the `init.lua` file. Several common replacements
/// are specified in their own variants, while others are specified via fields.
#[derive(Debug, Clone)]
pub enum LuaReplacement {
    /// `textDocument = vim.lsp.util.make_text_document_params(0)`
    ParamTextDocument,
    /// `<name> = { line = <line>, character = <character> }`
    /// If `name` is `None`, the default name of `position` is used.
    ParamPosition {
        pos: Position,
        name: Option<&'static str>,
    },
    /// Equivalent of `range = { start = { line = <start-line>, character = <start-character> }, ["end"] = { line = <end-line>, character = <end-character> } }`
    ParamRange(Range),
    /// An object that is converted to JSON in order to pass to the lua side. This
    /// object can be inserted directly into `params`.
    ParamDirect { name: &'static str, json: String },
    /// An object that is converted to JSON in order to pass to the lua side. After
    /// conversion from JSON into a lua table, each field is inserted into `params`
    /// individually.
    ParamDestructure {
        name: &'static str,
        fields: Vec<&'static str>,
        json: String,
    },
    /// An object that needs to contain other nested objects. Each replacement in
    /// `fields` is stored in a parent object `name`.
    ParamNested {
        name: &'static str,
        fields: Vec<LuaReplacement>,
    },
    /// Performs raw string substitution on the lua file. These subsituions are
    /// made before any other type to prevent conflicts with user-supplied values.
    Other { from: &'static str, to: String },
}

impl LuaReplacement {
    fn perform_replacement(&self, doc: &mut LuaDocumentReplacement, parent_name: Option<&str>) {
        let parent_name = parent_name.unwrap_or("params");
        match self {
            Self::ParamTextDocument => {
                writeln!(
                    &mut doc.params,
                    "\tassert(not {parent_name}['textDocument'], \"{parent_name}['textDocument'] already set\")
\t{parent_name}['textDocument'] = vim.lsp.util.make_text_document_params(0)"
                )
                .unwrap();
            }
            Self::ParamPosition { pos, name } => {
                let name = name.unwrap_or("position");
                writeln!(
                    &mut doc.params,
                    "\tassert(not {parent_name}['{name}'], \"{parent_name}['{name}'] already set\")
\t{parent_name}['{name}'] = {{ line = {}, character = {} }}",
                    pos.line, pos.character
                )
                .unwrap();
            }
            Self::ParamRange(range) => {
                let range = Self::ParamNested {
                    name: "range",
                    fields: vec![
                        Self::ParamPosition {
                            pos: range.start,
                            name: Some("start"),
                        },
                        Self::ParamPosition {
                            pos: range.end,
                            name: Some("end"),
                        },
                    ],
                };
                range.perform_replacement(doc, Some(parent_name));
            }
            Self::ParamDirect { name, json } => {
                writeln!(
                    &mut doc.params,
                    "\tlocal {name}_json = [[\n{json}\n]]
\tassert(not {parent_name}['{name}'], \"{parent_name}['{name}'] already set\")
\t{parent_name}['{name}'] = vim.json.decode({name}_json)"
                )
                .unwrap();
            }
            Self::ParamDestructure { name, fields, json } => {
                writeln!(&mut doc.params, "\tlocal {name}_json = [[\n{json}\n]]\n\tlocal {name} = vim.json.decode({name}_json)").unwrap();
                for field in fields {
                    writeln!(
                        &mut doc.params,
                        "\tassert(not {parent_name}['{field}'], \"{parent_name}['{field}'] already set\")
\t{parent_name}['{field}'] = {name}['{field}']"
                    )
                    .unwrap();
                }
            }
            Self::ParamNested { name, fields } => {
                writeln!(
                    &mut doc.params,
                    "\tassert(not {parent_name}['{name}'], \"{parent_name}['{name}'] already set\")"
                )
                .unwrap();
                writeln!(&mut doc.params, "\tlocal {name} = {{}}").unwrap();
                for field in fields {
                    field.perform_replacement(doc, Some(name));
                }
                writeln!(&mut doc.params, "\t{parent_name}['{name}'] = {name}").unwrap();
            }
            Self::Other { from, to } => doc.raw.push(((*from).to_string(), to.to_string())),
        }
    }
}

/// Represents the combined replacements from a series of `LuaReplacementType`s.
/// This type can be applied to the raw `init.lua` template to produce a valid
/// lua file that can be passed to neovim.
#[derive(Debug, Clone, Default)]
struct LuaDocumentReplacement {
    /// Represents objects that are passed via a JSON string, converted to a lua
    /// table, and inserted into `params`.
    pub params: String,
    /// Represents raw string replacements anywhere in the init.lua file.
    pub raw: Vec<(String, String)>,
}

impl LuaDocumentReplacement {
    fn new(repls: &Vec<LuaReplacement>) -> Self {
        let mut doc_repl = Self::default();
        for repl in repls {
            repl.perform_replacement(&mut doc_repl, None);
        }
        doc_repl
    }

    pub fn fill_document(&self, mut doc: String) -> String {
        for (repl_from, repl_to) in &self.raw {
            doc = doc.replace(repl_from, repl_to);
        }
        doc.replace("PARAM_ASSIGN", &self.params)
    }
}

#[cfg(test)]
mod test {
    use lsp_types::{CodeLens, Position, Range};

    use super::{LuaDocumentReplacement, LuaReplacement};

    #[test]
    fn text_document_param() {
        let replacements = vec![LuaReplacement::ParamTextDocument];
        let doc_repl = LuaDocumentReplacement::new(&replacements);
        let expected =
            "\tassert(not params['textDocument'], \"params['textDocument'] already set\")
\tparams['textDocument'] = vim.lsp.util.make_text_document_params(0)\n";
        assert_eq!(expected, doc_repl.params);
        assert!(doc_repl.raw.is_empty());
    }

    #[test]
    fn position_param() {
        let replacements = vec![LuaReplacement::ParamPosition {
            pos: Position {
                line: 1,
                character: 2,
            },
            name: None,
        }];
        let doc_repl = LuaDocumentReplacement::new(&replacements);
        let expected = "\tassert(not params['position'], \"params['position'] already set\")
\tparams['position'] = { line = 1, character = 2 }\n";
        assert_eq!(expected, doc_repl.params);
        assert!(doc_repl.raw.is_empty());
    }

    #[test]
    fn range_param() {
        let replacements = vec![LuaReplacement::ParamRange(Range {
            start: Position::new(1, 2),
            end: Position::new(3, 4),
        })];
        let doc_repl = LuaDocumentReplacement::new(&replacements);
        let expected = "\tassert(not params['range'], \"params['range'] already set\")
\tlocal range = {}\n\tassert(not range['start'], \"range['start'] already set\")
\trange['start'] = { line = 1, character = 2 }
\tassert(not range['end'], \"range['end'] already set\")
\trange['end'] = { line = 3, character = 4 }
\tparams['range'] = range\n";
        assert_eq!(expected, doc_repl.params);
        assert!(doc_repl.raw.is_empty());
    }

    #[test]
    fn param_direct() {
        let position = Position::new(1, 2);
        let position_json = serde_json::to_string(&position).expect("Failed to serialize position");
        let replacements = vec![LuaReplacement::ParamDirect {
            name: "position",
            json: position_json.clone(),
        }];
        let doc_repl = LuaDocumentReplacement::new(&replacements);
        let expected = format!(
            "\tlocal position_json = [[\n{position_json}\n]]
\tassert(not params['position'], \"params['position'] already set\")
\tparams['position'] = vim.json.decode(position_json)\n"
        );
        assert_eq!(expected, doc_repl.params);
        assert!(doc_repl.raw.is_empty());
    }

    #[test]
    fn param_destructure() {
        let code_lens = CodeLens {
            range: Range::default(),
            command: None,
            data: None,
        };
        let code_lens_json =
            serde_json::to_string(&code_lens).expect("Failed to serialize code lens");
        let replacements = vec![LuaReplacement::ParamDestructure {
            name: "code_lens",
            fields: vec!["range", "data", "command"],
            json: code_lens_json.clone(),
        }];
        let doc_repl = LuaDocumentReplacement::new(&replacements);
        let expected = format!(
            "\tlocal code_lens_json = [[\n{code_lens_json}\n]]
\tlocal code_lens = vim.json.decode(code_lens_json)
\tassert(not params['range'], \"params['range'] already set\")
\tparams['range'] = code_lens['range']
\tassert(not params['data'], \"params['data'] already set\")
\tparams['data'] = code_lens['data']
\tassert(not params['command'], \"params['command'] already set\")
\tparams['command'] = code_lens['command']\n"
        );
        assert_eq!(expected, doc_repl.params);
        assert!(doc_repl.raw.is_empty());
    }

    #[test]
    fn param_nested() {
        let include_decl_json = serde_json::to_string_pretty(&true)
            .expect("JSON deserialzation of include declaration failed");
        let replacements = vec![LuaReplacement::ParamNested {
            name: "context",
            fields: vec![LuaReplacement::ParamDirect {
                name: "includeDeclaration",
                json: include_decl_json.clone(),
            }],
        }];
        let doc_repl = LuaDocumentReplacement::new(&replacements);
        let expected = format!(
            "\tassert(not params['context'], \"params['context'] already set\")
\tlocal context = {{}}
\tlocal includeDeclaration_json = [[\n{include_decl_json}\n]]
\tassert(not context['includeDeclaration'], \"context['includeDeclaration'] already set\")
\tcontext['includeDeclaration'] = vim.json.decode(includeDeclaration_json)
\tparams['context'] = context\n"
        );
        assert_eq!(expected, doc_repl.params);
        assert!(doc_repl.raw.is_empty());
    }

    #[test]
    fn other() {
        let command_str = "\"rust-analyzer.runSingle\",
\"rust-analyzer.debugSingle\",
\"rust-analyzer.showReferences\",
\"rust-analyzer.gotoLocation\",
";
        let replacements = vec![LuaReplacement::Other {
            from: "commands",
            to: command_str.to_string(),
        }];
        let doc_repl = LuaDocumentReplacement::new(&replacements);
        assert!(doc_repl.params.is_empty());
        assert_eq!(1, doc_repl.raw.len());
        let raw = doc_repl.raw.first().unwrap();
        assert_eq!("commands", raw.0);
        assert_eq!(command_str, raw.1);
    }
}
