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
        "Path to server"
        "Source file name",
        "Source file contents"
    )
    .cursor_pos(Some(Position::new(0, 0)))
    .other_file( // Optional
        "Other file name",
        "Other file contents"
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

- The library's test corpus uses [rust-analyzer][rust-analyzer]. See `src/test.rs`
for examples of how to use the library.
- TODO: Add asm-lsp/other LSPs here once it's being used.

## Checklist/TODOs:

- [x] Refactor to use the type definitions from the [lsp-types][https://github.com/gluon-lang/lsp-types]
crate
- [x] Use neovim's builtin api to serialize lsp responses into JSON rather than
hand-encoding information to TOML
- [ ] Try to find a better way to determine when a `$/progress`-style server has
fully started up, rather than the current polling approach
- [x] Place Lua logic into dedicated files rather than as strings within the Rust
files
- [ ] Clean up Lua logic (I'm unfamiliar with the neovim API)
- [ ] Add CI and whatnot

As an eventual end goal, we'd obviously like to provide test coverage for *all* LSP methods.
To start though, let's focus on the following TODOs:

- [ ] It likely doesn't make sense to bundle a neovim execuatable with the project. The
solution here is probably just to require uses to have neovim installed on their systems
in order to use the project, but maybe there's a way around this.
- [ ] `textDocument/hover`
- [ ] `textDocument/publishDiagnostics`
- [ ] `textDocument/references`
- [ ] `textDocument/definition`
- [ ] `textDocument/formatting`
- [ ] `textDocument/rename`

## Gotchas

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
[rust-analyzer][rust-analyzer] on your `$PATH`, as it is used in the project's test suite.

[lsp-spec]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
[nvim-repo]: https://github.com/neovim/neovim
[nvim-install-docs]: https://github.com/neovim/neovim#install-from-source
[rust-analyzer]: https://github.com/rust-lang/rust-analyzer
