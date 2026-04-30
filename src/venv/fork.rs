use super::{EnvConfig, VenvService, create::create_uv_venv};
use crate::store::venv_store::{ScopeType, VenvStore, get_candidate_scopes};
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const PYTHON_INFO_SEPARATOR: char = '\u{1f}';
const PYTHON_INFO_SCRIPT: &str = r#"import sys
import sysconfig

separator = "\x1f"

def emit(key, value):
    if value is None:
        value = ""
    print(f"{key}{separator}{value}")

emit("executable", sys.executable)
emit("base_executable", getattr(sys, "_base_executable", sys.executable))
emit("prefix", sys.prefix)
emit("base_prefix", getattr(sys, "base_prefix", sys.prefix))
emit("real_prefix", getattr(sys, "real_prefix", ""))
emit("scripts", sysconfig.get_path("scripts") or "")
emit("purelib", sysconfig.get_path("purelib") or "")
emit("platlib", sysconfig.get_path("platlib") or "")
"#;

#[derive(Debug, Clone)]
pub(super) struct PythonEnvLayout {
    python: PathBuf,
    base_python: PathBuf,
    prefix: PathBuf,
    purelib: PathBuf,
    platlib: PathBuf,
    scripts_dir: PathBuf,
    include_system_site_packages: bool,
    requested_prefix: Option<PathBuf>,
}

impl PythonEnvLayout {
    pub(super) fn prefix(&self) -> &Path {
        &self.prefix
    }
}

#[derive(Debug, Clone)]
struct RewritePaths {
    replacements: Vec<(String, String)>,
}

enum ForkSource {
    Directory(PathBuf),
    Python(String),
}

pub(super) fn resolve_current_source() -> Result<PythonEnvLayout> {
    inspect_resolved_source(resolve_current_source_spec()?)
}

pub(super) fn resolve_named_source(source: &str, scope_type: ScopeType) -> Result<PythonEnvLayout> {
    inspect_resolved_source(resolve_source_spec(source, scope_type)?).with_context(|| {
        format!(
            "Failed to resolve fork source '{source}' in the selected scope or as a Python executable"
        )
    })
}

pub(super) fn create_with_source(
    uv_path: &str,
    source: &PythonEnvLayout,
    target_path: &Path,
) -> Result<()> {
    let target_path = normalize_path(target_path)?;
    let source_prefix = normalize_path(&source.prefix)?;
    if source_prefix == target_path {
        anyhow::bail!("Fork source and target cannot be the same environment");
    }

    create_uv_venv(
        uv_path,
        &target_path,
        source.base_python.to_string_lossy().as_ref(),
        false,
        source.include_system_site_packages,
    )?;

    let target_python = python_path_in_venv(&target_path);
    let mut target = inspect_python_env(target_python.to_string_lossy().as_ref())?;
    target.requested_prefix = Some(target_path.clone());
    let rewrite = build_rewrite_paths(source, &target);
    copy_site_packages(source, &target, &rewrite)?;
    copy_scripts(source, &target, &rewrite)?;
    Ok(())
}

fn normalize_path(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = path.as_ref();
    if path.exists() {
        path.canonicalize()
            .with_context(|| format!("Failed to canonicalize path '{}'", path.display()))
    } else {
        std::path::absolute(path)
            .with_context(|| format!("Failed to resolve path '{}'", path.display()))
    }
}

fn python_path_in_venv(venv_path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        return venv_path.join("Scripts").join("python.exe");
    }

    #[cfg(not(windows))]
    {
        venv_path.join("bin").join("python")
    }
}

fn is_text_file(path: &Path) -> Result<bool> {
    let bytes =
        fs::read(path).with_context(|| format!("Failed to read file '{}'", path.display()))?;
    if bytes.contains(&0) {
        return Ok(false);
    }
    Ok(std::str::from_utf8(&bytes).is_ok())
}

fn is_package_metadata(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext, "pth" | "egg-link"))
}

