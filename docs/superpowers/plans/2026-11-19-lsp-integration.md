# simplicityhl-lsp Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Opening a `.simf` file in Zed starts `simplicityhl-lsp`. The extension installs a pinned version with cargo when no server is found.

**Architecture:** This turns the config-only extension into a Rust/WASM Zed extension. `src/server.rs` holds pure, host-testable logic: names, paths, cargo arguments, version folders, env and error text. `src/lib.rs` is thin glue over `zed_extension_api` that does the file, process and settings I/O.

**Tech Stack:** Rust (edition 2021), `zed_extension_api` 0.7.0, `serde_json` 1, target `wasm32-wasip2`.

**Spec:** `docs/superpowers/specs/2026-11-19-lsp-integration-design.md`

## Global Constraints

- The server crate, binary, Zed language-server id and settings key are all `simplicityhl-lsp`.
- The pinned server version is `0.9.1`, defined once, as `PINNED_VERSION` in `src/server.rs`.
- The managed install goes to `<work dir>/simplicityhl-lsp-<version>/bin/simplicityhl-lsp`, with `.exe` on Windows.
- The install command is `cargo install --color never --locked --version <v> --root <abs dir> simplicityhl-lsp`, run as plain `cargo`. The `--color never` flag matches the VSCode extension.
- Every cargo command line must match a `[[capabilities]]` entry in `extension.toml` exactly. Zed rejects anything else.
- Never write outside the extension work dir. Never touch `~/.cargo/bin`.
- The language server is registered for the existing language `Simfony`. Don't rename it.
- Don't add HTTP calls or update checks.

## Review Focus

1. **No user settings.** The server's `didChangeConfiguration` must get `{}`, not `null`. The server rejects `null` and shows "Invalid SimplicityHL settings". Test in Task 2.
2. **Zed launched from a desktop launcher.** The shell PATH may lack `~/.cargo/bin`. Append `$CARGO_HOME/bin`, or `$HOME/.cargo/bin` when `CARGO_HOME` is unset, to PATH for the cargo run. Test in Task 3.
3. **Zed quits mid-build.** The version folder then exists without a binary. "Installed" means the binary file exists, not the folder. A half-built folder is never a fallback and never blocks a reinstall. Test in Task 3.
4. **Version folders compare numerically.** `0.10.0` is newer than `0.9.1`. Cleanup deletes only `simplicityhl-lsp-<x.y.z>` folders, never other entries in the work dir. Test in Task 3.
5. **cargo fails with long output.** The error must be the readable tail: the last 20 lines of stderr, uncolored. Test in Task 3.

---

## File Structure

| File | Responsibility |
|------|----------------|
| `Cargo.toml` (create) | Crate `zed_simplicityhl`: `[lib] crate-type = ["cdylib"]`, deps `zed_extension_api = "0.7.0"`, `serde_json = "1"` |
| `src/server.rs` (create) | Pure logic and its unit tests. Makes no `zed_extension_api` calls. |
| `src/lib.rs` (create) | `SimplicityHlExtension: zed::Extension` glue and `zed::register_extension!` |
| `extension.toml` (modify) | `[language_servers.simplicityhl-lsp]` and the cargo `[[capabilities]]` entry |
| `README.md` (create) | Requirements, first-start note and settings example |
| `Cargo.lock` (create, commit) | Generated |

---

### Task 1: Managed install end to end (spike, then checked in Zed)

This is the riskiest step. Nothing yet proves Zed tolerates a multi-minute `cargo install` inside `language_server_command`. Stop and report if the Zed checkpoint fails.

**Files:** Create `Cargo.toml`, `src/server.rs`, `src/lib.rs`. Modify `extension.toml`.

**Interfaces:**
- Produces (in `src/server.rs`):
  - `pub const SERVER_ID: &str = "simplicityhl-lsp";`
  - `pub const PINNED_VERSION: &str = "0.9.1";`
  - `pub fn version_dir(version: &str) -> String`
  - `pub fn binary_path(version_dir: &str, windows: bool) -> String`. The path is relative to the work dir.
  - `pub fn cargo_install_args(version: &str, root: &str) -> Vec<String>`
- Produces (in `src/lib.rs`): `fn managed_binary(&mut self, id: &zed::LanguageServerId, worktree: &zed::Worktree) -> zed::Result<String>`. It returns an absolute binary path.

- [ ] **Step 1: Write the failing tests** in `src/server.rs` `mod tests`:

