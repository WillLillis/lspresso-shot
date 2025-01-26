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
    // TODO: Fill this out once the API is more flushed out
}
```

That's it!

## Dependencies:

Neovim must be available on your `$PATH`. See the project's [documentation][nvim-install-docs]
for installation instructions.

## Checklist:

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
project utilizes [neovim](nvim-repo)'s, meaning that unexpected behavior may occur when your server
is used with other editors' clients.

## Contributing

- In addition to [neovim](nvim-repo), working on this project also requires having having
[rust-analyzer](rust-analyzer) on your `$PATH`, as it is used in the project's test suite.

[lsp-spec]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
[nvim-repo]: https://github.com/neovim/neovim
[nvim-install-docs]: https://github.com/neovim/neovim#install-from-source
[rust-analyzer]: https://github.com/rust-lang/rust-analyzer
