# Language server support for the Zed extension

Date: 2026-11-19 · Size: small change

## Intent

The extension highlights `.simf` files, but Zed shows no hover docs and
go-to-definition does nothing. Both come from a language server, and the
extension doesn't start one. The aim is to connect
[`simplicityhl-lsp`](https://github.com/BlockstreamResearch/simplicityhl-lsp)
(v0.9.1 on crates.io) and have the extension install it, so users don't have to
set it up themselves.

## Goals

- Opening a `.simf` file starts `simplicityhl-lsp`. The user gets hover docs,
  go-to-definition (functions only, a limit of the server), completions,
  diagnostics and formatting.
- If the server isn't installed, the extension installs it with no manual steps
  beyond having Rust.

## Non-goals

- Downloading prebuilt binaries. Upstream publishes none. The code keeps the
  install step separate, so a download can be added later.
- Go-to-definition for types, aliases or variables (the server doesn't support it).
- Renaming the language from "Simfony" to "SimplicityHL", changing the grammar's
  SSH URL, or supporting witness files (`.wit`).

## Anti-goals

- Failing silently. A `.simf` file that opens with no server and no explanation
  counts as a failure.
- Touching the user's global cargo installs (`~/.cargo/bin`).
- Rebuilding the server on every Zed start when nothing changed.

## Constraints

- No prebuilt binaries exist, so the server is built from source with cargo.
  Users need Rust and a C compiler (`simplicity-sys` builds C code via `cc`).
  The first start takes a few minutes.
- Running cargo needs the `process:exec` capability. Zed grants it to users by
  default, but `extension.toml` must also list every cargo command line the
  extension runs under `[[capabilities]]` (`extension_manifest.rs:168`). Zed
  matches the command name as an exact string, so the extension runs plain
  `cargo`, not an absolute path. The Ruby extension does the same with `gem`.
- The extension's work dir is available as an absolute host path through
  `std::env::current_dir()`. The Ruby extension uses it this way for
  `GEM_HOME`.
- Neither Zed's `run_command` nor `language_server_command` has a timeout in
  the source I read. A multi-minute `cargo install` inside that call is still
  untested, so the plan tests it first.
- The server needs Rust 1.91 or newer (`rust-version` in its `Cargo.toml`). Users
  with an older toolchain get a failed build.
- `zed_extension_api` 0.7.0 works with the installed Zed 1.29.
- The server reads its settings from `initializationOptions` and
  `workspace/didChangeConfiguration`. It accepts them with or without a
  `simplicityhl` wrapper key (`src/config.rs:98`).

## Approaches

- **A. Install with cargo on first use — chosen.** The extension finds the server
  or runs `cargo install` for it. This needs no infrastructure beyond this repo.
  The VSCode extension does the same. For updates, the user chose a pinned
  server version over automatic updates from crates.io, which would leave users
  with no server for a few minutes after each upstream release.
- **B. Download prebuilt binaries.** This gives the best experience, but
  distributed-lab would own a build pipeline and have to rebuild for every
  upstream release. Not chosen.

## Design

The extension becomes a Rust extension (`Cargo.toml`, `src/lib.rs`). It registers
the language server `simplicityhl-lsp` for the existing `Simfony` language.

**Finding the binary.** The extension tries these in order and uses the first one
that exists:
1. `lsp.simplicityhl-lsp.binary.path` from Zed settings, with that entry's
   `arguments` if given.
2. `simplicityhl-lsp` on the worktree's PATH (`worktree.which`).
3. A managed install of the pinned version (below). Zed shows the install status
   while cargo runs.

**Running cargo.** The extension runs plain `cargo` with the worktree's shell
environment. If cargo isn't on that PATH, it adds `$HOME/.cargo/bin` before
giving up. The install path given to `--root` comes from
`std::env::current_dir()`.

**Pinned version** (the user's choice). The extension pins one server version
in its code, starting at `0.9.1`. It never contacts crates.io for updates, and
users move to a new server version only when someone bumps the pin in this repo.

**Managed install** (my call on layout). Each version installs into its own
folder:
`cargo install simplicityhl-lsp --version 0.9.1 --locked --root <work dir>/simplicityhl-lsp-0.9.1`.
The binary is at `<that folder>/bin/simplicityhl-lsp`, so the binary's existence
shows the pinned version is installed and no version marker is needed.
`--locked` builds with the crate's own `Cargo.lock`, which makes builds
reproducible. After a bumped version installs, the extension deletes the old
version folders. If building a bumped version fails and an older version's
folder exists, the extension uses that older binary. Binaries found through
settings or PATH are left alone.

**Errors** (my call). If cargo isn't found, the error says Rust is required and
links to <https://rustup.rs>. If the first `cargo install` fails, the error
includes the end of cargo's stderr. A failed update is not an error, because the
old binary keeps working. These errors show in Zed's language server status.

**Settings** (my call). The extension forwards the user's
`lsp.simplicityhl-lsp.settings` to the server as both initialization options and
workspace configuration. This is how users turn on options such as
`experimentalFeatures.imports`.

**README** (my call). The repo gets a short README covering what the extension
does, the Rust and C compiler requirement, the slow first start, and a settings
example.

## Left to the builder

- The exact `[[capabilities]]` entry for the install command line. It must match
  the arguments above.
- How to test it: pure logic gets unit tests, the extension must build for
  `wasm32-wasip2`, and a final end-to-end check happens in Zed. The first task
  is a spike: a minimal `language_server_command` that runs the full install,
  checked in Zed and `Zed.log`.
