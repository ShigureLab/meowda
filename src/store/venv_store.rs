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
        // For local scope, check if we have a MEOWDA_LOCAL_VENV_DIR override
        if std::env::var_os(crate::envs::EnvVars::MEOWDA_LOCAL_VENV_DIR)
            .filter(|s| !s.is_empty())
            .is_some()
        {
            // If MEOWDA_LOCAL_VENV_DIR is set, only check this store's path
            return self.path.join(name).exists();
        }

        // Check if this is a local store by checking if the path ends with .meowda/venvs
        if self.path.ends_with(".meowda/venvs") {
            // For local stores, search recursively for the environment
            let local_dirs = find_local_venv_dirs();
            for dir in local_dirs {
                if dir.join(name).exists() {
                    return true;
                }
            }
            false
        } else {
            // For global stores, just check the primary path
            self.path.join(name).exists()
        }
    }

    /// Find the specific path where an environment exists (for local stores with recursive search)
    pub fn find_env_path(&self, name: &str) -> Option<PathBuf> {
        // For local scope, check if we have a MEOWDA_LOCAL_VENV_DIR override
        if std::env::var_os(crate::envs::EnvVars::MEOWDA_LOCAL_VENV_DIR)
            .filter(|s| !s.is_empty())
            .is_some()
        {
            // If MEOWDA_LOCAL_VENV_DIR is set, only check this store's path
            let env_path = self.path.join(name);
            return if env_path.exists() {
                Some(env_path)
            } else {
                None
            };
        }

        // Check if this is a local store by checking if the path ends with .meowda/venvs
        if self.path.ends_with(".meowda/venvs") {
            // For local stores, search recursively for the environment
            let local_dirs = find_local_venv_dirs();
            for dir in local_dirs {
                let env_path = dir.join(name);
                if env_path.exists() {
                    return Some(env_path);
                }
            }
            None
        } else {
            // For global stores, just check the primary path
            let env_path = self.path.join(name);
            if env_path.exists() {
                Some(env_path)
            } else {
                None
            }
        }
    }

    /// Get all paths to search for environments (used for listing)
    pub fn all_paths(&self) -> Vec<PathBuf> {
        // For local scope, check if we have a MEOWDA_LOCAL_VENV_DIR override
        if std::env::var_os(crate::envs::EnvVars::MEOWDA_LOCAL_VENV_DIR)
            .filter(|s| !s.is_empty())
            .is_some()
        {
            // If MEOWDA_LOCAL_VENV_DIR is set, only return this store's path
            return vec![self.path.clone()];
        }

        // Check if this is a local store by checking if the path ends with .meowda/venvs
        if self.path.ends_with(".meowda/venvs") {
            // For local stores, return all discovered local paths
            find_local_venv_dirs()
        } else {
            // For global stores, just return the primary path
            vec![self.path.clone()]
        }
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
    fn test_find_local_venv_dirs_with_hierarchy() {
        let temp_dir = TempDir::new().unwrap();
        let original_cwd = std::env::current_dir().unwrap();

        // Create a directory hierarchy with multiple .meowda/venvs
        let root_meowda = temp_dir.path().join(".meowda").join("venvs");
        let sub_meowda = temp_dir.path().join("sub").join(".meowda").join("venvs");
        let deep_dir = temp_dir.path().join("sub").join("deep");

        std::fs::create_dir_all(&root_meowda).unwrap();
        std::fs::create_dir_all(&sub_meowda).unwrap();
        std::fs::create_dir_all(&deep_dir).unwrap();

        // Change to deep directory and test, then immediately restore
        {
            std::env::set_current_dir(&deep_dir).unwrap();

            // Should find both directories in correct order (closest first)
            let result = find_local_venv_dirs();
            assert_eq!(result.len(), 2);
            assert_eq!(result[0], sub_meowda);
            assert_eq!(result[1], root_meowda);

            // Restore original directory before temp_dir is dropped
            std::env::set_current_dir(&original_cwd).unwrap();
        }
    }

    #[test]
    fn test_recursive_env_exists() {
        let temp_dir = TempDir::new().unwrap();
        let original_cwd = std::env::current_dir().unwrap();

        // Create a directory hierarchy
        let root_meowda = temp_dir.path().join(".meowda").join("venvs");
        let sub_meowda = temp_dir.path().join("sub").join(".meowda").join("venvs");
        let deep_dir = temp_dir.path().join("sub").join("deep");

        std::fs::create_dir_all(&root_meowda).unwrap();
        std::fs::create_dir_all(&sub_meowda).unwrap();
        std::fs::create_dir_all(&deep_dir).unwrap();

        // Create test environments
        std::fs::create_dir_all(root_meowda.join("root-env")).unwrap();
        std::fs::create_dir_all(sub_meowda.join("sub-env")).unwrap();
        std::fs::create_dir_all(sub_meowda.join("shadowed-env")).unwrap(); // This will shadow root's version
        std::fs::create_dir_all(root_meowda.join("shadowed-env")).unwrap();

        // Change to deep directory and test, then immediately restore
        {
            std::env::set_current_dir(&deep_dir).unwrap();

            // Create a local store and test recursive search
            let store = VenvStore {
                path: sub_meowda.clone(),
            };

            // Should find environments in both directories
            assert!(store.exists("root-env"));
            assert!(store.exists("sub-env"));
            assert!(store.exists("shadowed-env"));

            // Test find_env_path - should return closest path (shadowing)
            assert_eq!(
                store.find_env_path("sub-env"),
                Some(sub_meowda.join("sub-env"))
            );
            assert_eq!(
                store.find_env_path("shadowed-env"),
                Some(sub_meowda.join("shadowed-env"))
            ); // Should get sub version, not root
            assert_eq!(
                store.find_env_path("root-env"),
                Some(root_meowda.join("root-env"))
            );
            assert_eq!(store.find_env_path("nonexistent"), None);

            // Restore original directory before temp_dir is dropped
            std::env::set_current_dir(&original_cwd).unwrap();
        }
    }
}
