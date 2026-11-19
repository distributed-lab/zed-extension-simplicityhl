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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