fn apply_rewrite(path: &Path, rewrite: &RewritePaths) -> Result<()> {
    if !is_text_file(path)? {
        return Ok(());
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file '{}' as text", path.display()))?;
    let mut updated = content.clone();
    for (from, to) in &rewrite.replacements {
        updated = updated.replace(from, to);
    }
    if updated != content {
        fs::write(path, updated)
            .with_context(|| format!("Failed to rewrite file '{}'", path.display()))?;
    }
    Ok(())
}

fn copy_permissions(source: &Path, target: &Path) -> Result<()> {
    let permissions = fs::metadata(source)
        .with_context(|| format!("Failed to read metadata for '{}'", source.display()))?
        .permissions();
    fs::set_permissions(target, permissions)
        .with_context(|| format!("Failed to update permissions for '{}'", target.display()))
}

#[cfg(unix)]
fn create_symlink(target: &Path, link_path: &Path, _is_dir: bool) -> Result<()> {
    std::os::unix::fs::symlink(target, link_path).with_context(|| {
        format!(
            "Failed to create symlink '{}' -> '{}'",
            link_path.display(),
            target.display()
        )
    })
}

#[cfg(windows)]
fn create_symlink(target: &Path, link_path: &Path, is_dir: bool) -> Result<()> {
    if is_dir {
        std::os::windows::fs::symlink_dir(target, link_path)
    } else {
        std::os::windows::fs::symlink_file(target, link_path)
    }
    .with_context(|| {
        format!(
            "Failed to create symlink '{}' -> '{}'",
            link_path.display(),
            target.display()
        )
    })
}

fn rewrite_symlink_target(target: &Path, rewrite: &RewritePaths) -> PathBuf {
    let target_string = target.to_string_lossy().into_owned();
    for (from, to) in &rewrite.replacements {
        if let Some(relative) = target_string.strip_prefix(from) {
            return PathBuf::from(format!("{to}{relative}"));
        }
    }

    target.to_path_buf()
}

fn add_replacement_pair(
    replacements: &mut Vec<(String, String)>,
    from: impl AsRef<Path>,
    to: impl AsRef<Path>,
) {
    let from = from.as_ref().to_string_lossy().into_owned();
    let to = to.as_ref().to_string_lossy().into_owned();
    if from.is_empty() || to.is_empty() || from == to {
        return;
    }
    if replacements
        .iter()
        .any(|(existing_from, existing_to)| existing_from == &from && existing_to == &to)
    {
        return;
    }
    replacements.push((from, to));
}

fn build_rewrite_paths(source: &PythonEnvLayout, target: &PythonEnvLayout) -> RewritePaths {
    let mut replacements = Vec::new();
    add_replacement_pair(&mut replacements, &source.python, &target.python);
    add_replacement_pair(&mut replacements, &source.prefix, &target.prefix);

    if let (Some(source_prefix), Some(target_prefix)) = (
        source.requested_prefix.as_ref(),
        target.requested_prefix.as_ref(),
    ) {
        add_replacement_pair(&mut replacements, source_prefix, target_prefix);
    }

    RewritePaths { replacements }
}

fn copy_file(
    source: &Path,
    target: &Path,
    rewrite: &RewritePaths,
    rewrite_text_files: bool,
) -> Result<()> {
    fs::copy(source, target).with_context(|| {
        format!(
            "Failed to copy file '{}' to '{}'",
            source.display(),
            target.display()
        )
    })?;
    copy_permissions(source, target)?;
    if rewrite_text_files || is_package_metadata(source) {
        apply_rewrite(target, rewrite)?;
    }
    Ok(())
}

fn copy_path(
    source: &Path,
    target: &Path,
    rewrite: &RewritePaths,
    rewrite_text_files: bool,
) -> Result<()> {
    let metadata = fs::symlink_metadata(source)
        .with_context(|| format!("Failed to inspect '{}'", source.display()))?;
    if metadata.file_type().is_symlink() {
        let link_target = fs::read_link(source)
            .with_context(|| format!("Failed to read symlink '{}'", source.display()))?;
        let rewritten_target = rewrite_symlink_target(&link_target, rewrite);
        let link_is_dir = fs::metadata(source)
            .map(|meta| meta.is_dir())
            .unwrap_or(false);
        create_symlink(&rewritten_target, target, link_is_dir)?;
        return Ok(());
    }

    if metadata.is_dir() {
        fs::create_dir_all(target)
            .with_context(|| format!("Failed to create directory '{}'", target.display()))?;
        copy_permissions(source, target)?;
        for entry in fs::read_dir(source)
            .with_context(|| format!("Failed to read directory '{}'", source.display()))?
        {
            let entry = entry
                .with_context(|| format!("Failed to read entry inside '{}'", source.display()))?;
            copy_path(
                &entry.path(),
                &target.join(entry.file_name()),
                rewrite,
                rewrite_text_files,
            )?;
        }
        return Ok(());
    }

    copy_file(source, target, rewrite, rewrite_text_files)
}

fn copy_directory_contents(
    source_dir: &Path,
    target_dir: &Path,
    rewrite: &RewritePaths,
    rewrite_text_files: bool,
) -> Result<()> {
    if !source_dir.exists() {
        return Ok(());
    }
    fs::create_dir_all(target_dir)
        .with_context(|| format!("Failed to create '{}'", target_dir.display()))?;
    for entry in fs::read_dir(source_dir)
        .with_context(|| format!("Failed to read directory '{}'", source_dir.display()))?
    {
        let entry = entry
            .with_context(|| format!("Failed to read entry inside '{}'", source_dir.display()))?;
        copy_path(
            &entry.path(),
            &target_dir.join(entry.file_name()),
            rewrite,
            rewrite_text_files,
        )?;
    }
    Ok(())
}

fn is_core_script(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    lowered.starts_with("activate") || lowered.starts_with("python")
}

fn parse_python_layout(output: &str) -> Result<PythonEnvLayout> {
    let values = output
        .lines()
        .filter_map(|line| line.split_once(PYTHON_INFO_SEPARATOR))
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect::<HashMap<_, _>>();

    let required = |key: &str| -> Result<PathBuf> {
        let value = values
            .get(key)
            .ok_or_else(|| anyhow::anyhow!("Missing '{key}' in Python environment inspection"))?;
        if value.is_empty() {
            anyhow::bail!("Python environment inspection returned an empty '{key}'");
        }
        Ok(PathBuf::from(value))
    };

    let prefix = required("prefix")?;
    let base_prefix = required("base_prefix")?;
    let real_prefix = values.get("real_prefix").cloned().unwrap_or_default();
    let include_system_site_packages = EnvConfig::parse(prefix.join("pyvenv.cfg"))
        .map(|config| config.include_system_site_packages)
        .unwrap_or(false);

    Ok(PythonEnvLayout {
        python: required("executable")?,
        base_python: required("base_executable")?,
        prefix: prefix.clone(),
        purelib: required("purelib")?,
        platlib: required("platlib")?,
        scripts_dir: required("scripts")?,
        include_system_site_packages: prefix.join("pyvenv.cfg").exists()
            && (prefix != base_prefix || !real_prefix.is_empty() || include_system_site_packages),
        requested_prefix: None,
    })
}

fn inspect_python_env(python: &str) -> Result<PythonEnvLayout> {
    let output = Command::new(python)
        .args(["-c", PYTHON_INFO_SCRIPT])
        .output()
        .with_context(|| format!("Failed to inspect Python environment with '{python}'"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Failed to inspect Python environment with '{}': {}",
            python,
            stderr.trim()
        );
    }

    parse_python_layout(String::from_utf8_lossy(&output.stdout).as_ref())
}

fn find_python_in_directory(dir: &Path) -> Result<Option<PathBuf>> {
    #[cfg(windows)]
    {
        let candidates = [
            dir.join("Scripts").join("python.exe"),
            dir.join("python.exe"),
        ];
        for candidate in candidates {
            if candidate.exists() {
                return Ok(Some(candidate));
            }
        }
        Ok(None)
    }

    #[cfg(not(windows))]
    {
        let bin_dir = dir.join("bin");
        if !bin_dir.is_dir() {
            return Ok(None);
        }

        let preferred = ["python", "python3"];
        for name in preferred {
            let candidate = bin_dir.join(name);
            if candidate.exists() {
                return Ok(Some(candidate));
            }
        }

        let mut fallback = None;
        for entry in fs::read_dir(&bin_dir)
            .with_context(|| format!("Failed to read '{}'", bin_dir.display()))?
        {
            let entry = entry
                .with_context(|| format!("Failed to read entry inside '{}'", bin_dir.display()))?;
            if entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with("python"))
            {
                fallback = Some(entry.path());
                break;
            }
        }
        Ok(fallback)
    }
}

