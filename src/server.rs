//! Pure logic for locating and installing `simplicityhl-lsp`. Nothing here
//! calls the Zed extension API, so it runs under host `cargo test`.

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

#[cfg(test)]
mod tests {
    use super::*;

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
