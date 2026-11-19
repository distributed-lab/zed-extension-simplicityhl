//! Pure logic for locating and installing `simplicityhl-lsp`. Nothing here
//! calls the Zed extension API, so it runs under host `cargo test`.

use serde_json::Value;

/// Crate name, binary name, Zed language-server id and settings key.
pub const SERVER_ID: &str = "simplicityhl-lsp";

/// The server version the extension installs. Bump it deliberately.
pub const PINNED_VERSION: &str = "0.9.1";

/// Folder, relative to the extension work dir, that holds one server version.
pub fn version_dir(version: &str) -> String {
    format!("{SERVER_ID}-{version}")
}

/// Path of the server binary inside a version folder, relative to the work dir.
pub fn binary_path(version_dir: &str, windows: bool) -> String {
    let exe = if windows { ".exe" } else { "" };
    format!("{version_dir}/bin/{SERVER_ID}{exe}")
}

/// Arguments for `cargo` that install `version` into `root`. Must match the
/// `process:exec` capability in `extension.toml`.
pub fn cargo_install_args(version: &str, root: &str) -> Vec<String> {
    [
        "install", "--color", "never", "--locked", "--version", version, "--root", root, SERVER_ID,
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

/// Workspace configuration for the server: the user's
/// `lsp.simplicityhl-lsp.settings`, or `{}` because the server rejects `null`.
pub fn workspace_configuration(settings: Option<Value>) -> Value {
    settings.unwrap_or_else(|| Value::Object(Default::default()))
}

/// Initialization options: explicit `initialization_options` win, otherwise
/// the same `settings` the workspace configuration carries.
pub fn initialization_options(init: Option<Value>, settings: Option<Value>) -> Option<Value> {
    init.or(settings)
}

/// Parses a version folder name such as `simplicityhl-lsp-0.10.0`.
pub fn parse_version_dir(name: &str) -> Option<(u64, u64, u64)> {
    let version = name.strip_prefix(SERVER_ID)?.strip_prefix('-')?;
    let mut parts = version.split('.').map(|part| part.parse::<u64>().ok());
    let parsed = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(parsed)
}

/// Newest version folder in `installed`, other than `exclude`. Callers pass
/// only folders whose binary exists, so a half-built folder is never chosen.
pub fn newest_version_dir(installed: &[String], exclude: &str) -> Option<String> {
    installed
        .iter()
        .filter(|name| name.as_str() != exclude)
        .filter_map(|name| parse_version_dir(name).map(|version| (version, name)))
        .max_by_key(|(version, _)| *version)
        .map(|(_, name)| name.clone())
}

/// Version folders other than `keep`. Never lists anything that isn't a
/// `simplicityhl-lsp-<x.y.z>` folder name.
pub fn stale_version_dirs(entries: &[String], keep: &str) -> Vec<String> {
    entries
        .iter()
        .filter(|name| name.as_str() != keep && parse_version_dir(name).is_some())
        .cloned()
        .collect()
}

/// Appends cargo's bin dir (`$CARGO_HOME/bin`, else `<home>/.cargo/bin`) to
/// PATH, so cargo is found even when Zed's shell env doesn't include it.
pub fn env_with_cargo_bin(mut env: Vec<(String, String)>, windows: bool) -> Vec<(String, String)> {
    let lookup = |env: &[(String, String)], key: &str| {
        env.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    };
    let home_key = if windows { "USERPROFILE" } else { "HOME" };
    let cargo_bin = match (lookup(&env, "CARGO_HOME"), lookup(&env, home_key)) {
        (Some(cargo_home), _) => format!("{cargo_home}/bin"),
        (None, Some(home)) => format!("{home}/.cargo/bin"),
        (None, None) => return env,
    };
    let separator = if windows { ";" } else { ":" };
    let is_path = |k: &str| if windows { k.eq_ignore_ascii_case("PATH") } else { k == "PATH" };
    match env.iter_mut().find(|(k, _)| is_path(k)) {
        Some((_, path)) => *path = format!("{path}{separator}{cargo_bin}"),
        None => env.push(("PATH".to_string(), cargo_bin)),
    }
    env
}

/// The last `max_lines` lines of a process's stderr.
pub fn stderr_tail(stderr: &[u8], max_lines: usize) -> String {
    let text = String::from_utf8_lossy(stderr);
    let lines: Vec<&str> = text.trim_end().lines().collect();
    lines[lines.len().saturating_sub(max_lines)..].join("\n")
}

/// Error shown when cargo can't be started at all.
pub fn cargo_missing_message(err: &str) -> String {
    format!(
        "Couldn't run cargo to install simplicityhl-lsp ({err}). Install Rust from https://rustup.rs, or set lsp.simplicityhl-lsp.binary.path in Zed settings."
    )
}

/// Error shown when `cargo install` exits with a failure.
pub fn install_failed_message(version: &str, tail: &str) -> String {
    format!("cargo install simplicityhl-lsp {version} failed:\n{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn path_of(env: Vec<(String, String)>) -> Option<String> {
        env.into_iter().find(|(k, _)| k == "PATH").map(|(_, v)| v)
    }

    fn env(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn version_dirs_parse_and_compare_numerically() {
        // Review Focus 4
        assert_eq!(parse_version_dir("simplicityhl-lsp-0.10.0"), Some((0, 10, 0)));
        assert_eq!(parse_version_dir("simplicityhl-lsp-foo"), None);
        assert_eq!(parse_version_dir("other-0.1.0"), None);
        let installed: Vec<String> = vec![
            "simplicityhl-lsp-0.9.1".into(),
            "simplicityhl-lsp-0.10.0".into(),
            "simplicityhl-lsp-0.8.0".into(),
        ];
        assert_eq!(
            newest_version_dir(&installed, "simplicityhl-lsp-0.11.0").as_deref(),
            Some("simplicityhl-lsp-0.10.0")
        );
        assert_eq!(newest_version_dir(&installed[..1], "simplicityhl-lsp-0.9.1"), None);
    }

    #[test]
    fn stale_version_dirs_only_lists_other_version_folders() {
        // Review Focus 4
        let entries: Vec<String> = vec![
            "simplicityhl-lsp-0.8.0".into(),
            "simplicityhl-lsp-0.9.1".into(),
            "simplicityhl-lsp-foo".into(),
            "notes.txt".into(),
        ];
        assert_eq!(
            stale_version_dirs(&entries, "simplicityhl-lsp-0.9.1"),
            vec!["simplicityhl-lsp-0.8.0"]
        );
    }

    #[test]
    fn env_with_cargo_bin_appends_cargo_home_or_home() {
        // Review Focus 2
        assert_eq!(
            path_of(env_with_cargo_bin(env(&[("PATH", "/usr/bin"), ("HOME", "/h")]), false)),
            Some("/usr/bin:/h/.cargo/bin".into())
        );
        assert_eq!(
            path_of(env_with_cargo_bin(
                env(&[("PATH", "/usr/bin"), ("HOME", "/h"), ("CARGO_HOME", "/c")]),
                false
            )),
            Some("/usr/bin:/c/bin".into())
        );
        assert_eq!(
            path_of(env_with_cargo_bin(env(&[("HOME", "/h")]), false)),
            Some("/h/.cargo/bin".into())
        );
    }

    #[test]
    fn env_with_cargo_bin_uses_semicolon_and_userprofile_on_windows() {
        assert_eq!(
            path_of(env_with_cargo_bin(
                env(&[("PATH", "C:\\bin"), ("USERPROFILE", "C:\\Users\\u")]),
                true
            )),
            Some("C:\\bin;C:\\Users\\u/.cargo/bin".into())
        );
    }

    #[test]
    fn env_with_cargo_bin_leaves_env_alone_without_a_home() {
        let original = env(&[("PATH", "/usr/bin")]);
        assert_eq!(env_with_cargo_bin(original.clone(), false), original);
    }

    #[test]
    fn stderr_tail_keeps_last_lines() {
        // Review Focus 5
        let s: String = (1..=30).map(|i| format!("line {i}\n")).collect();
        let tail = stderr_tail(s.as_bytes(), 20);
        assert!(tail.starts_with("line 11") && tail.ends_with("line 30"), "{tail}");
    }

    #[test]
    fn messages_name_rustup_and_version() {
        assert!(cargo_missing_message("not found").contains("https://rustup.rs"));
        assert!(cargo_missing_message("not found").contains("lsp.simplicityhl-lsp.binary.path"));
        assert!(install_failed_message("0.9.1", "error: x").contains("0.9.1"));
    }

    #[test]
    fn workspace_configuration_defaults_to_empty_object() {
        // Review Focus 1: the server rejects `null` settings.
        assert_eq!(workspace_configuration(None), json!({}));
        let s = json!({"simplicityhl": {"experimentalFeatures": {"imports": true}}});
        assert_eq!(workspace_configuration(Some(s.clone())), s);
    }

    #[test]
    fn initialization_options_prefer_explicit_then_settings() {
        assert_eq!(
            initialization_options(Some(json!({"a": 1})), Some(json!({"b": 2}))),
            Some(json!({"a": 1}))
        );
        assert_eq!(initialization_options(None, Some(json!({"b": 2}))), Some(json!({"b": 2})));
        assert_eq!(initialization_options(None, None), None);
    }

    #[test]
    fn version_dir_names_the_server_and_version() {
        assert_eq!(version_dir("0.9.1"), "simplicityhl-lsp-0.9.1");
    }

    #[test]
    fn binary_path_is_under_bin_with_exe_on_windows() {
        assert_eq!(
            binary_path("simplicityhl-lsp-0.9.1", false),
            "simplicityhl-lsp-0.9.1/bin/simplicityhl-lsp"
        );
        assert_eq!(
            binary_path("simplicityhl-lsp-0.9.1", true),
            "simplicityhl-lsp-0.9.1/bin/simplicityhl-lsp.exe"
        );
    }

    #[test]
    fn cargo_install_args_pin_version_root_and_no_color() {
        assert_eq!(
            cargo_install_args("0.9.1", "/w/simplicityhl-lsp-0.9.1"),
            [
                "install",
                "--color",
                "never",
                "--locked",
                "--version",
                "0.9.1",
                "--root",
                "/w/simplicityhl-lsp-0.9.1",
                "simplicityhl-lsp"
            ]
        );
    }
}
