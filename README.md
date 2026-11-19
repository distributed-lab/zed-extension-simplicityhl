# SimplicityHL for Zed

[SimplicityHL](https://github.com/BlockstreamResearch/SimplicityHL) support for the
[Zed](https://zed.dev) editor.

- Syntax highlighting for `.simf` files, from
  [tree-sitter-simplicityhl](https://github.com/distributed-lab/tree-sitter-simplicityhl).
- Language features from
  [`simplicityhl-lsp`](https://github.com/BlockstreamResearch/simplicityhl-lsp):
  hover docs, go-to-definition, completions, diagnostics and formatting.

Hover and go-to-definition currently work for functions and jets only. Types such as
`Ge` or `Gej` get neither, because the server doesn't support them yet.

## Requirements

The extension builds the language server from source with cargo, so you need:

- Rust 1.91 or newer, installed with [rustup](https://rustup.rs)
- A C compiler (one of the server's dependencies compiles C code)

The first time you open a `.simf` file, the extension runs
`cargo install simplicityhl-lsp` into its own folder, so it doesn't touch
`~/.cargo/bin`. This takes a minute or two, and hover and completions start working
when it finishes. Later starts are instant.

The extension installs `simplicityhl-lsp` **0.9.1**. It doesn't update the server
automatically; new versions arrive with new releases of this extension.

## Using your own server binary

The extension looks for the server in this order:

1. `lsp.simplicityhl-lsp.binary.path` in your Zed settings
2. `simplicityhl-lsp` on your `PATH`
3. Its own managed install, described above

For example, to use a server you built yourself:

```json
{
  "lsp": {
    "simplicityhl-lsp": {
      "binary": { "path": "/path/to/simplicityhl-lsp" }
    }
  }
}
```

## Settings

Settings under `lsp.simplicityhl-lsp.settings` are passed to the server. See the
[server's README](https://github.com/BlockstreamResearch/simplicityhl-lsp#configuration)
for every option. For example, to turn on the experimental `use`/`mod` syntax:

```json
{
  "lsp": {
    "simplicityhl-lsp": {
      "settings": {
        "simplicityhl": {
          "experimentalFeatures": { "imports": true }
        }
      }
    }
  }
}
```

## Development

Install the extension from a checkout with `zed: install dev extension` and pick
this folder. Run the unit tests with `cargo test`.

## Maintaining

- **Bump the server.** Change `PINNED_VERSION` in `src/server.rs`, then reinstall
  the dev extension and check hover and go-to-definition in a `.simf` file. The
  extension keeps the old version until the new one builds, then deletes the old one.
- **Bump the grammar.** Change `rev` in `extension.toml` to the new commit SHA,
  reinstall the dev extension, and open a `.simf` file. A grammar change can rename
  or remove nodes that `languages/simplicityhl/highlights.scm` uses. Zed then logs
  an `Invalid node type` error in `~/.local/share/zed/logs/Zed.log` and highlighting
  stops, so check the log.
- **Change the grammar's repository URL.** Delete the `grammars/` folder before
  reinstalling. Zed refuses to reuse a grammar checkout whose remote doesn't match
  `extension.toml`.
- **Release.** Bump `version` in `extension.toml`.

## License

MIT, see [LICENSE](LICENSE).
