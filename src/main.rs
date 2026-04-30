mod cli;
mod envs;
mod store;
mod venv;
use anstream::eprintln;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = cli::args::Args::parse();
    let venv_service = match venv::VenvService::new() {
        Ok(venv_service) => venv_service,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let result = match args.command {
        cli::args::Commands::Create(create_args) => {
            cli::env::create(create_args, &venv_service).await
        }
        cli::args::Commands::Fork(fork_args) => cli::fork::fork(fork_args, &venv_service).await,
        cli::args::Commands::Remove(remove_args) => {
            cli::env::remove(remove_args, &venv_service).await
        }
        cli::args::Commands::Env(env_args) => match env_args {
            cli::args::EnvCommandsArgs::Create(create_args) => {
                cli::env::create(create_args, &venv_service).await
            }
            cli::args::EnvCommandsArgs::Fork(fork_args) => {
                cli::fork::fork(fork_args, &venv_service).await
            }
            cli::args::EnvCommandsArgs::Remove(remove_args) => {
                cli::env::remove(remove_args, &venv_service).await
            }
            cli::args::EnvCommandsArgs::List(list_args) => {
                cli::env::list(list_args, &venv_service).await
            }
            cli::args::EnvCommandsArgs::Dir(dir_args) => {
                cli::env::dir(dir_args, &venv_service).await
            }
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
            cli::install::install(install_args, &venv_service).await
        }
        cli::args::Commands::Uninstall(uninstall_args) => {
            cli::install::uninstall(uninstall_args, &venv_service).await
        }
        cli::args::Commands::Link(link_args) => cli::link::link(link_args, &venv_service).await,
        cli::args::Commands::Unlink(unlink_args) => {
            cli::link::unlink(unlink_args, &venv_service).await
        }
    };

    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }

    Ok(())
}
