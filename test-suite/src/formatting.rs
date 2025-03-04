#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, time::Duration};

    use crate::test_helpers::cargo_dot_toml;
    use lspresso_shot::{
        lspresso_shot, test_formatting,
        types::{FormattingResult, ServerStartType, TestCase, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{OneOf, Position, Range, ServerCapabilities, TextEdit};
    use rstest::rstest;

    fn formatting_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            document_formatting_provider: Some(OneOf::Left(true)),
            ..Default::default()
        }
    }

    #[test]
    fn test_server_formatting_state_simple() {
        let contents = "Some source contents";
        let source_file = TestFile::new(test_server::get_source_path(), contents);
        let test_case = TestCase::new(get_dummy_server_path(), source_file).cleanup(false);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        // NOTE: Sending a `response_num` of 0 indicates an empty edit response
        send_response_num(0, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&formatting_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_formatting(
            test_case,
            None,
            &FormattingResult::EndState(contents.to_string())
        ));
    }

    #[rstest]
    fn test_server_formatting_response_simple(#[values(1, 2, 3)] response_num: u32) {
        let edits = test_server::responses::get_formatting_response(response_num).unwrap();
        let source_file = TestFile::new(test_server::get_source_path(), "Some source contents");
        let test_case = TestCase::new(get_dummy_server_path(), source_file).cleanup(false);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&formatting_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_formatting(
            test_case,
            None,
            &FormattingResult::Response(edits)
        ));
    }

    #[test]
    fn rust_analyzer_formatting_state() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
let foo = 5;
}",
        );
        let test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(1).unwrap(),
                "rustAnalyzer/Indexing".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .cleanup(false)
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_formatting(
            test_case,
            None,
            &FormattingResult::EndState(
                "pub fn main() {
    let foo = 5;
}
"
                .to_string()
            )
        ));
    }

    #[test]
    fn rust_analyzer_formatting_response() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
let foo = 5;
}",
        );
        let test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(1).unwrap(),
                "rustAnalyzer/Indexing".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .cleanup(false)
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_formatting(
            test_case,
            None,
            &FormattingResult::Response(vec![
                TextEdit {
                    new_text: "    ".to_string(),
                    range: Range {
                        start: Position::new(1, 0),
                        end: Position::new(1, 0),
                    },
                },
                TextEdit {
                    new_text: "\n".to_string(),
                    range: Range {
                        start: Position::new(2, 1),
                        end: Position::new(2, 1),
                    }
                }
            ]),
        ));
    }
}
