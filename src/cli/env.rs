use crate::backend::VenvBackend;
use crate::cli::args::{CreateArgs, RemoveArgs};
use anstream::println;
use anyhow::Result;
use owo_colors::OwoColorize;

pub async fn create(args: CreateArgs, backend: &VenvBackend) -> Result<()> {
    backend.create(&args.name, &args.python, args.clear).await?;
    println!("Virtual environment '{}' created successfully.", args.name);
    Ok(())
}

pub async fn remove(args: RemoveArgs, backend: &VenvBackend) -> Result<()> {
    backend.remove(&args.name).await?;
    println!("Virtual environment '{}' removed successfully.", args.name);
    Ok(())
}

pub async fn list(backend: &VenvBackend) -> Result<()> {
    let envs = backend.list().await?;
    if envs.is_empty() {
        println!("No virtual environments found.");
    } else {
        println!("Available virtual environments:");
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

pub async fn dir(backend: &VenvBackend) -> Result<()> {
    let path = backend.dir()?;
    println!("{}", path.display());
    Ok(())
}
