use crate::cli::args::ActivateArgs;
use crate::store::venv_store::VenvStore;
use anyhow::Result;

pub async fn activate(_args: ActivateArgs) -> Result<()> {
    anyhow::bail!("Please run `meowda init <shell_profile>` to set up the activation script.");
}

pub async fn deactivate() -> Result<()> {
    anyhow::bail!("Please run `meowda init <shell_profile>` to set up the activation script.");
}

pub async fn detect_activate_venv_path(args: ActivateArgs) -> Result<()> {
    let scope = crate::cli::utils::parse_scope(&args.scope)?;
    let detected_venv_scope = crate::cli::utils::search_venv(scope, &args.name)?;
    let venv_store = VenvStore::create(Some(detected_venv_scope))?;
    let venv_path = venv_store.path().join(&args.name);
    println!("{}", venv_path.display());
    Ok(())
}
