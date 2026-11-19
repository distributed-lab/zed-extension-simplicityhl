mod server;

use std::fs;

use zed_extension_api::{
    self as zed, serde_json::Value, settings::LspSettings, LanguageServerId,
    LanguageServerInstallationStatus, Worktree,
};

use server::{
    binary_path, cargo_install_args, initialization_options, version_dir, workspace_configuration,
    PINNED_VERSION, SERVER_ID,
};

struct SimplicityHlExtension;

impl SimplicityHlExtension {
    /// Returns the absolute path of the managed server binary, installing the
    /// pinned version with cargo first if it isn't there.
    fn managed_binary(&mut self, id: &LanguageServerId, worktree: &Worktree) -> zed::Result<String> {
        let windows = zed::current_platform().0 == zed::Os::Windows;
        let dir = version_dir(PINNED_VERSION);
        let rel_binary = binary_path(&dir, windows);
        let work_dir = std::env::current_dir().map_err(|e| format!("no extension work dir: {e}"))?;
        let abs_binary = work_dir.join(&rel_binary).to_string_lossy().into_owned();

        if is_file(&rel_binary) {
            return Ok(abs_binary);
        }

        zed::set_language_server_installation_status(id, &LanguageServerInstallationStatus::Downloading);
        let root = work_dir.join(&dir).to_string_lossy().into_owned();
        let output = zed::process::Command::new("cargo")
            .args(cargo_install_args(PINNED_VERSION, &root))
            .envs(worktree.shell_env())
            .output();

        let error = match output {
            Ok(out) if out.status == Some(0) && is_file(&rel_binary) => {
                zed::set_language_server_installation_status(id, &LanguageServerInstallationStatus::None);
                return Ok(abs_binary);
            }
            Ok(out) => String::from_utf8_lossy(&out.stderr).into_owned(),
            Err(err) => err,
        };
        zed::set_language_server_installation_status(id, &LanguageServerInstallationStatus::Failed(error.clone()));
        Err(error)
    }
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
