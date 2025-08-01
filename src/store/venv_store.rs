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

/// Recursively searches for `.meowda/venvs` directories from current directory up to root.
/// Returns all found directories in order from most specific (current dir) to least specific.
/// Silently skips directories that cannot be accessed due to permission errors.
fn find_local_venv_dirs() -> Vec<PathBuf> {
    let mut venv_dirs = Vec::new();

    // Start from current working directory
    let mut current_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(_) => return venv_dirs,
    };

    loop {
        let meowda_venvs = current_dir.join(".meowda").join("venvs");

        // Check if the .meowda/venvs directory exists and is accessible
        if meowda_venvs.exists() && meowda_venvs.is_dir() {
            // Try to read the directory to ensure we have permissions
            if meowda_venvs.read_dir().is_ok() {
                venv_dirs.push(meowda_venvs);
            }
        }

        // Move to parent directory
        match current_dir.parent() {
            Some(parent) => current_dir = parent.to_path_buf(),
            None => break, // Reached root directory
        }
    }

    venv_dirs
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum VenvScope {
    Local,
    Global,
}

pub struct VenvStore {
    path: PathBuf,
    // For local scope, this contains all discovered local venv directories
    // For global scope, this is empty
    additional_local_paths: Vec<PathBuf>,
}

impl VenvStore {
    /// Detects the local venv directory in the current working directory.
    ///
    /// Prefer, in order:
    /// 1. The specific tool directory specified by the user, i.e., `MEOWDA_LOCAL_VENV_DIR`
    /// 2. The first `.meowda/venvs` directory found recursively from current directory up to root
    fn local_path() -> Result<PathBuf> {
        if let Some(tool_dir) =
            std::env::var_os(EnvVars::MEOWDA_LOCAL_VENV_DIR).filter(|s| !s.is_empty())
        {
            std::path::absolute(tool_dir).with_context(|| {
                "Invalid path for `MEOWDA_LOCAL_VENV_DIR` environment variable".to_string()
            })
        } else {
            let local_dirs = find_local_venv_dirs();
            if let Some(first_dir) = local_dirs.first() {
                Ok(first_dir.clone())
            } else {
                // Fall back to current directory if no existing .meowda/venvs found
                let current_dir =
                    std::env::current_dir().context("Failed to get current working directory")?;
                Ok(current_dir.join(".meowda").join("venvs"))
            }
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
        let path = Self::detect_path(scope.clone())?;
        let additional_local_paths = if scope == Some(VenvScope::Local) {
            // For local scope, get all local venv directories except the primary one
            let mut all_local_paths = find_local_venv_dirs();
            // Remove the primary path if it exists in the list
            all_local_paths.retain(|p| p != &path);
            all_local_paths
        } else {
            Vec::new()
        };

        Ok(VenvStore {
            path,
            additional_local_paths,
        })
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

    /// Returns all local venv directories for local scope, or just the primary path for global scope
    pub fn all_paths(&self) -> Vec<&PathBuf> {
        let mut paths = vec![&self.path];
        paths.extend(self.additional_local_paths.iter());
        paths
    }

    pub fn exists(&self, name: &str) -> bool {
        // Check primary path first
        if self.path.join(name).exists() {
            return true;
        }
        // Check additional local paths for local scope
        for path in &self.additional_local_paths {
            if path.join(name).exists() {
                return true;
            }
        }
        false
    }

    /// Find the specific path where an environment exists
    pub fn find_env_path(&self, name: &str) -> Option<PathBuf> {
        // Check primary path first
        let primary_env_path = self.path.join(name);
        if primary_env_path.exists() {
            return Some(primary_env_path);
        }
        // Check additional local paths
        for path in &self.additional_local_paths {
            let env_path = path.join(name);
            if env_path.exists() {
                return Some(env_path);
            }
        }
        None
    }

    pub fn contains(&self, path: impl AsRef<Path>) -> Result<bool> {
        let path_ref = path.as_ref();
        // Check primary path
        if path_ref.starts_with(self.path()) {
            return Ok(true);
        }
        // Check additional local paths
        for local_path in &self.additional_local_paths {
            if path_ref.starts_with(local_path) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub async fn lock(&self) -> Result<FileLock> {
        let lock_path = self.path.join(".lock");
        FileLock::acquire(lock_path, "venv_store")
            .await
            .context("Failed to acquire lock for VenvStore")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_find_local_venv_dirs_in_tempdir() {
        let temp_dir = TempDir::new().unwrap();
        let original_cwd = std::env::current_dir().unwrap();
        
        // Change to temp directory for testing
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        // Test should return empty vector when no .meowda/venvs exists
        let result = find_local_venv_dirs();
        assert_eq!(result.len(), 0);
        
        // Restore original directory
        std::env::set_current_dir(original_cwd).unwrap();
    }
    
    #[test]
    fn test_venv_store_creation() {
        // Test creating global store
        let global_store = VenvStore::create(Some(VenvScope::Global));
        assert!(global_store.is_ok());
        let global_store = global_store.unwrap();
        assert_eq!(global_store.additional_local_paths.len(), 0);
        
        // Test creating local store  
        let local_store = VenvStore::create(Some(VenvScope::Local));
        assert!(local_store.is_ok());
        // Should work even if no additional paths found
    }
}
