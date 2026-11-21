use crate::temp_workspace::TempWorkspace;
use crate::utils::utils::write_embed_file;
use anyhow::{Context, Result, anyhow, ensure};
use bit7z::{Archive, Bit7z, CompressionOptions, ExtractOptions, FileRange};
use std::env;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;

/// Application resource/workspace adapter for the independent bit7z crate.
/// Each operation opens its own native reader in the calling worker thread.
#[derive(Debug, Clone)]
pub struct SevenZip {
    engine: Bit7z,
    _workspace: Option<Arc<TempWorkspace>>,
}

impl SevenZip {
    pub fn new() -> Result<Self> {
        let workspace = Arc::new(TempWorkspace::create("sevenzip")?);
        Self::new_in_path(workspace.path(), Some(Arc::clone(&workspace)))
    }

    /// The caller must keep this workspace alive for this instance and its clones.
    pub fn new_in(workspace: &Path) -> Result<Self> {
        Self::new_in_path(workspace, None)
    }

    fn new_in_path(workspace: &Path, owner: Option<Arc<TempWorkspace>>) -> Result<Self> {
        if let Ok(exe) = env::current_exe()
            && let Some(directory) = exe.parent()
        {
            let dll = directory.join("7z.dll");
            if dll.is_file() {
                return Ok(Self {
                    engine: Bit7z::new(&dll)?,
                    _workspace: None,
                });
            }
        }
        fs::create_dir_all(workspace)?;
        let dll = workspace.join("7z.dll");
        if !dll.exists() {
            write_embed_file("7z.dll", &dll)
                .context("Write 7z.dll to temporary workspace failed")?;
        }
        Ok(Self {
            engine: Bit7z::new(&dll).context("Load 7z.dll failed")?,
            _workspace: owner,
        })
    }

    fn open(&self, path: &Path, password: Option<&str>) -> Result<Archive> {
        if let Some(region) = embedded_archive_range(path)? {
            self.engine.open_range(path, region, password)
        } else {
            self.engine.open(path, password)
        }
        .with_context(|| format!("Open archive {}", path.display()))
    }

    /// Creates a 7z archive of directory contents using Ultra, 8 MiB solid blocks,
    /// and the codec's automatic thread count.
    pub fn create_archive(&self, input_path: &Path, out_path: &Path) -> Result<()> {
        self.engine
            .compress_directory(input_path, out_path, CompressionOptions::default())
            .with_context(|| format!("Create archive {}", out_path.display()))
    }

    pub fn extract_files_from_path(
        &self,
        archive_path: &Path,
        password: Option<&str>,
        extract_path: &str,
        out_path: &Path,
    ) -> Result<()> {
        self.extract_files_from_paths(archive_path, password, &[extract_path], out_path)
    }

    pub fn extract_files_from_paths(
        &self,
        archive_path: &Path,
        password: Option<&str>,
        extract_paths: &[&str],
        out_path: &Path,
    ) -> Result<()> {
        self.extract_selected(archive_path, password, extract_paths, out_path, true, true)
    }

    /// Extracts matching files into one directory, omitting archive directories.
    pub fn extract_files(
        &self,
        archive_path: &Path,
        password: Option<&str>,
        extract_path: &str,
        out_path: &Path,
    ) -> Result<()> {
        self.extract_selected(
            archive_path,
            password,
            &[extract_path],
            out_path,
            false,
            false,
        )
    }

    fn extract_selected(
        &self,
        archive_path: &Path,
        password: Option<&str>,
        patterns: &[&str],
        out_path: &Path,
        retain_directories: bool,
        recursive: bool,
    ) -> Result<()> {
        let mut archive = self.open(archive_path, password)?;
        let patterns: Vec<_> = patterns
            .iter()
            .filter(|p| !p.is_empty())
            .map(|p| normalize(p))
            .collect();
        let indices: Vec<_> = archive
            .entries()?
            .into_iter()
            .filter(|entry| {
                (retain_directories || !entry.is_directory)
                    && (patterns.is_empty()
                        || patterns.iter().any(|pattern| {
                            matches_pattern(
                                &normalize(&entry.path.to_string_lossy()),
                                pattern,
                                recursive,
                            )
                        }))
            })
            .map(|entry| entry.index)
            .collect();
        archive
            .extract(
                &indices,
                out_path,
                ExtractOptions {
                    retain_directories,
                    skip_existing: true,
                },
            )
            .with_context(|| format!("Extract archive {}", archive_path.display()))
    }

