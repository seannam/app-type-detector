#![allow(missing_docs)]

//! File-system snapshot abstraction.
//!
//! The detector only sees a project through an [`InputSnapshot`]: a cheap
//! interface that answers three questions — does a file exist, what globs match,
//! and what are the contents of a known file. This keeps the engine pure and
//! lets consumers pass in-memory maps (`MemorySnapshot`) or real directories
//! (`FilesystemSnapshot`, gated behind the `fs` feature) interchangeably.

use std::collections::HashMap;

use globset::{Glob, GlobSet, GlobSetBuilder};

pub const MAX_FILE_BYTES: u64 = 64 * 1024;
pub const IGNORED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "dist",
    ".next",
    "target",
    "build",
    "Library",
    "Temp",
    "obj",
    "bin",
    ".venv",
    ".gradle",
    "Pods",
    "DerivedData",
];

#[derive(Debug, Clone, Default)]
pub struct FileEntry {
    pub path: String,
    pub bytes: u64,
}

pub trait InputSnapshot {
    fn file_exists(&self, path: &str) -> bool;
    fn file_contents(&self, path: &str) -> Option<String>;
    fn glob_count(&self, pattern: &str) -> u64;
    fn glob_list(&self, pattern: &str) -> Vec<String>;
    fn all_files(&self) -> Vec<FileEntry>;
    fn ignored_paths(&self) -> Vec<String> {
        IGNORED_DIRS.iter().map(|s| s.to_string()).collect()
    }
}

/// In-memory snapshot. Paths are normalized to use forward slashes.
#[derive(Debug, Clone, Default)]
pub struct MemorySnapshot {
    files: HashMap<String, Option<String>>,
    ordered_paths: Vec<String>,
}

impl MemorySnapshot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_file(mut self, path: impl Into<String>, contents: impl Into<String>) -> Self {
        let p = normalize(path.into());
        let c = contents.into();
        self.files.insert(p.clone(), Some(c));
        if !self.ordered_paths.contains(&p) {
            self.ordered_paths.push(p);
        }
        self
    }

    pub fn with_empty(mut self, path: impl Into<String>) -> Self {
        let p = normalize(path.into());
        self.files.insert(p.clone(), None);
        if !self.ordered_paths.contains(&p) {
            self.ordered_paths.push(p);
        }
        self
    }

    pub fn from_map(files: HashMap<String, Option<String>>) -> Self {
        let mut ordered = files.keys().cloned().collect::<Vec<_>>();
        ordered.sort();
        Self {
            files,
            ordered_paths: ordered,
        }
    }
}

fn normalize(path: String) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_string()
}

fn build_globset(pattern: &str) -> Option<GlobSet> {
    let glob = Glob::new(pattern).ok()?;
    let mut builder = GlobSetBuilder::new();
    builder.add(glob);
    builder.build().ok()
}

impl InputSnapshot for MemorySnapshot {
    fn file_exists(&self, path: &str) -> bool {
        let k = normalize(path.to_string());
        self.files.contains_key(&k)
    }

    fn file_contents(&self, path: &str) -> Option<String> {
        let k = normalize(path.to_string());
        self.files.get(&k).cloned().flatten()
    }

    fn glob_count(&self, pattern: &str) -> u64 {
        self.glob_list(pattern).len() as u64
    }

    fn glob_list(&self, pattern: &str) -> Vec<String> {
        let Some(set) = build_globset(pattern) else {
            return Vec::new();
        };
        self.ordered_paths
            .iter()
            .filter(|p| set.is_match(p.as_str()))
            .cloned()
            .collect()
    }

    fn all_files(&self) -> Vec<FileEntry> {
        self.ordered_paths
            .iter()
            .map(|p| FileEntry {
                path: p.clone(),
                bytes: self
                    .files
                    .get(p)
                    .and_then(|c| c.as_ref().map(|s| s.len() as u64))
                    .unwrap_or(0),
            })
            .collect()
    }
}

