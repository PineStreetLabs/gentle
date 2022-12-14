use super::{file_selector::FileSelector, Build};

use std::{collections::*, fmt::Display, path::*, process::*};

mod docker;
mod go;
mod rust;

pub fn targets() -> anyhow::Result<Vec<Box<dyn Target>>> {
    let mut result = Vec::new();

    for entry in ignore::Walk::new("./") {
        let entry = entry?;

        let is_dir = entry.file_type().expect("no stdin/stdout").is_dir();
        if !is_dir {
            continue;
        }
        let path = entry.into_path();

        for factory in TARGET_DISCOVERY {
            result.extend(factory(&path)?);
        }
    }

    Ok(result)
}

#[linkme::distributed_slice]
static TARGET_DISCOVERY: [fn(&Path) -> anyhow::Result<Vec<Box<dyn Target>>>] = [..];

pub trait Target: Display + Send + Sync + 'static {
    fn perform_test(&self) -> anyhow::Result<()>;
    // TODO(shelbyd): Default to successful and logging implementation.
    fn perform_lint(&self) -> anyhow::Result<()>;
    fn perform_format(&self) -> anyhow::Result<()>;
    fn perform_build(&self, build: &Build) -> anyhow::Result<()>;

    fn cache_paths(&self) -> HashSet<PathBuf> {
        Default::default()
    }

    fn lock_files(&self) -> HashSet<PathBuf> {
        Default::default()
    }

    fn src_files(&self) -> anyhow::Result<Option<FileSelector>> {
        Ok(None)
    }
}

trait OutputExt {
    fn success_ok(self) -> Result<StringOutput, StringOutput>;
}

impl OutputExt for Output {
    fn success_ok(self) -> Result<StringOutput, StringOutput> {
        let output = StringOutput {
            stdout: String::from_utf8_lossy(&self.stdout).to_string(),
            stderr: String::from_utf8_lossy(&self.stderr).to_string(),
        };
        if self.status.success() {
            Ok(output)
        } else {
            Err(output)
        }
    }
}

#[derive(Debug)]
struct StringOutput {
    stdout: String,
    stderr: String,
}