fn resolve_managed_env(name: &str, scope_type: ScopeType) -> Result<Option<PathBuf>> {
    for scope in get_candidate_scopes(scope_type)? {
        let store = VenvStore::from_specified_scope(scope)?;
        if store.is_ready() && store.exists(name) {
            return Ok(Some(store.path().join(name)));
        }
    }
    Ok(None)
}

fn env_root_from_python_path(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    let dirname = parent.file_name()?.to_str()?;
    if matches!(dirname, "bin" | "Scripts") {
        return parent.parent().map(Path::to_path_buf);
    }
    None
}

fn resolve_current_source_spec() -> Result<ForkSource> {
    if let Some(current_venv) = VenvService::detect_current_venv() {
        return Ok(ForkSource::Directory(current_venv));
    }

    for candidate in ["python", "python3"] {
        if Command::new(candidate)
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
        {
            return Ok(ForkSource::Python(candidate.to_string()));
        }
    }

    anyhow::bail!(
        "Unable to resolve the current Python environment. Activate one first or pass an explicit path"
    )
}

fn resolve_source_spec(source: &str, scope_type: ScopeType) -> Result<ForkSource> {
    let source_path = PathBuf::from(source);
    if source_path.exists() {
        if source_path.is_dir() {
            let absolute = normalize_path(&source_path)?;
            return Ok(ForkSource::Directory(absolute));
        }

        let absolute = std::path::absolute(&source_path).with_context(|| {
            format!(
                "Failed to resolve Python executable path '{}'",
                source_path.display()
            )
        })?;
        return Ok(ForkSource::Python(absolute.to_string_lossy().into_owned()));
    }

    if let Some(managed_env) = resolve_managed_env(source, scope_type)? {
        return Ok(ForkSource::Directory(normalize_path(managed_env)?));
    }

    Ok(ForkSource::Python(source.to_string()))
}

