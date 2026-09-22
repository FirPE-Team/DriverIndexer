use anyhow::{Context, Result, anyhow};
use std::env::temp_dir;
use std::fs;
use std::path::{Path, PathBuf};

/// Owns a unique per-command temporary directory.
///
/// The directory is removed when the workspace is dropped. Cleanup errors are
/// intentionally ignored here because a cleanup failure must not replace the
/// command's actual result; callers can use `cleanup` when they need to report
/// the error explicitly.
#[derive(Debug)]
pub struct TempWorkspace {
    path: PathBuf,
}

impl TempWorkspace {
    pub fn create(prefix: &str) -> Result<Self> {
        let root = temp_dir();
        for attempt in 0..32u32 {
            let path = root.join(format!(
                "DriverIndexer-{prefix}-{}-{attempt}",
                fastrand::u32(..)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("create temporary workspace {}", path.display()));
                }
            }
        }
        Err(anyhow!("unable to allocate a unique temporary workspace"))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn cleanup(mut self) -> std::io::Result<()> {
        let path = std::mem::take(&mut self.path);
        if path.exists() {
            fs::remove_dir_all(path)
        } else {
            Ok(())
        }
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        if !self.path.as_os_str().is_empty() {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_unique_workspace_and_cleans_it() {
        let workspace = TempWorkspace::create("test").unwrap();
        let path = workspace.path().to_path_buf();
        assert!(path.is_dir());
        workspace.cleanup().unwrap();
        assert!(!path.exists());
    }
}
