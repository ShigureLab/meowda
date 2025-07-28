use crate::backend::VenvBackend;
use crate::cli::args::{InstallArgs, UninstallArgs};
use anyhow::Result;

pub async fn install(args: InstallArgs, backend: &VenvBackend) -> Result<()> {
    let extra_args: Vec<&str> = args.extra_args.iter().map(|s| s.as_str()).collect();
    backend.install(&extra_args).await?;
    Ok(())
}

pub async fn uninstall(args: UninstallArgs, backend: &VenvBackend) -> Result<()> {
    let extra_args: Vec<&str> = args.extra_args.iter().map(|s| s.as_str()).collect();
    backend.uninstall(&extra_args).await?;
    Ok(())
}
