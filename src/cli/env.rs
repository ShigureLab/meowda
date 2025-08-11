use crate::backend::{EnvInfo, VenvBackend};
use crate::cli::args::{CreateArgs, DirArgs, ListArgs, RemoveArgs};
use crate::store::venv_store::{ScopeType, VenvScope, VenvStore};
use anstream::println;
use anyhow::Result;
use owo_colors::OwoColorize;

pub async fn create(args: CreateArgs, backend: &VenvBackend) -> Result<()> {
    let scope_type = args.scope.try_into_scope_type()?;
    let store = VenvStore::from_scope_type(scope_type)?;
    store.init_if_needed()?;
    backend
        .create(&store, &args.name, &args.python, args.clear)
        .await?;
    println!("Virtual environment '{}' created successfully.", args.name);
    Ok(())
}

pub async fn remove(args: RemoveArgs, backend: &VenvBackend) -> Result<()> {
    let scope_type = args.scope.try_into_scope_type()?;
    let detected_venv_scope = crate::cli::utils::search_venv(scope_type, &args.name)?;
    let store = VenvStore::from_specified_scope(detected_venv_scope)?;
    if !store.exists(&args.name) {
        anyhow::bail!(
            "Virtual environment '{}' does not exist in the specified scope.",
            args.name
        );
    }
    backend.remove(&store, &args.name).await?;
    println!("Virtual environment '{}' removed successfully.", args.name);
    Ok(())
}

fn show_envs(envs: &[EnvInfo], shadowed_names: &[String]) -> Result<()> {
    if envs.is_empty() {
        return Ok(());
    }
    for env in envs {
        let indicator = if env.is_active { "* " } else { "  " };
        let mut name_display = format!("{}{}", indicator, env.name);
        let mut info_display = env.path.display().blue().to_string();
        if shadowed_names.contains(&env.name) && !env.is_active {
            name_display = name_display.dimmed().to_string();
        }
        if env.is_active {
            name_display = name_display.green().bold().to_string();
        }
        if let Some(config) = &env.config
            && let Some(version) = &config.version
        {
            info_display = format!(
                "{} {}",
                info_display,
                format!("python {version}").cyan().bold()
            );
        }
        println!("{} ({})", name_display, info_display);
    }
    Ok(())
}

pub async fn list(args: ListArgs, backend: &VenvBackend) -> Result<()> {
    let all_envs = backend.list().await?;
    let scope_type = args.scope.try_into_scope_type()?;
    let show_local = matches!(scope_type, ScopeType::Local | ScopeType::Unspecified);
    let show_global = matches!(scope_type, ScopeType::Global | ScopeType::Unspecified);
    let mut shadowed_names = vec![];
    let mut local_title_shown = false;
    for (scope, envs) in all_envs {
        if matches!(scope, VenvScope::Local(_)) && !show_local {
            continue;
        }
        if matches!(scope, VenvScope::Global) && !show_global {
            continue;
        }
        if envs.is_empty() {
            continue;
        }
        if !local_title_shown && matches!(scope, VenvScope::Local(_)) {
            println!("Available local virtual environments:");
            local_title_shown = true;
        }
        if matches!(scope, VenvScope::Global) {
            println!("Available global virtual environments:");
        }
        show_envs(&envs, &shadowed_names)?;
        shadowed_names.extend(envs.iter().map(|env| env.name.clone()));
    }
    Ok(())
}

pub async fn dir(args: DirArgs, backend: &VenvBackend) -> Result<()> {
    let scope_type = args.scope.try_into_scope_type()?;
    let store = VenvStore::from_scope_type(scope_type)?;
    let path = backend.dir(&store)?;
    println!("{}", path.display());
    Ok(())
}
