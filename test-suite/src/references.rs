#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, str::FromStr as _, time::Duration};

    use crate::test_helpers::cargo_dot_toml;
    use lspresso_shot::{
        lspresso_shot, test_references,
        types::{ServerStartType, TestCase, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{Location, OneOf, Position, Range, ServerCapabilities, Uri};
    use rstest::rstest;

    fn references_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            references_provider: Some(OneOf::Left(true)),
            ..ServerCapabilities::default()
        }
    }

    #[rstest]
    fn test_server_references_simple(#[values(1, 2, 3)] response_num: u32) {
        let refs = test_server::responses::get_references_response(response_num).unwrap();
        let source_file = TestFile::new(test_server::get_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file)
            .cursor_pos(Some(Position::default()));
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&references_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_references(test_case, true, &refs,));
    }

    #[test]
    fn rust_analyzer_references() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
    let foo = 5;
}",
        );
        let reference_test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(5).unwrap(),
                "rustAnalyzer/Indexing".to_string(),
            ))
            .cursor_pos(Some(Position::new(1, 9)))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_references(
            reference_test_case,
            true,
            &vec![Location {
                uri: Uri::from_str("src/main.rs").unwrap(),
                range: Range {
                    start: Position::new(1, 8),
                    end: Position::new(1, 11)
                },
            }]
        ));
    }
}
