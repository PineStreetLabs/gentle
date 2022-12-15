#![allow(unused_variables, unused_mut, dead_code)]

use glob::{MatchOptions, Pattern, PatternError};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct FileSelector {
    files: HashSet<PathBuf>,
    globs: Vec<Pattern>,
}

impl FileSelector {
    pub fn builder() -> FileSelectorBuilder {
        Default::default()
    }

    pub fn list(&self) -> anyhow::Result<HashSet<PathBuf>> {
        let mut result = HashSet::new();

        for entry in walkdir::WalkDir::new("./") {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            if !self.includes(path) {
                continue;
            }

            result.insert(path.to_path_buf());
        }

        if result.is_empty() {
            anyhow::bail!("Found no files");
        }

        Ok(result)
    }

    fn includes(&self, path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        let path = path.strip_prefix("./").unwrap_or(path).to_path_buf();
        if self.files.contains(&path) {
            return true;
        }

        let mut match_options = MatchOptions::default();
        match_options.require_literal_separator = true;

        self.globs
            .iter()
            .any(|pat| pat.matches_path_with(&path, match_options))
    }
}

#[derive(Default)]
pub struct FileSelectorBuilder {
    subdir: Option<PathBuf>,
    files: HashSet<PathBuf>,
    globs: Vec<Pattern>,
}

impl FileSelectorBuilder {
    pub fn file(mut self, path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        let path = path.strip_prefix("./").unwrap_or(path);

        let path = match &self.subdir {
            Some(p) => p.join(path),
            None => path.to_path_buf(),
        };
        self.files.insert(path);
        self
    }

    pub fn glob(mut self, pattern: &str) -> Result<Self, PatternError> {
        let pattern = match &self.subdir {
            Some(p) => Pattern::new(&format!("{}/{pattern}", p.display()))?,
            None => Pattern::new(pattern)?,
        };
        self.globs.push(pattern);
        Ok(self)
    }

    pub fn set_subdir(mut self, path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        let path = path.strip_prefix("./").unwrap_or(path);

        // if path != Path::new("") {
        self.subdir = Some(path.to_path_buf());
        // }
        self
    }

    pub fn build(self) -> FileSelector {
        FileSelector {
            files: self.files,
            globs: self.globs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_selector_matches_nothing() {
        let selector = FileSelector::builder().build();

        assert_eq!(selector.includes("foo.txt"), false);
    }

    #[test]
    fn explicit_file_includes_that() {
        let selector = FileSelector::builder().file("foo.txt").build();

        assert_eq!(selector.includes("foo.txt"), true);
    }

    #[test]
    fn subdir_prefixes_files() {
        let selector = FileSelector::builder()
            .set_subdir("some_dir")
            .file("foo.txt")
            .build();

        assert_eq!(selector.includes("foo.txt"), false);
        assert_eq!(selector.includes("some_dir/foo.txt"), true);
    }

    #[test]
    fn glob_matches_file() {
        let selector = FileSelector::builder().glob("*.txt").unwrap().build();

        assert_eq!(selector.includes("foo.txt"), true);
        assert_eq!(selector.includes("foo.rs"), false);
        assert_eq!(selector.includes("some/dir/foo.txt"), false);
    }

    #[test]
    fn glob_with_subdir() {
        let selector = FileSelector::builder()
            .set_subdir("some/dir")
            .glob("*.txt")
            .unwrap()
            .build();

        assert_eq!(selector.includes("some/dir/foo.txt"), true);
        assert_eq!(selector.includes("foo.txt"), false);
        assert_eq!(selector.includes("another/dir/foo.txt"), false);
    }

    #[test]
    fn selector_has_prefix() {
        let selector = FileSelector::builder().file("./foo.txt").build();

        assert_eq!(selector.includes("foo.txt"), true);
    }

    #[test]
    fn file_has_prefix() {
        let selector = FileSelector::builder().file("foo.txt").build();

        assert_eq!(selector.includes("./foo.txt"), true);
    }

    #[test]
    fn subdir_is_self() {
        let selector = FileSelector::builder()
            .set_subdir("./")
            .file("foo.txt")
            .build();

        assert_eq!(selector.includes("foo.txt"), true);
    }
}
