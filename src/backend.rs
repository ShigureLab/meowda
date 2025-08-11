use crate::store::venv_store::{ScopeType, VenvScope, VenvStore, get_candidate_scopes};
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
    pub config: Option<EnvConfig>,
}

/// Parsed `pyvenv.cfg`, refer to uv `PyVenvConfiguration`
#[derive(Debug, Clone)]
pub struct EnvConfig {
    #[allow(dead_code)]
    pub virtualenv: bool,
    #[allow(dead_code)]
    pub uv: bool,
    #[allow(dead_code)]
    pub relocatable: bool,
    #[allow(dead_code)]
    pub seed: bool,
    #[allow(dead_code)]
    pub include_system_site_packages: bool,
    pub version: Option<String>,
}

impl EnvConfig {
    pub fn parse(cfg: impl AsRef<Path>) -> Result<Self> {
        let mut virtualenv = false;
        let mut uv = false;
        let mut relocatable = false;
        let mut seed = false;
        let mut include_system_site_packages = true;
        let mut version = None;

        let cfg_path = cfg.as_ref();
        if !cfg_path.exists() {
            anyhow::bail!("Configuration file '{}' does not exist", cfg_path.display());
        }
        let content =
            std::fs::read_to_string(cfg_path).context("Failed to read configuration file")?;
        for line in content.lines() {
            let line = line.trim();
            if !line.contains('=') {
                continue; // Skip lines without '=' (e.g., comments or empty lines)
            }
            let (key, value) = line.split_once('=').with_context(|| {
                format!(
                    "Invalid line in configuration file '{}': {}",
                    cfg_path.display(),
                    line
                )
            })?;
            let (key, value) = (key.trim(), value.trim());
            match key {
                "virtualenv" => virtualenv = true,
                "uv" => uv = true,
                "relocatable" => relocatable = value.to_lowercase() == "true",
                "seed" => seed = value.to_lowercase() == "true",
                "include-system-site-packages" => {
                    include_system_site_packages = value.to_lowercase() == "true";
                }
                "version" | "version_info" => {
                    version = Some(value.to_string());
                }
                _ => continue, // Ignore unknown keys
            }
        }

        Ok(EnvConfig {
            virtualenv,
            uv,
            relocatable,
            seed,
            include_system_site_packages,
            version,
        })
    }
}

pub struct VenvBackend {
    uv_path: String,
}

