use crate::cli::args::ForkArgs;
use crate::store::venv_store::VenvStore;
use crate::venv::{ForkOptions, VenvService};
use anstream::println;
use anyhow::Result;

pub async fn fork(args: ForkArgs, venv_service: &VenvService) -> Result<()> {
    let scope_type = args.scope.try_into_scope_type()?;
    let store = VenvStore::from_scope_type(scope_type)?;
    store.init_if_needed()?;
    venv_service
        .fork(
            &store,
            &args.name,
            ForkOptions {
                scope_type,
                source: args.source.as_deref(),
                clear: args.clear,
            },
        )
        .await?;
    println!("Virtual environment '{}' forked successfully.", args.name);
    Ok(())
}
