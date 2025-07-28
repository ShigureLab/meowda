use crate::store::venv_store::VenvStore;
use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::info;

#[derive(Debug, Clone)]
pub struct EnvInfo {
    pub name: String,
    pub path: PathBuf,
    pub is_active: bool,
}

pub struct VenvBackend {
    uv_path: String,
}

impl VenvBackend {
    pub fn new() -> Result<Self> {
        let uv_path = "uv";
        if !Self::check_uv_available(uv_path) {
            anyhow::bail!(
                "uv is not available, please install it first. See https://docs.astral.sh/uv/getting-started/installation/"
            );
        }

        Ok(VenvBackend {
            uv_path: uv_path.to_string(),
        })
    }

    fn check_uv_available(uv_path: &str) -> bool {
        // check if uv is available by commanding `uv --version`
        Command::new(uv_path)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn get_venv_store() -> Result<VenvStore> {
        let store = VenvStore::new()?;
        if !store.is_ready() {
            store.init().context("Failed to initialize venv store")?;
        }
        Ok(store)
    }

    fn remove_venv(store: &VenvStore, name: &str) -> Result<()> {
        std::fs::remove_dir_all(store.path().join(name))
            .context("Failed to remove virtual environment")?;
        Ok(())
    }

    fn detect_current_venv() -> Option<PathBuf> {
        std::env::var("VIRTUAL_ENV")
            .ok()
            .and_then(|s| std::path::absolute(PathBuf::from(s)).ok())
    }

    fn contains(&self, path: impl AsRef<Path>) -> Result<bool> {
        let store = Self::get_venv_store()?;
        Ok(path.as_ref().starts_with(store.path()))
    }

    // Venv management methods
    pub async fn create(&self, name: &str, python: &str, clear: bool) -> Result<()> {
        let store = Self::get_venv_store()?;
        let _lock = store.lock().await?;
        if store.exists(name) {
            if clear {
                Self::remove_venv(&store, name)?;
            } else {
                anyhow::bail!("Virtual environment '{}' already exists", name);
            }
        }
        let venv_path = store.path().join(name);
        let venv_path_str = venv_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid path for virtual environment"))?;

        let status = Command::new(&self.uv_path)
            .args(["venv", venv_path_str, "--python", python, "--seed"])
            .status()
            .context("Failed to execute uv command")?;

        if !status.success() {
            anyhow::bail!("Failed to create virtual environment");
        }

        info!(
            "Created virtual environment '{}' at {}",
            name.green(),
            venv_path_str.blue()
        );
        Ok(())
    }
    pub async fn remove(&self, name: &str) -> Result<()> {
        let store = Self::get_venv_store()?;
        let _lock = store.lock().await?;
        if !store.exists(name) {
            anyhow::bail!("Virtual environment '{}' does not exist", name);
        }
        Self::remove_venv(&store, name)?;
        info!("Removed virtual environment '{}'", name.green());
        Ok(())
    }
    pub async fn list(&self) -> Result<Vec<EnvInfo>> {
        let store = Self::get_venv_store()?;
        let _lock = store.lock().await?;
        let current_venv = Self::detect_current_venv();

        let entries = store
            .path()
            .read_dir()
            .context("Failed to read venv directory")?
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    if e.path().is_dir() {
                        e.file_name().to_str().map(|name| {
                            let env_path = e.path();
                            let is_active = if let Some(ref current) = current_venv {
                                // Compare the actual environment paths
                                env_path.canonicalize().ok() == current.canonicalize().ok()
                            } else {
                                false
                            };

                            EnvInfo {
                                name: name.to_string(),
                                path: env_path,
                                is_active,
                            }
                        })
                    } else {
                        None
                    }
                })
            })
            .collect();
        Ok(entries)
    }

    // Package management methods
    pub async fn install(&self, extra_args: &[&str]) -> Result<()> {
        let store = Self::get_venv_store()?;
        let _lock = store.lock().await?;
        if !store.path().exists() {
            anyhow::bail!("Virtual environment does not exist");
        }
        let current_venv = Self::detect_current_venv()
            .ok_or_else(|| anyhow::anyhow!("No virtual environment is currently activated"))?;

        if !self.contains(current_venv.clone())? {
            anyhow::bail!(
                "Current virtual environment ({}) is not managed by this backend ({})",
                current_venv.display(),
                store.path().display()
            );
        }

        let status = Command::new(&self.uv_path)
            .args(["pip", "install"])
            .args(extra_args)
            .status()
            .context("Failed to execute uv pip install command")?;

        if !status.success() {
            anyhow::bail!("Failed to install packages");
        }

        info!("Packages installed successfully.");
        Ok(())
    }
    pub async fn uninstall(&self, extra_args: &[&str]) -> Result<()> {
        let store = Self::get_venv_store()?;
        let _lock = store.lock().await?;
        if !store.path().exists() {
            anyhow::bail!("Virtual environment does not exist");
        }
        let current_venv = Self::detect_current_venv()
            .ok_or_else(|| anyhow::anyhow!("No virtual environment is currently activated"))?;

        if !self.contains(current_venv.clone())? {
            anyhow::bail!(
                "Current virtual environment ({}) is not managed by this backend ({})",
                current_venv.display(),
                store.path().display()
            );
        }

        let status = Command::new(&self.uv_path)
            .args(["pip", "uninstall"])
            .args(extra_args)
            .status()
            .context("Failed to execute uv pip uninstall command")?;

        if !status.success() {
            anyhow::bail!("Failed to uninstall packages");
        }

        info!("Packages uninstalled successfully.");
        Ok(())
    }

    // File management methods
    pub fn dir(&self) -> Result<PathBuf> {
        let store = Self::get_venv_store()?;
        Ok(store.path().clone())
    }
}
