use crate::cli::args::{InstallArgs, UninstallArgs};
use crate::venv::VenvService;
use anyhow::Result;

pub async fn install(args: InstallArgs, venv_service: &VenvService) -> Result<()> {
    let extra_args: Vec<&str> = args.extra_args.iter().map(|s| s.as_str()).collect();
    venv_service.install(&extra_args).await?;
    Ok(())
}

pub async fn uninstall(args: UninstallArgs, venv_service: &VenvService) -> Result<()> {
    let extra_args: Vec<&str> = args.extra_args.iter().map(|s| s.as_str()).collect();
    venv_service.uninstall(&extra_args).await?;
    Ok(())
}