```rust
#[test]
fn version_dir_names_the_server_and_version() {
    assert_eq!(version_dir("0.9.1"), "simplicityhl-lsp-0.9.1");
}
#[test]
fn binary_path_is_under_bin_with_exe_on_windows() {
    assert_eq!(binary_path("simplicityhl-lsp-0.9.1", false), "simplicityhl-lsp-0.9.1/bin/simplicityhl-lsp");
    assert_eq!(binary_path("simplicityhl-lsp-0.9.1", true), "simplicityhl-lsp-0.9.1/bin/simplicityhl-lsp.exe");
}
#[test]
fn cargo_install_args_pin_version_root_and_no_color() {
    assert_eq!(cargo_install_args("0.9.1", "/w/simplicityhl-lsp-0.9.1"),
        ["install", "--color", "never", "--locked", "--version", "0.9.1",
         "--root", "/w/simplicityhl-lsp-0.9.1", "simplicityhl-lsp"]);
}
```

Create `Cargo.toml` and a `src/lib.rs` with `mod server;` so the tests compile.

- [ ] **Step 2: Run** `cargo test`. Expected: compile errors, because the functions are undefined.
- [ ] **Step 3: Implement** the three functions and the two constants.
- [ ] **Step 4: Run** `cargo test`. Expected: 3 passed. If `zed_extension_api` fails to build on the host, report it, because the plan's testing approach depends on it.
- [ ] **Step 5: Write the glue** in `src/lib.rs`.
  - `language_server_command` returns `zed::Command { command: managed_binary(..)?, args: vec![], env: worktree.shell_env() }`.
  - `managed_binary` works like this:
    1. Compute the relative binary path from `PINNED_VERSION` and `zed::current_platform().0 == zed::Os::Windows`.
    2. If `std::fs::metadata(rel).is_ok_and(|m| m.is_file())`, return `current_dir().join(rel)`.
    3. Otherwise call `zed::set_language_server_installation_status(id, &Downloading)` and run `zed::process::Command::new("cargo").args(cargo_install_args(PINNED_VERSION, <abs root>)).envs(worktree.shell_env()).output()`.
    4. On exit status `Some(0)` with the binary present, set the status to `None` and return the path. Otherwise set `Failed(<stderr as lossy UTF-8>)` and return `Err` with the same text.
- [ ] **Step 6: Edit `extension.toml`.** Add:

```toml
[language_servers.simplicityhl-lsp]
name = "SimplicityHL Language Server"
languages = ["Simfony"]

[[capabilities]]
kind = "process:exec"
command = "cargo"
args = ["install", "--color", "never", "--locked", "--version", "*", "--root", "*", "simplicityhl-lsp"]
```

- [ ] **Step 7: Build for WASM.** Run `rustup target add wasm32-wasip2 && cargo build --release --target wasm32-wasip2`. Expected: `Finished` with no errors.
- [ ] **Step 8: Prove the install command on the host.** In the scratchpad, run `cargo install --color never --locked --version 0.9.1 --root <scratchpad>/lsp-root simplicityhl-lsp`. Expected: exit 0, and `<scratchpad>/lsp-root/bin/simplicityhl-lsp` exists. Record the wall time, since it is the first-start delay users see.
- [ ] **Step 9: Checkpoint with the user in Zed.** The user runs `zed: install dev extension` on the repo root, then opens a `.simf` file and waits. Expected:
  - `Zed.log` shows the extension compiled with no ERROR.
  - The install status appears.
  - `~/.local/share/zed/extensions/work/simfony/simplicityhl-lsp-0.9.1/bin/simplicityhl-lsp` exists.
  - Hovering a jet such as `jet::eq_32` shows docs.

  If Zed times out or the server never starts, stop and report what `Zed.log` says.
- [ ] **Step 10: Commit** `Cargo.toml Cargo.lock src/ extension.toml` with the message `Start simplicityhl-lsp from a managed cargo install`.

### Task 2: Lookup order and settings pass-through

**Files:** Modify `src/server.rs` and `src/lib.rs`.

**Interfaces:**
- Consumes: `managed_binary`, `SERVER_ID` (Task 1).
- Produces (in `src/server.rs`):
  - `pub fn workspace_configuration(settings: Option<serde_json::Value>) -> serde_json::Value`
  - `pub fn initialization_options(init: Option<serde_json::Value>, settings: Option<serde_json::Value>) -> Option<serde_json::Value>`

