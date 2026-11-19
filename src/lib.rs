mod server;

use std::fs;

use zed_extension_api::{
    self as zed, serde_json::Value, settings::LspSettings, LanguageServerId,
    LanguageServerInstallationStatus, Worktree,
};

use server::{
    binary_path, cargo_install_args, cargo_missing_message, env_with_cargo_bin,
    initialization_options, install_failed_message, newest_version_dir, stale_version_dirs,
    stderr_tail, version_dir, workspace_configuration, PINNED_VERSION, SERVER_ID,
};

struct SimplicityHlExtension;

impl SimplicityHlExtension {
    /// Returns the absolute path of the managed server binary, installing the
    /// pinned version with cargo first if it isn't there. If that install
    /// fails, falls back to the newest older version that is fully installed.
    fn managed_binary(&mut self, id: &LanguageServerId, worktree: &Worktree) -> zed::Result<String> {
        let windows = zed::current_platform().0 == zed::Os::Windows;
        let pinned_dir = version_dir(PINNED_VERSION);
        let rel_binary = binary_path(&pinned_dir, windows);
        let work_dir = std::env::current_dir().map_err(|e| format!("no extension work dir: {e}"))?;
        let abs = |rel: &str| work_dir.join(rel).to_string_lossy().into_owned();

        if is_file(&rel_binary) {
            return Ok(abs(&rel_binary));
        }

        zed::set_language_server_installation_status(id, &LanguageServerInstallationStatus::Downloading);
        let output = zed::process::Command::new("cargo")
            .args(cargo_install_args(PINNED_VERSION, &abs(&pinned_dir)))
            .envs(env_with_cargo_bin(worktree.shell_env(), windows))
            .output();

        let error = match output {
            Ok(out) if out.status == Some(0) && is_file(&rel_binary) => {
                zed::set_language_server_installation_status(id, &LanguageServerInstallationStatus::None);
                for stale in stale_version_dirs(&work_dir_entries(), &pinned_dir) {
                    let _ = fs::remove_dir_all(stale);
                }
                return Ok(abs(&rel_binary));
            }
            Ok(out) => install_failed_message(PINNED_VERSION, &stderr_tail(&out.stderr, 20)),
            Err(err) => cargo_missing_message(&err),
        };

        // Review Focus 3: only folders whose binary exists count as installed.
        let installed: Vec<String> = work_dir_entries()
            .into_iter()
            .filter(|dir| is_file(&binary_path(dir, windows)))
            .collect();
        if let Some(fallback) = newest_version_dir(&installed, &pinned_dir) {
            zed::set_language_server_installation_status(id, &LanguageServerInstallationStatus::None);
            return Ok(abs(&binary_path(&fallback, windows)));
        }

        zed::set_language_server_installation_status(id, &LanguageServerInstallationStatus::Failed(error.clone()));
        Err(error)
    }
}

/// Names of the entries in the extension work dir.
fn work_dir_entries() -> Vec<String> {
    fs::read_dir(".")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect()
}

fn lsp_settings(worktree: &Worktree) -> LspSettings {
    LspSettings::for_worktree(SERVER_ID, worktree).unwrap_or_default()
}

fn is_file(path: &str) -> bool {
    fs::metadata(path).is_ok_and(|m| m.is_file())
}

impl zed::Extension for SimplicityHlExtension {
    fn new() -> Self {
        SimplicityHlExtension
    }

    /// Lookup order: `lsp.simplicityhl-lsp.binary.path`, then PATH, then the
    /// managed install. The shell env lets the server find `simfmt`.
    fn language_server_command(&mut self, id: &LanguageServerId, worktree: &Worktree) -> zed::Result<zed::Command> {
        let env = worktree.shell_env();
        let binary = lsp_settings(worktree).binary;
        if let Some(path) = binary.as_ref().and_then(|b| b.path.clone()) {
            let args = binary.and_then(|b| b.arguments).unwrap_or_default();
            return Ok(zed::Command { command: path, args, env });
        }
        if let Some(path) = worktree.which(SERVER_ID) {
            return Ok(zed::Command { command: path, args: vec![], env });
        }
        Ok(zed::Command { command: self.managed_binary(id, worktree)?, args: vec![], env })
    }

    fn language_server_workspace_configuration(
        &mut self,
        _id: &LanguageServerId,
        worktree: &Worktree,
    ) -> zed::Result<Option<Value>> {
        Ok(Some(workspace_configuration(lsp_settings(worktree).settings)))
    }

    fn language_server_initialization_options(
        &mut self,
        _id: &LanguageServerId,
        worktree: &Worktree,
    ) -> zed::Result<Option<Value>> {
        let settings = lsp_settings(worktree);
        Ok(initialization_options(settings.initialization_options, settings.settings))
    }
}

zed::register_extension!(SimplicityHlExtension);
