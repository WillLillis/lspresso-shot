# LSPresso-Shot

A concentrated dose of LSP testing power!

## Goal

Provide an easy way to perform integration tests on language servers implemented in Rust.

## Usage

First, add lspresso-shot as a dependency to your Rust project:

```shell
cargo add --dev lspresso-shot
```

Write a test:

```rust
#[test]
fn it_does_the_hover_thing() {
    let hover_test_case = TestCase::new(
        "Path to server",
        TestFile::new("Source file name", "Contents")
    )
    .other_file( // Optional
        TestFile::new("Other file name", "Other contents")
    );

    let cursor_pos = Position::new(1, 2);
    lspresso_shot!(test_hover(
        hover_test_case,
        &cursor_pos,
        Some(&Hover {
            range: Some(Range {
                start: lsp_types::Position {
                    line: 1,
                    character: 2,
                },
                end: lsp_types::Position {
                    line: 3,
                    character: 4,
                },
            }),
            contents: lsp_types::HoverContents::Markup(MarkupContent {
                kind: lsp_types::MarkupKind::Markdown,
                value: "Hover window contents here".to_string(),
            })
        })
    ));
}
```

That's it!

## Dependencies:

Neovim must be available on your `$PATH`. See the project's [documentation][nvim-install-docs]
for installation instructions. Versions at or later than [`517ecb8`][nvim-min-commit]
are necessary.

## Examples:

- The library's test corpus uses [rust-analyzer][rust-analyzer]. See [`test-suite/src/*`][repo-tests]
for examples of how to use the library.
- TODO: Add asm-lsp/other LSPs here once it's being used.

## TODO:

As an eventual end goal, we'd obviously like to provide test coverage for *all* LSP methods.
So far, we have:

- [x] `callHierarchy/incomingCalls`
- [x] `callHierarchy/outgoingCalls`
- [x] `codeLens/resolve`
- [x] `completionItem/resolve`
- [x] `documentLink/resolve`
- [x] `textDocument/codeAction`
- [x] `textDocument/codeLens`
- [x] `textDocument/colorPresentation`
- [x] `textDocument/completion`
- [x] `textDocument/declaration`
- [x] `textDocument/definition`
- [x] `textDocument/diagnostic`
- [x] `textDocument/documentColor`
- [x] `textDocument/documentHighlight`
- [x] `textDocument/documentLink`
- [x] `textDocument/documentSymbol`
- [x] `textDocument/foldingRange`
- [x] `textDocument/formatting`
- [x] `textDocument/implementation`
- [x] `textDocument/inlayHint`
- [x] `textDocument/hover`
- [x] `textDocument/linkedEditingRange`
- [x] `textDocument/moniker`
- [x] `textDocument/onTypeFormatting`
- [x] `textDocument/prepareCallHierarchy`
- [x] `textDocument/prepareRename`
- [x] `textDocument/prepareTypeHierarchy`
- [x] `textDocument/publishDiagnostics`
- [x] `textDocument/rangeFormatting`
- [x] `textDocument/references`
- [x] `textDocument/rename`
- [x] `textDocument/selectionRange`
- [x] `textDocument/semanticTokens/full`
- [x] `textDocument/semanticTokens/full/delta` -- Could use some work
- [x] `textDocument/semanticTokens/range`
- [x] `textDocument/signatureHelp`
- [x] `textDocument/typeDefinition`
- [x] `workspace/diagnostic`

## Gotchas/Known Issues

- If your server undergoes some sort of indexing process at startup before it's ready
to service a given request, you need to account for this by specifying `ServerStartType::Progress(NonZeroU32, String)`
to the test case. The `NonZeroU32` specifies *which* `end` message to issue the request
after (in case there are multiple). The `String` provides the relevant [progress token][progress-token].

- **String comparison of results**: Many LSP client implementations do some post processing
of responses returned by a given language server, primarily removing newlines. Your expected
response may need to be minimally altered from what you originally expect in order for tests
to pass.

- **Uri fields**: If a response contains a Uri field with an absolute path, this field
will be sanitizd to a relative path up to the test case's root directory. Your test case's
expected results may need to be adjusted to reflect this.

- **Variance in LSP client implementation**: The [LSP Spec][lsp-spec] is somewhat loosely defined,
leaving plenty of room for client implementations to behave differently from one another. This
project utilizes [neovim][nvim-repo]'s, meaning that unexpected behavior may occur when your server
is used with other editors' clients.

- **Error Messages with Empty Container Types**: If the response to an LSP request contains an enum
with inner types such as `Vec` or `HashMap`, and you expect this to be empty, the error messages
associated with a failure with said test case may display the wrong enum variant. This is because
the Rust LSP types are untagged, so there is no way to differentiate between empty container variants
during deserialization.

## Contributing

- In addition to [neovim][nvim-repo], working on this project also requires having having
[rust-analyzer][rust-analyzer] 1.85 on your `$PATH`, as it is used in the project's test suite.

[lsp-spec]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
[progress-token]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#progress
[nvim-repo]: https://github.com/neovim/neovim
[nvim-install-docs]: https://github.com/neovim/neovim#install-from-source
[nvim-min-commit]: https://github.com/neovim/neovim/commit/517ecb85f58ed6ac8b4d5443931612e75e7c7dc2
[rust-analyzer]: https://github.com/rust-lang/rust-analyzer
[repo-tests]: https://github.com/WillLillis/lspresso-shot/tree/master/test-suite/src
