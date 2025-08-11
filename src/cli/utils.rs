use crate::store::venv_store::{ScopeType, VenvScope, VenvStore, get_candidate_scopes};

pub fn search_venv(scope_type: ScopeType, env_name: &str) -> anyhow::Result<VenvScope> {
    let search_local = matches!(scope_type, ScopeType::Local | ScopeType::Unspecified);
    let search_global = matches!(scope_type, ScopeType::Global | ScopeType::Unspecified);
    let scopes = get_candidate_scopes(scope_type)?;

    for scope in scopes {
        let venv_store = VenvStore::from_specified_scope(scope.clone())?;
        if venv_store.is_ready() && venv_store.exists(env_name) {
            return Ok(scope);
        }
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
