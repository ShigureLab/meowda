use crate::cli::args::ActivateArgs;
use anyhow::Result;

pub async fn activate(_args: ActivateArgs) -> Result<()> {
    anyhow::bail!("Please run `meowda init <shell_profile>` to set up the activation script.");
}

pub async fn deactivate() -> Result<()> {
    anyhow::bail!("Please run `meowda init <shell_profile>` to set up the activation script.");
}
