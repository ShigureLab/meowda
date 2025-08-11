/// Provides a user-level directory for storing application state.
/// Heavy inspiration from the uv implementation.
use crate::envs::EnvVars;
use crate::store::file_lock::FileLock;
use anyhow::{Context, Result};
use etcetera::BaseStrategy;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Returns an appropriate user-level directory for storing application state.
///
/// Corresponds to `$XDG_DATA_HOME/meowda` on Unix.
fn user_state_dir() -> Option<PathBuf> {
    etcetera::base_strategy::choose_base_strategy()
        .ok()
        .map(|dirs| dirs.data_dir().join("meowda"))
}

pub fn resolve_parent_path(path: &Path, parent_level: u8) -> Result<PathBuf> {
    let mut current = path;
    for _ in 0..parent_level {
        current = current.parent().ok_or_else(|| {
            anyhow::anyhow!(
                "Cannot resolve parent path: {} with parent level {}",
                path.display(),
                parent_level
            )
        })?;
    }
    Ok(current.to_path_buf())
}

pub fn get_candidate_scopes(scope_type: ScopeType) -> Result<Vec<VenvScope>> {
    let current_dir = std::env::current_dir().context("Failed to get current working directory")?;
    let search_local = matches!(scope_type, ScopeType::Local | ScopeType::Unspecified);
    let search_global = matches!(scope_type, ScopeType::Global | ScopeType::Unspecified);

    let mut scopes = Vec::new();
    if search_local {
        for parent_level in 0.. {
            match resolve_parent_path(current_dir.as_path(), parent_level) {
                Ok(_) => scopes.push(VenvScope::Local(parent_level)),
                Err(_) => break, // Stop if we can't resolve any further parent directories
            }
        }
    }
    if search_global {
        scopes.push(VenvScope::Global);
    }
    Ok(scopes)
}

pub enum ScopeType {
    Local,
    Global,
    Unspecified,
}

#[derive(PartialEq, Eq, Clone)]
pub enum VenvScope {
    Local(u8),
    Global,
}

pub struct VenvStore {
    path: PathBuf,
}

impl VenvStore {
    /// Detects the local venv directory in the current working directory.
    ///
    /// Prefer, in order:
    /// 1. The specific tool directory specified by the user, i.e., `MEOWDA_LOCAL_VENV_DIR`
    /// 2. A directory in the local data directory, e.g., `./.meowda/venvs`
    fn local_path(parent_level: u8) -> Result<PathBuf> {
        let local_venv_dir_path = if let Some(local_venv_dir) =
            std::env::var_os(EnvVars::MEOWDA_LOCAL_VENV_DIR).filter(|s| !s.is_empty())
        {
            if PathBuf::from(local_venv_dir.clone()).is_absolute() {
                return std::path::absolute(local_venv_dir).with_context(|| {
                    "Invalid path for `MEOWDA_LOCAL_VENV_DIR` environment variable".to_string()
                });
            }
            PathBuf::from(local_venv_dir)
        } else {
            PathBuf::from(".meowda").join("venvs")
        };
        let current_dir =
            std::env::current_dir().context("Failed to get current working directory")?;
        Ok(resolve_parent_path(&current_dir, parent_level)?.join(local_venv_dir_path))
    }

    /// Detects the global venv directory in the current working directory.
    ///
    /// Prefer, in order:
    ///
    /// 1. The specific tool directory specified by the user, i.e., `MEOWDA_GLOBAL_VENV_DIR`
    /// 2. A directory in the system-appropriate user-level data directory, e.g., `~/.local/meowda/venvs`
    fn global_path() -> Result<PathBuf> {
        if let Some(tool_dir) =
            std::env::var_os(EnvVars::MEOWDA_GLOBAL_VENV_DIR).filter(|s| !s.is_empty())
        {
            std::path::absolute(tool_dir).with_context(|| {
                "Invalid path for `MEOWDA_GLOBAL_VENV_DIR` environment variable".to_string()
            })
        } else {
            user_state_dir()
                .map(|dir| dir.join("venvs"))
                .ok_or_else(|| anyhow::anyhow!("Failed to determine user state directory"))
        }
    }

    pub fn from_specified_scope(scope: VenvScope) -> Result<Self> {
        let path = match scope {
            VenvScope::Local(parent_level) => Self::local_path(parent_level)?,
            VenvScope::Global => Self::global_path()?,
        };
        Ok(VenvStore { path })
    }

    pub fn from_scope_type(scope_type: ScopeType) -> Result<Self> {
        let path = match scope_type {
            ScopeType::Local => Self::local_path(0)?,
            ScopeType::Global => Self::global_path()?,
            ScopeType::Unspecified => Self::global_path()?,
        };
        Ok(VenvStore { path })
    }

    pub fn is_ready(&self) -> bool {
        self.path.exists() && self.path.is_dir() && self.path.join(".gitignore").exists()
    }

    pub fn init(&self) -> io::Result<()> {
        std::fs::create_dir_all(&self.path)?;

        // Add a .gitignore.
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.path.join(".gitignore"))
        {
            Ok(mut file) => file.write_all(b"*"),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Ok(()),
            Err(err) => Err(err),
        }
    }

    pub fn init_if_needed(&self) -> Result<()> {
        if !self.is_ready() {
            self.init().context("Failed to initialize venv store")?;
        }
        Ok(())
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn exists(&self, name: &str) -> bool {
        self.path.join(name).exists()
    }

    pub fn contains(&self, path: impl AsRef<Path>) -> Result<bool> {
        Ok(path.as_ref().starts_with(self.path()))
    }

    pub async fn lock(&self) -> Result<FileLock> {
        let lock_path = self.path.join(".lock");
        FileLock::acquire(lock_path, "venv_store")
            .await
            .context("Failed to acquire lock for VenvStore")
    }
}
