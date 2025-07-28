use crate::cli::args::ActivateArgs;
use crate::store::venv_store::{VenvScope, VenvStore};
use anyhow::Result;

pub async fn activate(_args: ActivateArgs) -> Result<()> {
    anyhow::bail!("Please run `meowda init <shell_profile>` to set up the activation script.");
}

pub async fn deactivate() -> Result<()> {
    anyhow::bail!("Please run `meowda init <shell_profile>` to set up the activation script.");
}

pub async fn detect_activate_venv_path(args: ActivateArgs) -> Result<()> {
    let scope = crate::cli::utils::parse_scope(&args.scope)?;
    let local_store = VenvStore::create(Some(VenvScope::Local))?;
    let global_store = VenvStore::create(Some(VenvScope::Global))?;
    let search_local = scope.is_none() || scope == Some(VenvScope::Local);
    let search_global = scope.is_none() || scope == Some(VenvScope::Global);
    let env_name = &args.name;

    if search_local && local_store.is_ready() && local_store.path().join(env_name).exists() {
        println!("{}", local_store.path().join(env_name).display());
        return Ok(());
    }
    if search_global && global_store.is_ready() && global_store.path().join(env_name).exists() {
        println!("{}", global_store.path().join(env_name).display());
        return Ok(());
    }

    anyhow::bail!(if search_local && search_global {
        format!("Virtual environment '{env_name}' not found in local or global scope.")
    } else if search_local {
        format!("Virtual environment '{env_name}' not found in local scope.")
    } else if search_global {
        format!("Virtual environment '{env_name}' not found in global scope.")
    } else {
        unreachable!("Unexpected scope combination")
    })
}