fn inspect_resolved_source(source: ForkSource) -> Result<PythonEnvLayout> {
    match source {
        ForkSource::Directory(path) => {
            if path.join("pyvenv.cfg").exists() {
                let python = find_python_in_directory(&path)?.ok_or_else(|| {
                    anyhow::anyhow!(
                        "Virtual environment '{}' does not contain a usable Python executable",
                        path.display()
                    )
                })?;
                let mut layout = inspect_python_env(&python.to_string_lossy())?;
                layout.requested_prefix = Some(path);
                return Ok(layout);
            }

            if let Some(python) = find_python_in_directory(&path)? {
                let mut layout = inspect_python_env(&python.to_string_lossy())?;
                layout.requested_prefix = Some(path);
                return Ok(layout);
            }

            anyhow::bail!(
                "Fork source '{}' is not a Python environment or Python executable",
                path.display()
            )
        }
        ForkSource::Python(python) => {
            let mut layout = inspect_python_env(&python)?;
            layout.requested_prefix = env_root_from_python_path(Path::new(&python));
            Ok(layout)
        }
    }
}

fn copy_site_packages(
    source: &PythonEnvLayout,
    target: &PythonEnvLayout,
    rewrite: &RewritePaths,
) -> Result<()> {
    let mut copied_pairs = HashSet::new();
    for (source_dir, target_dir) in [
        (&source.purelib, &target.purelib),
        (&source.platlib, &target.platlib),
    ] {
        let key = format!(
            "{}=>{}",
            source_dir.to_string_lossy(),
            target_dir.to_string_lossy()
        );
        if !copied_pairs.insert(key) {
            continue;
        }

        copy_directory_contents(source_dir, target_dir, rewrite, false)?;
    }

    Ok(())
}

