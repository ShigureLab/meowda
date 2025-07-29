use crate::cli;
use crate::store::venv_store::{VenvScope, VenvStore};

pub fn parse_scope(scope_args: &cli::args::ScopeArgs) -> anyhow::Result<Option<VenvScope>> {
    if scope_args.local && scope_args.global {
        return Err(anyhow::anyhow!(
            "Cannot specify both local and global scopes"
        ));
    }
    if !scope_args.local && !scope_args.global {
        // Unspecified scope
        return Ok(None);
    }
    if scope_args.local {
        return Ok(Some(VenvScope::Local));
    }
    Ok(Some(VenvScope::Global))
}

pub fn search_venv(scope: Option<VenvScope>, env_name: &str) -> anyhow::Result<VenvScope> {
    let local_store = VenvStore::create(Some(VenvScope::Local))?;
    let global_store = VenvStore::create(Some(VenvScope::Global))?;
    let search_local = scope.is_none() || scope == Some(VenvScope::Local);
    let search_global = scope.is_none() || scope == Some(VenvScope::Global);

    if search_local && local_store.is_ready() && local_store.path().join(env_name).exists() {
        return Ok(VenvScope::Local);
    }
    if search_global && global_store.is_ready() && global_store.path().join(env_name).exists() {
        return Ok(VenvScope::Global);
    }
    anyhow::bail!(if search_local && search_global {
        format!("Virtual environment '{env_name}' not found in local or global scope.")
    } else if search_local {
        format!("Virtual environment '{env_name}' not found in local scope.")
    } else if search_global {
        format!("Virtual environment '{env_name}' not found in global scope.")
    } else {
        unreachable!("Unexpected scope combination")
    })
}
