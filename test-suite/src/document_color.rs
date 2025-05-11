#[cfg(test)]
mod test {
    use lspresso_shot::{
        lspresso_shot, test_document_color,
        types::{TestCase, TestFile},
    };
    use std::str::FromStr as _;
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{ColorProviderCapability, ServerCapabilities, Uri};
    use rstest::rstest;

    fn document_color_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            color_provider: Some(ColorProviderCapability::Simple(true)),
            ..ServerCapabilities::default()
        }
    }

    #[rstest]
    fn test_server_simple_expect_some_got_some(#[values(0, 1)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp = test_server::responses::get_document_color_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&document_color_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_document_color(test_case, None, &resp));
    }

    // NOTE: rust-analyzer doesn't support `textDocument/documentColor`
}