- [ ] **Step 1: Write the failing tests:**

```rust
#[test]
fn workspace_configuration_defaults_to_empty_object() { // Review Focus 1
    assert_eq!(workspace_configuration(None), json!({}));
    let s = json!({"simplicityhl": {"experimentalFeatures": {"imports": true}}});
    assert_eq!(workspace_configuration(Some(s.clone())), s);
}
#[test]
fn initialization_options_prefer_explicit_then_settings() {
    assert_eq!(initialization_options(Some(json!({"a": 1})), Some(json!({"b": 2}))), Some(json!({"a": 1})));
    assert_eq!(initialization_options(None, Some(json!({"b": 2}))), Some(json!({"b": 2})));
    assert_eq!(initialization_options(None, None), None);
}
```

- [ ] **Step 2: Run** `cargo test`. Expected: FAIL, because the functions are undefined.
- [ ] **Step 3: Implement** both functions.
- [ ] **Step 4: Run** `cargo test`. Expected: all tests pass.
- [ ] **Step 5: Glue in `src/lib.rs`.** Read settings with `LspSettings::for_worktree(SERVER_ID, worktree)` (use `unwrap_or_default()` on error).
  - In `language_server_command`, try these in order:
    1. `settings.binary.path` → use it with `binary.arguments.unwrap_or_default()`.
    2. `worktree.which(SERVER_ID)` → use it with no arguments.
    3. Otherwise `managed_binary`.
  - In every case the env is `worktree.shell_env()`, which lets the server find `simfmt` on PATH.
  - Implement `language_server_workspace_configuration` → `Ok(Some(workspace_configuration(settings.settings)))`.
  - Implement `language_server_initialization_options` → `Ok(initialization_options(settings.initialization_options, settings.settings))`.
- [ ] **Step 6: Run** `cargo build --release --target wasm32-wasip2`. Expected: `Finished`.
- [ ] **Step 7: Commit** with the message `Honour binary path, PATH and LSP settings for simplicityhl-lsp`.

### Task 3: Robust install: cargo PATH, errors, fallback, cleanup

**Files:** Modify `src/server.rs` and `src/lib.rs`.

**Interfaces:**
- Consumes: `version_dir`, `binary_path`, `cargo_install_args`, `managed_binary` (Task 1).
- Produces (in `src/server.rs`):
  - `pub fn parse_version_dir(name: &str) -> Option<(u64, u64, u64)>`
  - `pub fn newest_version_dir(installed: &[String], exclude: &str) -> Option<String>`
  - `pub fn stale_version_dirs(entries: &[String], keep: &str) -> Vec<String>`
  - `pub fn env_with_cargo_bin(env: Vec<(String, String)>) -> Vec<(String, String)>`
  - `pub fn stderr_tail(stderr: &[u8], max_lines: usize) -> String`
  - `pub fn cargo_missing_message(err: &str) -> String`
  - `pub fn install_failed_message(version: &str, tail: &str) -> String`

- [ ] **Step 1: Write the failing tests:**

