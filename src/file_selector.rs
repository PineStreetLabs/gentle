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
        let path = simplify(path);

        let included_by_file = path.ancestors().any(|a| self.files.contains(a));
        if included_by_file {
            return true;
        }

        let mut match_options = MatchOptions::default();
        match_options.require_literal_separator = true;

        self.globs
            .iter()
            .any(|pat| pat.matches_path_with(&path, match_options))
    }

    pub fn include(&mut self, other: FileSelector) {
        self.files.extend(other.files);
        self.globs.extend(other.globs);
    }
}

#[derive(Default, Debug)]
pub struct FileSelectorBuilder {
    subdir: PathBuf,
    files: HashSet<PathBuf>,
    globs: Vec<Pattern>,
}

impl FileSelectorBuilder {
    pub fn path(mut self, path: impl AsRef<Path>) -> Self {
        self.files.insert(self.subdir.join(simplify(path)));
        self
    }

    pub fn glob(mut self, pattern: &str) -> Result<Self, PatternError> {
        let pattern = Pattern::new(&self.subdir.join(pattern).display().to_string())?;
        self.globs.push(pattern);
        Ok(self)
    }

    pub fn set_subdir(mut self, path: impl AsRef<Path>) -> Self {
        self.subdir = simplify(path).to_path_buf();
        self
    }

    pub fn build(self) -> FileSelector {
        FileSelector {
            files: self.files,
            globs: self.globs,
        }
    }
}

fn simplify(path: impl AsRef<Path>) -> PathBuf {
    use std::path::Component;

    let path = path.as_ref();

    let mut stack = Vec::new();
    for comp in path.components() {
        if comp == Component::CurDir {
            continue;
        }

        if comp == Component::ParentDir {
            stack.pop().expect("parent dir escapes path");
            continue;
        }

        stack.push(comp);
    }

    stack.into_iter().collect()
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
        let selector = FileSelector::builder().path("foo.txt").build();

        assert_eq!(selector.includes("foo.txt"), true);
    }

    #[test]
    fn dir_path_includes_children() {
        let selector = FileSelector::builder().path("some/dir").build();

        assert_eq!(selector.includes("some/dir/foo.txt"), true);
    }

    #[test]
    fn subdir_prefixes_files() {
        let selector = FileSelector::builder()
            .set_subdir("some_dir")
            .path("foo.txt")
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
        let selector = FileSelector::builder().path("./foo.txt").build();

        assert_eq!(selector.includes("foo.txt"), true);
    }

    #[test]
    fn file_has_prefix() {
        let selector = FileSelector::builder().path("foo.txt").build();

        assert_eq!(selector.includes("./foo.txt"), true);
    }

    #[test]
    fn subdir_is_self() {
        let selector = FileSelector::builder()
            .set_subdir("./")
            .path("foo.txt")
            .build();

        assert_eq!(selector.includes("foo.txt"), true);
    }

    #[test]
    fn subdir_with_relative() {
        let selector = FileSelector::builder()
            .set_subdir("some/dir/../another")
            .path("foo.txt")
            .build();

        assert_eq!(selector.includes("some/dir/foo.txt"), false);
        assert_eq!(selector.includes("some/another/foo.txt"), true);
    }

    #[test]
    fn simplify_cases() {
        assert_eq!(simplify("some/dir"), PathBuf::from("some/dir"));
        assert_eq!(simplify("./some/dir"), PathBuf::from("some/dir"));
        assert_eq!(simplify("dir/.."), PathBuf::from(""));
        assert_eq!(simplify("dir/../some/path"), PathBuf::from("some/path"));
    }
}
