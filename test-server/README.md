# Test Server

This is a bare-bones LSP server implementation for the purposes of testing
lspresso-shot. As more is added, this README can serve as basic documentation
for contributors.

## Usage

The test server is intended to be used in conjunction with test cases defined in
`test_suite/src/*`. Make sure to run `cargo build --workspace` after making any changes
within the `test-server` directory.

### Responses

All responses made by the test server are defined as function in `src/responses.rs`.
The `responses` module is public, which allows the test cases and server to share
a common definition from a single source. All response functions take in a `response_num`
parameter, which allows for different responses to be returned as desired by the
test case. The test case communicates the `response_num` to the server via `send_response_num`,
which simply writes `response_num` to `RESPONSE_NUM.txt` in the test case's root.
This file is read by the server after it has received a request, returning the corresponding
data as appropriate.

### Capabilities

Test cases define the necessary capabilities for it to pass. These capabilities
are passed to `send_capabilities()`. These capabilities are serialized to JSON,
and written to `capabilities.json` in the test case's root. At start up time,
the server reads this file, deserializes its contents, and sends them to the relevant
LSP client. The server is structured in this way in order to allow the testing of
multiple server configurations with only one implementation. When incorrect capabilities
are passed, the test usually fails with a timeout failure.