```rust
#[test]
fn version_dirs_parse_and_compare_numerically() { // Review Focus 4
    assert_eq!(parse_version_dir("simplicityhl-lsp-0.10.0"), Some((0, 10, 0)));
    assert_eq!(parse_version_dir("simplicityhl-lsp-foo"), None);
    assert_eq!(parse_version_dir("other-0.1.0"), None);
    let installed = vec!["simplicityhl-lsp-0.9.1".into(), "simplicityhl-lsp-0.10.0".into(), "simplicityhl-lsp-0.8.0".into()];
    assert_eq!(newest_version_dir(&installed, "simplicityhl-lsp-0.11.0").as_deref(), Some("simplicityhl-lsp-0.10.0"));
    assert_eq!(newest_version_dir(&installed[..1], "simplicityhl-lsp-0.9.1"), None);
}
#[test]
fn stale_version_dirs_only_lists_other_version_folders() { // Review Focus 4
    let entries = vec!["simplicityhl-lsp-0.8.0".into(), "simplicityhl-lsp-0.9.1".into(),
                       "simplicityhl-lsp-foo".into(), "notes.txt".into()];
    assert_eq!(stale_version_dirs(&entries, "simplicityhl-lsp-0.9.1"), vec!["simplicityhl-lsp-0.8.0"]);
}
#[test]
fn env_with_cargo_bin_appends_cargo_home_or_home() { // Review Focus 2
    let get = |e: Vec<(String, String)>| e.into_iter().find(|(k, _)| k == "PATH").map(|(_, v)| v);
    assert_eq!(get(env_with_cargo_bin(vec![("PATH".into(), "/usr/bin".into()), ("HOME".into(), "/h".into())])),
               Some("/usr/bin:/h/.cargo/bin".into()));
    assert_eq!(get(env_with_cargo_bin(vec![("PATH".into(), "/usr/bin".into()), ("HOME".into(), "/h".into()),
                                           ("CARGO_HOME".into(), "/c".into())])),
               Some("/usr/bin:/c/bin".into()));
    assert_eq!(get(env_with_cargo_bin(vec![("HOME".into(), "/h".into())])), Some("/h/.cargo/bin".into()));
}
#[test]
fn stderr_tail_keeps_last_lines() { // Review Focus 5
    let s: String = (1..=30).map(|i| format!("line {i}\n")).collect();
    let tail = stderr_tail(s.as_bytes(), 20);
    assert!(tail.starts_with("line 11") && tail.ends_with("line 30"));
}
#[test]
fn messages_name_rustup_and_version() {
    assert!(cargo_missing_message("not found").contains("https://rustup.rs"));
    assert!(cargo_missing_message("not found").contains("lsp.simplicityhl-lsp.binary.path"));
    assert!(install_failed_message("0.9.1", "error: x").contains("0.9.1"));
}
```

Use `;` as the PATH separator on Windows. `env_with_cargo_bin` may take a `windows: bool` if needed, in which case the tests above pass `false`.

- [ ] **Step 2: Run** `cargo test`. Expected: FAIL, because the functions are undefined.
- [ ] **Step 3: Implement** the functions. The exact copy:
  - `cargo_missing_message`: `"Couldn't run cargo to install simplicityhl-lsp ({err}). Install Rust from https://rustup.rs, or set lsp.simplicityhl-lsp.binary.path in Zed settings."`
  - `install_failed_message`: `"cargo install simplicityhl-lsp {version} failed:\n{tail}"`
- [ ] **Step 4: Run** `cargo test`. Expected: all tests pass.
- [ ] **Step 5: Glue in `managed_binary`.**
  - Run cargo with `env_with_cargo_bin(worktree.shell_env())`.
  - If `output()` returns `Err`, the error is `cargo_missing_message`.
  - If cargo exits non-zero, the error is `install_failed_message(PINNED_VERSION, &stderr_tail(&out.stderr, 20))`.
  - Fallback: list `std::fs::read_dir(".")` entries, keep only dirs whose `binary_path` is a file (Review Focus 3), then use `newest_version_dir(.., version_dir(PINNED_VERSION))`. If there is one, set the status to `None` and return its absolute path. Otherwise set `Failed(msg)` and return `Err(msg)`.
  - On success, `std::fs::remove_dir_all` each `stale_version_dirs` entry, ignoring errors.
- [ ] **Step 6: Run** `cargo build --release --target wasm32-wasip2`. Expected: `Finished`.
- [ ] **Step 7: Commit** with the message `Handle missing cargo, failed builds and old server versions`.

### Task 4: README and final check in Zed

**Files:** Create `README.md`.

- [ ] **Step 1: Write `README.md`.** Cover:
  - What the extension gives: highlighting, plus hover, go-to-definition (functions only), completions, diagnostics and formatting from `simplicityhl-lsp`.
  - Requirements: Rust 1.91+ via rustup and a C compiler. The first `.simf` open builds the server, which takes a few minutes.
  - The pinned version, `0.9.1`.
  - Using your own binary with `lsp.simplicityhl-lsp.binary.path`, or `simplicityhl-lsp` on PATH.
  - A settings example:

```json
{ "lsp": { "simplicityhl-lsp": { "settings": { "simplicityhl": { "experimentalFeatures": { "imports": true } } } } } }
```

- [ ] **Step 2: Checkpoint with the user in Zed.** The user re-runs `zed: install dev extension`, then:
  - opens a `.simf` file in a project;
  - hovers a jet (docs appear);
  - uses go-to-definition on a call to a user function (it jumps to the `fn`);
  - checks that diagnostics appear on a deliberate syntax error.

  Expected: all four work, and `Zed.log` has no ERROR for `simplicityhl-lsp`.
- [ ] **Step 3: Commit** `README.md` with the message `Document language server setup`.
