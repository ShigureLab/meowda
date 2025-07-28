use crate::cli;
use crate::store::venv_store::VenvScope;

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
