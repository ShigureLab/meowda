use crate::cli::args::ActivateArgs;
use anstream::panic;

pub async fn activate(_args: ActivateArgs) {
    panic!("Please run `meowda init <shell_profile>` to set up the activation script.");
}

pub async fn deactivate() {
    panic!("Please run `meowda init <shell_profile>` to set up the activation script.");
}