fn copy_scripts(
    source: &PythonEnvLayout,
    target: &PythonEnvLayout,
    rewrite: &RewritePaths,
) -> Result<()> {
    if !source.scripts_dir.exists() {
        return Ok(());
    }

    fs::create_dir_all(&target.scripts_dir)
        .with_context(|| format!("Failed to create '{}'", target.scripts_dir.display()))?;
    for entry in fs::read_dir(&source.scripts_dir)
        .with_context(|| format!("Failed to read '{}'", source.scripts_dir.display()))?
    {
        let entry = entry.with_context(|| {
            format!(
                "Failed to read entry inside '{}'",
                source.scripts_dir.display()
            )
        })?;
        let name = entry.file_name();
        if name.to_str().is_some_and(is_core_script) {
            continue;
        }

        copy_path(
            &entry.path(),
            &target.scripts_dir.join(&name),
            rewrite,
            true,
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn package_metadata_rewrite_updates_source_paths() -> Result<()> {
        let temp = tempdir()?;
        let source_root = temp.path().join("source");
        let target_root = temp.path().join("target");
        let source_file = source_root.join("lib/python3.12/site-packages/demo.pth");
        let target_file = target_root.join("lib/python3.12/site-packages/demo.pth");
        fs::create_dir_all(source_file.parent().expect("parent exists"))?;
        fs::create_dir_all(target_file.parent().expect("parent exists"))?;
        fs::write(
            &source_file,
            format!(
                "{}\n{}\n",
                source_root.display(),
                source_root.join("bin/python").display()
            ),
        )?;

        let rewrite = RewritePaths {
            replacements: vec![
                (
                    source_root
                        .join("bin/python")
                        .to_string_lossy()
                        .into_owned(),
                    target_root
                        .join("bin/python")
                        .to_string_lossy()
                        .into_owned(),
                ),
                (
                    source_root.to_string_lossy().into_owned(),
                    target_root.to_string_lossy().into_owned(),
                ),
            ],
        };

        copy_file(&source_file, &target_file, &rewrite, false)?;

        let rewritten = fs::read_to_string(target_file)?;
        assert!(rewritten.contains(target_root.to_string_lossy().as_ref()));
        assert!(!rewritten.contains(source_root.to_string_lossy().as_ref()));
        Ok(())
    }

    #[test]
    fn script_copy_skips_python_and_activate_entries() {
        assert!(is_core_script("python"));
        assert!(is_core_script("python3.12"));
        assert!(is_core_script("activate.fish"));
        assert!(!is_core_script("ruff"));
    }

    #[test]
    fn parse_python_layout_reads_required_fields() -> Result<()> {
        let output = [
            "executable\u{1f}/tmp/source/bin/python",
            "base_executable\u{1f}/usr/bin/python3.12",
            "prefix\u{1f}/tmp/source",
            "base_prefix\u{1f}/usr",
            "real_prefix\u{1f}",
            "scripts\u{1f}/tmp/source/bin",
            "purelib\u{1f}/tmp/source/lib/python3.12/site-packages",
            "platlib\u{1f}/tmp/source/lib/python3.12/site-packages",
        ]
        .join("\n");

        let layout = parse_python_layout(&output)?;
        assert_eq!(layout.prefix, PathBuf::from("/tmp/source"));
        assert_eq!(layout.base_python, PathBuf::from("/usr/bin/python3.12"));
        assert_eq!(layout.scripts_dir, PathBuf::from("/tmp/source/bin"));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn resolve_source_spec_preserves_venv_python_symlink_path() -> Result<()> {
        let temp = tempdir()?;
        let source_root = temp.path().join("source-env");
        let bin_dir = source_root.join("bin");
        fs::create_dir_all(&bin_dir)?;

        let base_python = temp.path().join("python3.14");
        fs::write(&base_python, "#!/bin/sh\n")?;
        let venv_python = bin_dir.join("python");
        std::os::unix::fs::symlink(&base_python, &venv_python)?;

        let resolved = resolve_source_spec(
            venv_python.to_string_lossy().as_ref(),
            ScopeType::Unspecified,
        )?;
        match resolved {
            ForkSource::Python(path) => {
                assert_eq!(PathBuf::from(path), std::path::absolute(&venv_python)?);
            }
            ForkSource::Directory(_) => {
                anyhow::bail!("expected a Python executable source");
            }
        }

        Ok(())
    }

    #[test]
    fn resolve_source_spec_treats_current_as_regular_source_name() -> Result<()> {
        let resolved = resolve_source_spec("current", ScopeType::Unspecified)?;
        match resolved {
            ForkSource::Python(path) => {
                assert_eq!(path, "current");
            }
            ForkSource::Directory(path) => {
                anyhow::bail!(
                    "expected 'current' to remain an explicit source name, got '{}'",
                    path.display()
                );
            }
        }

        Ok(())
    }
}
