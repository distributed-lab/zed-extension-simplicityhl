mod server;

use std::fs;

use zed_extension_api::{self as zed, LanguageServerId, LanguageServerInstallationStatus, Worktree};

use server::{binary_path, cargo_install_args, version_dir, PINNED_VERSION};

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

fn is_file(path: &str) -> bool {
    fs::metadata(path).is_ok_and(|m| m.is_file())
}

impl zed::Extension for SimplicityHlExtension {
    fn new() -> Self {
        SimplicityHlExtension
    }

    fn language_server_command(&mut self, id: &LanguageServerId, worktree: &Worktree) -> zed::Result<zed::Command> {
        Ok(zed::Command {
            command: self.managed_binary(id, worktree)?,
            args: vec![],
            env: worktree.shell_env(),
        })
    }
}

zed::register_extension!(SimplicityHlExtension);
