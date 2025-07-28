use crate::backend::{EnvInfo, VenvBackend};
use crate::cli::args::{CreateArgs, DirArgs, ListArgs, RemoveArgs};
use crate::store::venv_store::VenvScope;
use anstream::println;
use anyhow::Result;
use owo_colors::OwoColorize;

pub async fn create(args: CreateArgs, backend: &VenvBackend) -> Result<()> {
    let scope = crate::cli::utils::parse_scope(&args.scope)?;
    backend
        .create(&args.name, &args.python, args.clear, scope)
        .await?;
    println!("Virtual environment '{}' created successfully.", args.name);
    Ok(())
}

pub async fn remove(args: RemoveArgs, backend: &VenvBackend) -> Result<()> {
    let scope = crate::cli::utils::parse_scope(&args.scope)?;
    backend.remove(&args.name, scope).await?;
    println!("Virtual environment '{}' removed successfully.", args.name);
    Ok(())
}

fn show_envs(envs: &[EnvInfo], scope: &VenvScope) -> Result<()> {
    let scope_name = match scope {
        VenvScope::Local => "local",
        VenvScope::Global => "global",
    };
    if envs.is_empty() {
        println!("No virtual {scope_name} environments found.");
    } else {
        println!("Available virtual {scope_name} environments:");
        for env in envs {
            let indicator = if env.is_active { "* " } else { "  " };
            let name_display = format!("{}{}", indicator, env.name);
            if env.is_active {
                println!(
                    "{} ({})",
                    name_display.green().bold(),
                    env.path.display().blue()
                );
            } else {
                println!("{} ({})", name_display, env.path.display().blue());
            }
        }
    }
    Ok(())
}

pub async fn list(args: ListArgs, backend: &VenvBackend) -> Result<()> {
    let (local_envs, global_envs) = backend.list().await?;
    let scope = crate::cli::utils::parse_scope(&args.scope)?;
    if scope.is_none() || scope == Some(VenvScope::Local) {
        show_envs(&local_envs, &VenvScope::Local)?;
    }
    if scope.is_none() || scope == Some(VenvScope::Global) {
        show_envs(&global_envs, &VenvScope::Global)?;
    }
    Ok(())
}

pub async fn dir(args: DirArgs, backend: &VenvBackend) -> Result<()> {
    let scope = crate::cli::utils::parse_scope(&args.scope)?;
    let path = backend.dir(scope)?;
    println!("{}", path.display());
    Ok(())
}
