use crate::store::venv_store::VenvStore;
use anstream::panic;
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
    pub fn new() -> Self {
        let uv_path = "uv";
        if !Self::check_uv_available(uv_path) {
            panic!(
                "uv is not available, please install it first. See https://docs.astral.sh/uv/getting-started/installation/"
            );
        }

        VenvBackend {
            uv_path: uv_path.to_string(),
        }
    }

    fn check_uv_available(uv_path: &str) -> bool {
        // check if uv is available by commanding `uv --version`
        Command::new(uv_path)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn get_venv_store() -> VenvStore {
        let store = VenvStore::new();
        if !store.is_ready() {
            store.init().expect("Failed to initialize venv store");
        }
        store
    }

    fn remove_venv(store: &VenvStore, name: &str) {
        std::fs::remove_dir_all(store.path().join(name))
            .expect("Failed to remove virtual environment");
    }

    fn detect_current_venv() -> Option<PathBuf> {
        std::env::var("VIRTUAL_ENV")
            .ok()
            .and_then(|s| std::path::absolute(PathBuf::from(s)).ok())
    }

    fn contains(&self, path: impl AsRef<Path>) -> bool {
        let store = Self::get_venv_store();
        path.as_ref().starts_with(store.path())
    }

    // Venv management methods
    pub async fn create(&self, name: &str, python: &str, clear: bool) {
        let store = Self::get_venv_store();
        let _lock = store.lock().await;
        if store.exists(name) {
            if clear {
                Self::remove_venv(&store, name);
            } else {
                panic!("Virtual environment '{name}' already exists");
            }
        }
        let venv_path = store.path().join(name);
        Command::new(&self.uv_path)
            .args([
                "venv",
                store.path().join(name).to_str().unwrap(),
                "--python",
                python,
                "--seed",
            ])
            .status()
            .expect("Failed to create virtual environment");
        info!(
            "Created virtual environment '{}' at {}",
            name.green(),
            venv_path.to_str().unwrap().blue()
        );
    }
    pub async fn remove(&self, name: &str) {
        let store = Self::get_venv_store();
        let _lock = store.lock().await;
        if !store.exists(name) {
            panic!("Virtual environment '{name}' does not exist");
        }
        Self::remove_venv(&store, name);
        info!("Removed virtual environment '{}'", name.green());
    }
    pub async fn list(&self) -> Vec<EnvInfo> {
        let store = Self::get_venv_store();
        let _lock = store.lock().await;
        let current_venv = Self::detect_current_venv();
        
        store
            .path()
            .read_dir()
            .expect("Failed to read venv directory")
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    if e.path().is_dir() {
                        e.file_name().to_str().map(|name| {
                            let env_path = e.path();
                            let is_active = if let Some(ref current) = current_venv {
                                // Compare the actual environment paths using a robust method
                                Self::paths_are_equivalent(&env_path, current)
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
            .collect()
    }

    // Package management methods
    pub async fn install(&self, extra_args: &[&str]) {
        let store = Self::get_venv_store();
        let _lock = store.lock().await;
        if !store.path().exists() {
            panic!("Virtual environment does not exist");
        }
        let current_venv = Self::detect_current_venv();
        if current_venv.is_none() {
            panic!("No virtual environment is currently activated");
        }
        let current_venv = current_venv.unwrap();
        if !self.contains(current_venv.clone()) {
            panic!(
                "Current virtual environment ({}) is not managed by this backend ({})",
                current_venv.display(),
                store.path().display()
            );
        }
        Command::new(&self.uv_path)
            .args(["pip", "install"])
            .args(extra_args)
            .status()
            .expect("Failed to install packages");
        info!("Packages installed successfully.");
    }
    pub async fn uninstall(&self, extra_args: &[&str]) {
        let store = Self::get_venv_store();
        let _lock = store.lock().await;
        if !store.path().exists() {
            panic!("Virtual environment does not exist");
        }
        let current_venv = Self::detect_current_venv();
        if current_venv.is_none() {
            panic!("No virtual environment is currently activated");
        }
        let current_venv = current_venv.unwrap();
        if !self.contains(current_venv.clone()) {
            panic!(
                "Current virtual environment ({}) is not managed by this backend ({})",
                current_venv.display(),
                store.path().display()
            );
        }
        Command::new(&self.uv_path)
            .args(["pip", "uninstall"])
            .args(extra_args)
            .status()
            .expect("Failed to uninstall packages");
        info!("Packages uninstalled successfully.");
    }

    // File management methods
    pub fn dir(&self) -> PathBuf {
        Self::get_venv_store().path().clone()
    }
}
