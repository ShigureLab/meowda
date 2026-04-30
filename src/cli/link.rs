use crate::cli::args::{LinkArgs, UnlinkArgs};
use crate::venv::VenvService;
use anyhow::Result;

pub async fn link(args: LinkArgs, venv_service: &VenvService) -> Result<()> {
    venv_service.link(&args.name, &args.path).await?;
    Ok(())
}

pub async fn unlink(args: UnlinkArgs, venv_service: &VenvService) -> Result<()> {
    venv_service.unlink(&args.name).await?;
    Ok(())
}
