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
    let scope_type = args.scope.try_into_scope_type()?;
    let detected_venv_scope = crate::cli::utils::search_venv(scope_type, &args.name)?;
    let venv_store = VenvStore::from_specified_scope(detected_venv_scope)?;
    let venv_path = venv_store.path().join(&args.name);
    println!("{}", venv_path.display());
    Ok(())
}
