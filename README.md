# LSPresso-Shot

A concentrated dose of LSP testing power!

## WIP

This library is currently a work in progress and is *not* ready for use in other projects!

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
    .cursor_pos(Some(Position::new(0, 0)))
    .other_file( // Optional
        TestFile::new("Other file name", "Other contents")
    );

    lspresso_shot!(test_hover(
        hover_test_case,
        Hover {
            range: Some(Range {
                start: lsp_types::Position {
                    line: 1,
                    character: 2,
                },
                end: lsp_types::Position {
                    line: 1,
                    character: 3,
                },
            }),
            contents: lsp_types::HoverContents::Markup(MarkupContent {
                kind: lsp_types::MarkupKind::Markdown,
                value: "Hover window contents here".to_string(),
            })
        }
    ));
}
```

That's it!

## Dependencies:

Neovim must be available on your `$PATH`. See the project's [documentation][nvim-install-docs]
for installation instructions. (TODO: Figure out what versions are compatible)

## Examples:

- The library's test corpus uses [rust-analyzer][rust-analyzer]. See [`src/test.rs`][repo-tests]
for examples of how to use the library.
- TODO: Add asm-lsp/other LSPs here once it's being used.

## Checklist/TODOs:

- [x] Refactor to use the type definitions from the [lsp-types](https://github.com/gluon-lang/lsp-types)
crate
- [x] Use neovim's builtin api to serialize lsp responses into JSON rather than
hand-encoding information to TOML
- [x] Try to find a better way to determine when a `$/progress`-style server has
fully started up, rather than the current polling approach
- [x] Place Lua logic into dedicated files rather than as strings within the Rust
files
- [ ] Clean up Lua logic (I'm unfamiliar with the neovim API)
    - Add Lua unit tests? (Do we roll our own/ is there an easy framework?)
- [x] Add CI and whatnot (Improvements to current lua workflow?)

As an eventual end goal, we'd obviously like to provide test coverage for *all* LSP methods.
To start though, let's focus on the following TODOs:

- [x] Create a *very* simple test server so we can ensure coverage of all type variants
    - The basic thought here is to have the server's response defined so that it can
      be accessed on the Rust side by both the testing library and test server.
    - There will be a simple event loop, matching against each covered request type.
    - For methods with multiple return types (i.e. `textDocument/completion`, we'll
      have multiple predefined responses for each. The expected response type can be communicated
      from the test to the test server through one or more of the request params (i.e.
      line number).
- [ ] Sync up test server test coverage with current rust-analyzer coverage
- [x] `textDocument/hover`
- [x] `textDocument/publishDiagnostics`
- [x] `textDocument/references`
- [ ] `textDocument/definition` (needs test coverage for other variants)
- [ ] `textDocument/completion` (needs better ergonomics for failing cases)
- [x] `textDocument/formatting`
- [x] `textDocument/rename`

## Gotchas

- If your server undergoes some sort of indexing process at startup before it's ready
to service a given request, you need to account for this by specifying `ServerStartType::Progress(i32, String)`
to the test case. The `NonZeroU32` specifies *which* `end` message to issue the request
after (in case there are multiple). The `String` provides the relevant [progress token][progress-token].

- **String comparison of results**: Many LSP client implementations do some post processing
of responses returned by a given language server, primarily removing newlines. Your expected
response may need to be minimally altered from what you originally expect in order for tests
to pass.

- **Variance in LSP client implementation**: The [LSP Spec][lsp-spec] is somewhat loosely defined,
leaving plenty of room for client implementations to behave differently from one another. This
project utilizes [neovim][nvim-repo]'s, meaning that unexpected behavior may occur when your server
is used with other editors' clients.

## Contributing

- In addition to [neovim][nvim-repo], working on this project also requires having having
[rust-analyzer][rust-analyzer] 1.84.1 on your `$PATH`, as it is used in the project's test suite.

[lsp-spec]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
[progress-token]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#progress
[nvim-repo]: https://github.com/neovim/neovim
[nvim-install-docs]: https://github.com/neovim/neovim#install-from-source
[rust-analyzer]: https://github.com/rust-lang/rust-analyzer
[repo-tests]: https://github.com/WillLillis/lspresso-shot/blob/master/src/test.rs
