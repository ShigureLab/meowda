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

#[derive(PartialEq, Eq)]
pub enum VenvScope {
    Local,
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
    fn local_path() -> Result<PathBuf> {
        if let Some(tool_dir) =
            std::env::var_os(EnvVars::MEOWDA_LOCAL_VENV_DIR).filter(|s| !s.is_empty())
        {
            std::path::absolute(tool_dir).with_context(|| {
                "Invalid path for `MEOWDA_LOCAL_VENV_DIR` environment variable".to_string()
            })
        } else {
            let current_dir =
                std::env::current_dir().context("Failed to get current working directory")?;
            Ok(current_dir.join(".meowda").join("venvs"))
        }
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

    /// Detects the appropriate directory for storing virtual environments.
    fn detect_path(venv_scope: Option<VenvScope>) -> Result<PathBuf> {
        match venv_scope {
            Some(VenvScope::Local) => Self::local_path(),
            Some(VenvScope::Global) => Self::global_path(),
            None => {
                // Default to global if no scope is specified
                Self::global_path()
            }
        }
    }

    pub fn create(scope: Option<VenvScope>) -> Result<Self> {
        let path = Self::detect_path(scope)?;
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