#[cfg(feature = "fs")]
pub use fs_impl::FilesystemSnapshot;

#[cfg(feature = "fs")]
mod fs_impl {
    use super::*;
    use std::path::{Path, PathBuf};
    use walkdir::WalkDir;

    /// Real-filesystem snapshot. Walks a directory once (depth-capped) and
    /// caches paths + sizes. Contents are read lazily per file, and are
    /// truncated to [`MAX_FILE_BYTES`] before any regex is run.
    pub struct FilesystemSnapshot {
        root: PathBuf,
        relative_paths: Vec<String>,
        sizes: std::collections::HashMap<String, u64>,
    }

    impl FilesystemSnapshot {
        pub fn new(root: impl AsRef<Path>) -> std::io::Result<Self> {
            let root = root.as_ref().to_path_buf();
            if !root.exists() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("path {} does not exist", root.display()),
                ));
            }
            let mut relative_paths: Vec<String> = Vec::new();
            let mut sizes = std::collections::HashMap::new();
            let walker = WalkDir::new(&root)
                .follow_links(false)
                .max_depth(8)
                .into_iter();
            for entry in walker.filter_entry(|e| !is_ignored(e)) {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                if !entry.file_type().is_file() {
                    continue;
                }
                let rel = match entry.path().strip_prefix(&root) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                let rel_str = rel
                    .to_string_lossy()
                    .replace('\\', "/")
                    .trim_start_matches("./")
                    .to_string();
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                sizes.insert(rel_str.clone(), size);
                relative_paths.push(rel_str);
            }
            relative_paths.sort();
            Ok(Self {
                root,
                relative_paths,
                sizes,
            })
        }
    }

    fn is_ignored(entry: &walkdir::DirEntry) -> bool {
        if entry.file_type().is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            super::IGNORED_DIRS.contains(&name.as_str())
        } else {
            false
        }
    }

    impl InputSnapshot for FilesystemSnapshot {
        fn file_exists(&self, path: &str) -> bool {
            self.sizes.contains_key(&normalize(path.to_string()))
        }

        fn file_contents(&self, path: &str) -> Option<String> {
            let rel = normalize(path.to_string());
            if !self.sizes.contains_key(&rel) {
                return None;
            }
            let abs = self.root.join(&rel);
            let bytes = std::fs::read(&abs).ok()?;
            let truncated = if bytes.len() as u64 > MAX_FILE_BYTES {
                &bytes[..MAX_FILE_BYTES as usize]
            } else {
                &bytes[..]
            };
            String::from_utf8(truncated.to_vec()).ok()
        }

        fn glob_count(&self, pattern: &str) -> u64 {
            self.glob_list(pattern).len() as u64
        }

        fn glob_list(&self, pattern: &str) -> Vec<String> {
            let Some(set) = build_globset(pattern) else {
                return Vec::new();
            };
            self.relative_paths
                .iter()
                .filter(|p| set.is_match(p.as_str()))
                .cloned()
                .collect()
        }

        fn all_files(&self) -> Vec<FileEntry> {
            self.relative_paths
                .iter()
                .map(|p| FileEntry {
                    path: p.clone(),
                    bytes: *self.sizes.get(p).unwrap_or(&0),
                })
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_snapshot_glob_basic() {
        let s = MemorySnapshot::new()
            .with_file("Cargo.toml", "[package]")
            .with_file("src/lib.rs", "")
            .with_file("src/main.rs", "");
        assert!(s.file_exists("Cargo.toml"));
        assert_eq!(s.glob_count("src/**/*.rs"), 2);
    }

    #[test]
    fn memory_snapshot_empty_vs_missing() {
        let s = MemorySnapshot::new().with_empty("empty.txt");
        assert!(s.file_exists("empty.txt"));
        assert!(s.file_contents("empty.txt").is_none());
        assert!(!s.file_exists("nope.txt"));
    }
}
