use crate::backend::VenvBackend;
use crate::cli::args::{LinkArgs, UnlinkArgs};
use anyhow::Result;

pub async fn link(args: LinkArgs, backend: &VenvBackend) -> Result<()> {
    backend.link(&args.name, &args.path).await?;
    Ok(())
}

pub async fn unlink(args: UnlinkArgs, backend: &VenvBackend) -> Result<()> {
    backend.unlink(&args.name).await?;
    Ok(())
}
