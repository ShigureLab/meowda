use crate::backend::VenvBackend;
use crate::cli::args::{CreateArgs, RemoveArgs};
use anstream::println;
use owo_colors::OwoColorize;

pub async fn create(args: CreateArgs, backend: &VenvBackend) {
    backend.create(&args.name, &args.python, args.clear).await;
    println!("Virtual environment '{}' created successfully.", args.name);
}

pub async fn remove(args: RemoveArgs, backend: &VenvBackend) {
    backend.remove(&args.name).await;
    println!("Virtual environment '{}' removed successfully.", args.name);
}

pub async fn list(backend: &VenvBackend) {
    let envs = backend.list().await;
    if envs.is_empty() {
        println!("No virtual environments found.");
    } else {
        println!("Available virtual environments:");
        for env in envs {
            let indicator = if env.is_active { "* " } else { "  " };
            let name_display = format!("{}{}", indicator, env.name);
            if env.is_active {
                println!("{} ({})", name_display.green().bold(), env.path.display().blue());
            } else {
                println!("{} ({})", name_display, env.path.display().blue());
            }
        }
    }
}

pub async fn dir(backend: &VenvBackend) {
    let path = backend.dir();
    println!("{}", path.display());
}