    pub fn is_driver_package(&self, path: &Path, password: Option<&str>) -> Result<()> {
        let mut archive = self.open(path, password)?;
        ensure!(
            archive
                .entries()?
                .iter()
                .any(|entry| !entry.is_directory && has_extension(&entry.path, "inf")),
            "not a driver package: does not contain an .inf file"
        );
        Ok(())
    }

    /// The first .index entry takes precedence over the first .json entry.
    /// Paths remain structured, including whitespace and non-ASCII characters.
    pub fn extract_index(&self, archive_path: &Path, out_path: &Path) -> Result<()> {
        let mut archive = self.open(archive_path, None)?;
        let entries = archive.entries()?;
        let selected = ["index", "json"].iter().find_map(|extension| {
            entries
                .iter()
                .find(|entry| !entry.is_directory && has_extension(&entry.path, extension))
        });
        if let Some(entry) = selected {
            archive.extract(
                &[entry.index],
                out_path,
                ExtractOptions {
                    retain_directories: false,
                    skip_existing: true,
                },
            )?;
        }
        Ok(())
    }
}

fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
}
fn normalize(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

fn matches_pattern(path: &str, pattern: &str, recursive: bool) -> bool {
    let pattern = pattern.trim_end_matches('/');
    let literal = !pattern.contains(['*', '?']);
    let matches = |candidate: &str| {
        wildcard_match(candidate, pattern)
            || (literal
                && candidate
                    .strip_prefix(pattern)
                    .is_some_and(|suffix| suffix.starts_with('/')))
    };
    if matches(path) {
        return true;
    }
    recursive
        && path
            .match_indices('/')
            .any(|(index, _)| matches(&path[index + 1..]))
}

// 7-Zip-style wildcards rather than glob bracket syntax. A single operation
// unions all patterns before extraction, so solid blocks are not read per filter.
fn wildcard_match(path: &str, pattern: &str) -> bool {
    let path: Vec<_> = path.chars().collect();
    let pattern: Vec<_> = pattern.chars().collect();
    let (mut input, mut token, mut star, mut retry) = (0, 0, None, 0);
    while input < path.len() {
        if token < pattern.len() && (pattern[token] == '?' || pattern[token] == path[input]) {
            input += 1;
            token += 1;
        } else if token < pattern.len() && pattern[token] == '*' {
            star = Some(token);
            token += 1;
            retry = input;
        } else if let Some(index) = star {
            retry += 1;
            input = retry;
            token = index + 1;
        } else {
            return false;
        }
    }
    while token < pattern.len() && pattern[token] == '*' {
        token += 1;
    }
    token == pattern.len()
}

/// Read the existing fixed-width footer without depending on command modules
/// (this module is also exposed by the library target).
fn embedded_archive_range(path: &Path) -> Result<Option<FileRange>> {
    const FOOTER_SIZE: u64 = 88;
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    if size < FOOTER_SIZE {
        return Ok(None);
    }
    file.seek(SeekFrom::End(-(FOOTER_SIZE as i64)))?;
    let mut footer = [0; FOOTER_SIZE as usize];
    file.read_exact(&mut footer)?;
    if &footer[80..88] != b"DRV_PKG!" {
        return Ok(None);
    }
    let offset = u64::from_le_bytes(footer[0..8].try_into().unwrap());
    let length = u64::from_le_bytes(footer[8..16].try_into().unwrap());
    if length == 0 || offset.checked_add(length) != Some(size - FOOTER_SIZE) {
        return Err(anyhow!(
            "Invalid embedded archive range in {}",
            path.display()
        ));
    }
    Ok(Some(FileRange { offset, length }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_matches_recursive_filters_and_directory_boundaries() {
        assert!(matches_pattern(
            &normalize("A\\中文 空格\\FILE.INF"),
            "*.inf",
            true
        ));
        assert!(matches_pattern("outer/driver/a.sys", "driver", true));
        assert!(matches_pattern("driver/sub/a.sys", "driver", true));
        assert!(!matches_pattern("drivers/a.sys", "driver", true));
        assert!(!matches_pattern("a.inf.backup", "*.inf", true));
        assert!(matches_pattern("[literal].inf", "[literal].inf", true));
        assert!(matches_pattern("a.inf", "?.inf", true));
        assert!(!matches_pattern("aa.inf", "?.inf", true));
        assert!(!matches_pattern("outer/file.inf", "file.inf", false));
    }
}
