mod backend;
mod cli;
mod envs;
mod store;
use anstream::eprintln;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = cli::args::Args::parse();
    let backend = match backend::VenvBackend::new() {
        Ok(backend) => backend,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let result = match args.command {
        cli::args::Commands::Create(create_args) => cli::env::create(create_args, &backend).await,
        cli::args::Commands::Remove(remove_args) => cli::env::remove(remove_args, &backend).await,
        cli::args::Commands::Env(env_args) => match env_args {
            cli::args::EnvCommandsArgs::Create(create_args) => {
                cli::env::create(create_args, &backend).await
            }
            cli::args::EnvCommandsArgs::Remove(remove_args) => {
                cli::env::remove(remove_args, &backend).await
            }
            cli::args::EnvCommandsArgs::List(list_args) => {
                cli::env::list(list_args, &backend).await
            }
            cli::args::EnvCommandsArgs::Dir(dir_args) => cli::env::dir(dir_args, &backend).await,
        },
        cli::args::Commands::Init(init_args) => cli::init::init(init_args).await,
        cli::args::Commands::_GenerateInitScript => cli::init::generate_init_script().await,
        cli::args::Commands::Activate(activate_args) => {
            cli::activate::activate(activate_args).await
        }
        cli::args::Commands::Deactivate => cli::activate::deactivate().await,
        cli::args::Commands::_DetectActivateVenvPath(activate_args) => {
            cli::activate::detect_activate_venv_path(activate_args).await
        }
        cli::args::Commands::Install(install_args) => {
            cli::install::install(install_args, &backend).await
        }
        cli::args::Commands::Uninstall(uninstall_args) => {
            cli::install::uninstall(uninstall_args, &backend).await
        }
    };

    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }

    Ok(())
}
