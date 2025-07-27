use clap::{Parser, Subcommand};

#[derive(Parser, Debug, PartialEq)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand, PartialEq)]
pub enum Commands {
    Create(CreateArgs),
    Remove(RemoveArgs),
    #[command(subcommand)]
    Env(EnvCommandsArgs),
    Init(InitArgs),
    Activate(ActivateArgs),
    Deactivate,
    Install(InstallArgs),
    Uninstall(UninstallArgs),
    #[clap(name = "generate-init-script", hide = true)]
    _GenerateInitScript,
}

#[derive(Debug, Parser, PartialEq)]
pub struct CreateArgs {
    pub name: String,
    #[arg(short, long, default_value = "3.13")]
    pub python: String,
    #[arg(short, long, default_value = "false")]
    pub clear: bool,
}

#[derive(Debug, Parser, PartialEq)]
pub struct RemoveArgs {
    pub name: String,
}

#[derive(Debug, Parser, PartialEq)]
pub struct InitArgs {
    pub shell_profile: String,
}

#[derive(Debug, Subcommand, PartialEq)]
pub enum EnvCommandsArgs {
    Create(CreateArgs),
    Remove(RemoveArgs),
    List,
    Dir,
}

#[derive(Debug, Parser, PartialEq)]
pub struct ActivateArgs {
    pub name: String,
}

#[derive(Debug, Parser, PartialEq)]
pub struct InstallArgs {
    #[arg(trailing_var_arg = true)]
    #[arg(num_args = 1..)]
    pub extra_args: Vec<String>,
}

#[derive(Debug, Parser, PartialEq)]
pub struct UninstallArgs {
    #[arg(trailing_var_arg = true)]
    #[arg(num_args = 1..)]
    pub extra_args: Vec<String>,
}
