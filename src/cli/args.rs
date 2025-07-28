use clap::{Parser, Subcommand};

#[derive(Parser, Debug, PartialEq)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand, PartialEq)]
pub enum Commands {
    #[clap(about = "Create a new virtual environment")]
    Create(CreateArgs),
    #[clap(about = "Remove a virtual environment")]
    Remove(RemoveArgs),
    #[command(subcommand)]
    #[clap(about = "Manage virtual environments")]
    Env(EnvCommandsArgs),
    #[clap(about = "Initialize the shell for Meowda, to get meowda activate/deactivate commands")]
    Init(InitArgs),
    #[clap(about = "Activate a virtual environment")]
    Activate(ActivateArgs),
    #[clap(about = "Deactivate the current virtual environment")]
    Deactivate,
    #[clap(
        about = "Install packages in the current virtual environment (alias for `uv pip install`)"
    )]
    Install(InstallArgs),
    #[clap(
        about = "Uninstall packages from the current virtual environment (alias for `uv pip uninstall`)"
    )]
    Uninstall(UninstallArgs),
    #[clap(name = "generate-init-script", hide = true)]
    _GenerateInitScript,
    #[clap(name = "detect-activate-venv-path", hide = true)]
    _DetectActivateVenvPath(ActivateArgs),
}

#[derive(Debug, Parser, PartialEq)]
pub struct CreateArgs {
    #[arg(help = "Name of the virtual environment")]
    pub name: String,
    #[arg(
        short,
        long,
        default_value = "3.13",
        help = "Python version/path to use"
    )]
    pub python: String,
    #[arg(
        short,
        long,
        default_value = "false",
        help = "Clear existing virtual environment"
    )]
    pub clear: bool,
    #[clap(flatten)]
    pub scope: ScopeArgs,
}

#[derive(Debug, Parser, PartialEq)]
pub struct RemoveArgs {
    #[arg(help = "Name of the virtual environment to remove")]
    pub name: String,
    #[clap(flatten)]
    pub scope: ScopeArgs,
}

#[derive(Debug, Parser, PartialEq)]
pub struct InitArgs {
    #[arg(help = "Path to the shell profile to inject the initialization script")]
    pub shell_profile: String,
}

#[derive(Debug, Subcommand, PartialEq)]
pub enum EnvCommandsArgs {
    #[clap(about = "Create a new virtual environment")]
    Create(CreateArgs),
    #[clap(about = "Remove a virtual environment")]
    Remove(RemoveArgs),
    #[clap(about = "List all virtual environments")]
    List(ListArgs),
    #[clap(about = "Show directory of the virtual environment store")]
    Dir(DirArgs),
}

#[derive(Debug, Parser, PartialEq)]
pub struct ListArgs {
    #[clap(flatten)]
    pub scope: ScopeArgs,
}

#[derive(Debug, Parser, PartialEq)]
pub struct DirArgs {
    #[clap(flatten)]
    pub scope: ScopeArgs,
}

#[derive(Debug, Parser, PartialEq)]
pub struct ActivateArgs {
    #[arg(help = "Name of the virtual environment to activate")]
    pub name: String,
    #[clap(flatten)]
    pub scope: ScopeArgs,
}

#[derive(Debug, Parser, PartialEq)]
pub struct InstallArgs {
    #[arg(trailing_var_arg = true)]
    #[arg(num_args = 1..)]
    #[clap(
        help = "Install packages in the current virtual environment, the arguments are passed to the `uv pip install` command"
    )]
    pub extra_args: Vec<String>,
}

#[derive(Debug, Parser, PartialEq)]
pub struct UninstallArgs {
    #[arg(trailing_var_arg = true)]
    #[arg(num_args = 1..)]
    #[clap(
        help = "Uninstall packages from the current virtual environment, the arguments are passed to the `uv pip uninstall` command"
    )]
    pub extra_args: Vec<String>,
}

#[derive(Debug, Parser, PartialEq)]
pub struct ScopeArgs {
    #[arg(long, help = "Select local virtual environment")]
    pub local: bool,
    #[arg(long, help = "Select global virtual environment")]
    pub global: bool,
}
