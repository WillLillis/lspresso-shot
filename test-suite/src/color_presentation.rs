#[cfg(test)]
mod test {
    use lspresso_shot::{
        lspresso_shot, test_color_presentation,
        types::{TestCase, TestFile},
    };
    use std::str::FromStr as _;
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{ColorProviderCapability, ServerCapabilities, Uri};
    use rstest::rstest;

    fn color_provider_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            color_provider: Some(ColorProviderCapability::Simple(true)),
            ..ServerCapabilities::default()
        }
    }

    #[rstest]
    fn test_server_simple_expect_some_got_some(#[values(0, 1, 2, 3, 4)] response_num: u32) {
        use lsp_types::{Color, Range};

        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp =
            test_server::responses::get_color_presentation_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&color_provider_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        let color = Color {
            red: 0.0,
            green: 0.0,
            blue: 0.0,
            alpha: 0.0,
        };

        lspresso_shot!(test_color_presentation(
            &test_case,
            color,
            Range::default(),
            None,
            &resp
        ));
    }

    // NOTE: rust-analyzer doesn't support `textDocument/colorPresentation`
}