impl VenvBackend {
    pub fn new() -> Result<Self> {
        let uv_path = "uv";
        if !Self::check_uv_available(uv_path) {
            anyhow::bail!(
                "uv is not available, please install it first.\nSee https://docs.astral.sh/uv/getting-started/installation/ for installation instructions"
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

    fn get_site_package_dir(&self, env_name: &str, store: &VenvStore) -> Result<PathBuf> {
        let lib_dir = store.path().join(env_name).join("lib");
        let site_package_dir = lib_dir
            .read_dir()
            .context("Failed to read lib directory")?
            .filter_map(Result::ok)
            .find(|entry| {
                entry.file_type().is_ok_and(|ft| {
                    ft.is_dir()
                        && entry
                            .file_name()
                            .to_str()
                            .is_some_and(|name| name.starts_with("python"))
                })
            })
            .map(|entry| entry.path().join("site-packages"))
            .ok_or_else(|| {
                anyhow::anyhow!("No site-packages directory found in virtual environment")
            })?;
        Ok(site_package_dir)
    }

    // Venv management methods
    pub async fn create(
        &self,
        store: &VenvStore,
        name: &str,
        python: &str,
        clear: bool,
    ) -> Result<()> {
        let _lock = store.lock().await?;
        if store.exists(name) {
            if clear {
                Self::remove_venv(store, name)?;
            } else {
                anyhow::bail!(
                    "Virtual environment '{}' already exists. Use --clear to recreate it",
                    name
                );
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
            anyhow::bail!(
                "Failed to create virtual environment. Check Python version and try again"
            );
        }

        info!(
            "Created virtual environment '{}' at {}",
            name.green(),
            venv_path_str.blue()
        );
        Ok(())
    }

    pub async fn remove(&self, store: &VenvStore, name: &str) -> Result<()> {
        let _lock = store.lock().await?;
        if !store.exists(name) {
            anyhow::bail!("Virtual environment '{}' does not exist", name);
        }
        Self::remove_venv(store, name)?;
        info!("Removed virtual environment '{}'", name.green());
        Ok(())
    }

    fn list_venvs_in_store(
        store: &VenvStore,
        current_venv: Option<&PathBuf>,
    ) -> Result<Vec<EnvInfo>> {
        let entries = store
            .path()
            .read_dir()
            .context("Failed to read venv directory")?
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    if e.path().is_dir() {
                        e.file_name().to_str().map(|name| {
                            let env_path = e.path();
                            let is_active = if let Some(current) = current_venv {
                                // Compare the actual environment paths
                                env_path.canonicalize().ok() == current.canonicalize().ok()
                            } else {
                                false
                            };

                            EnvInfo {
                                name: name.to_string(),
                                path: env_path.clone(),
                                is_active,
                                config: EnvConfig::parse(env_path.join("pyvenv.cfg")).ok(),
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

    pub async fn list(&self) -> Result<Vec<(VenvScope, Vec<EnvInfo>)>> {
        let current_venv = Self::detect_current_venv();
        let scopes = get_candidate_scopes(ScopeType::Unspecified)?;

        let mut results = Vec::new();
        for scope in scopes {
            let venv_store = VenvStore::from_specified_scope(scope.clone())?;
            if !venv_store.is_ready() {
                continue;
            }
            results.push((
                scope.clone(),
                Self::list_venvs_in_store(&venv_store, current_venv.as_ref())?,
            ));
        }
        Ok(results)
    }

    // File management methods
    pub fn dir(&self, store: &VenvStore) -> Result<PathBuf> {
        Ok(store.path().clone())
    }

    // Package management methods
    fn check_env_is_managed(current_venv: &PathBuf) -> Result<VenvScope> {
        let scopes = get_candidate_scopes(ScopeType::Unspecified)?;
        for scope in scopes {
            let store = VenvStore::from_specified_scope(scope.clone())?;
            if store.contains(current_venv)? {
                return Ok(scope);
            }
        }

        anyhow::bail!(
            "Current virtual environment ({}) is not managed by meowda.\nPlease activate a meowda-managed environment first",
            current_venv.display()
        );
    }
    pub async fn install(&self, extra_args: &[&str]) -> Result<()> {
        let current_venv = Self::detect_current_venv()
            .ok_or_else(|| anyhow::anyhow!("No virtual environment is currently activated.\nPlease activate a virtual environment first with: meowda activate <env_name>"))?;
        let scope = Self::check_env_is_managed(&current_venv)?;
        let store = VenvStore::from_specified_scope(scope)?;
        let _lock = store.lock().await?;

        let status = Command::new(&self.uv_path)
            .args(["pip", "install"])
            .args(extra_args)
            .status()
            .context("Failed to execute uv pip install command")?;

        if !status.success() {
            anyhow::bail!("Failed to install packages. Check package names and try again");
        }

        println!("Packages installed successfully.");
        Ok(())
    }
    pub async fn uninstall(&self, extra_args: &[&str]) -> Result<()> {
        let current_venv = Self::detect_current_venv()
            .ok_or_else(|| anyhow::anyhow!("No virtual environment is currently activated.\nPlease activate a virtual environment first with: meowda activate <env_name>"))?;
        let scope = Self::check_env_is_managed(&current_venv)?;
        let store = VenvStore::from_specified_scope(scope)?;
        let _lock = store.lock().await?;

        let status = Command::new(&self.uv_path)
            .args(["pip", "uninstall"])
            .args(extra_args)
            .status()
            .context("Failed to execute uv pip uninstall command")?;

        if !status.success() {
            anyhow::bail!("Failed to uninstall packages. Check package names and try again");
        }

        println!("Packages uninstalled successfully.");
        Ok(())
    }
    pub async fn link(&self, project_name: &str, project_path: &str) -> Result<()> {
        let current_venv = Self::detect_current_venv()
            .ok_or_else(|| anyhow::anyhow!("No virtual environment is currently activated.\nPlease activate a virtual environment first with: meowda activate <env_name>"))?;
        let scope = Self::check_env_is_managed(&current_venv)?;
        let store = VenvStore::from_specified_scope(scope)?;
        let venv_name = current_venv
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid virtual environment name"))?;
        let _lock = store.lock().await?;

        let site_package_dir = self.get_site_package_dir(venv_name, &store)?;
        let pth_file = site_package_dir.join(format!("meowda_link_{}.pth", project_name));
        let abs_path =
            std::path::absolute(project_path).context("Failed to get absolute path for project")?;
        std::fs::write(&pth_file, abs_path.to_string_lossy().as_bytes())
            .context("Failed to create .pth file")?;

        println!("Project linked successfully.");
        Ok(())
    }
    pub async fn unlink(&self, project_name: &str) -> Result<()> {
        let current_venv = Self::detect_current_venv()
            .ok_or_else(|| anyhow::anyhow!("No virtual environment is currently activated.\nPlease activate a virtual environment first with: meowda activate <env_name>"))?;
        let scope = Self::check_env_is_managed(&current_venv)?;
        let store = VenvStore::from_specified_scope(scope)?;
        let venv_name = current_venv
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid virtual environment name"))?;
        let _lock = store.lock().await?;

        let site_package_dir = self.get_site_package_dir(venv_name, &store)?;
        let pth_file = site_package_dir.join(format!("meowda_link_{}.pth", project_name));
        std::fs::remove_file(&pth_file).context("Failed to remove .pth file")?;

        println!("Project unlinked successfully.");
        Ok(())
    }
}
