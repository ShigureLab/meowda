use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub(super) fn create_uv_venv(
    uv_path: &str,
    venv_path: &Path,
    python: &str,
    seed: bool,
    include_system_site_packages: bool,
) -> Result<()> {
    let venv_path_str = venv_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid path for virtual environment"))?;

    let mut command = Command::new(uv_path);
    command.args(["venv", venv_path_str, "--python", python]);
    if seed {
        command.arg("--seed");
    }
    if include_system_site_packages {
        command.arg("--system-site-packages");
    }

    let status = command.status().context("Failed to execute uv command")?;
    if !status.success() {
        anyhow::bail!(
            "Failed to create virtual environment. Check Python version/source environment and try again"
        );
    }

    Ok(())
}
