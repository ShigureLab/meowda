use crate::backend::VenvBackend;
use crate::cli::args::{InstallArgs, UninstallArgs};

pub async fn install(args: InstallArgs, backend: &VenvBackend) {
    let extra_args: Vec<&str> = args.extra_args.iter().map(|s| s.as_str()).collect();
    backend.install(&extra_args).await;
}

pub async fn uninstall(args: UninstallArgs, backend: &VenvBackend) {
    let extra_args: Vec<&str> = args.extra_args.iter().map(|s| s.as_str()).collect();
    backend.uninstall(&extra_args).await;
}
