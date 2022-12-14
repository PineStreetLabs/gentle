use super::*;

use anyhow::Context;

#[linkme::distributed_slice(TARGET_DISCOVERY)]
fn discover(path: &Path) -> anyhow::Result<Vec<Box<dyn Target>>> {
    if path.join("go.mod").try_exists()? {
        Ok(vec![(Box::new(GoModTarget::new(&path)))])
    } else {
        Ok(Vec::new())
    }
}

pub struct GoModTarget {
    path: PathBuf,
}

impl GoModTarget {
    pub fn new(path: &Path) -> Self {
        Self { path: path.into() }
    }

    fn cache_dir(&self) -> PathBuf {
        std::env::var("GOCACHE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                Path::new(&std::env::var("HOME").unwrap_or(String::from("/")))
                    .join(".cache/go-build")
            })
            .into()
    }
}

impl Display for GoModTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let package = self.path.display().to_string().replacen("./", "", 1);
        write!(f, "//{package}:go_mod")
    }
}

impl Target for GoModTarget {
    fn perform_test(&self) -> anyhow::Result<()> {
        Command::new("go")
            .args(&["test"])
            .env("GOCACHE", self.cache_dir())
            .current_dir(&self.path)
            .output()?
            .success_ok()
            .map(|_| ())
            .map_err(|out| anyhow::anyhow!(out.stderr))
    }

    fn perform_lint(&self) -> anyhow::Result<()> {
        // TODO(shelbyd): Install required tools.
        Command::new("golangci-lint")
            .args(["--verbose"])
            .current_dir(&self.path)
            .output()
            .context("Running golangci-lint")?
            .success_ok()
            .map(|_| ())
            .map_err(|out| anyhow::anyhow!(out.stderr))
    }

    fn perform_format(&self) -> anyhow::Result<()> {
        let out = Command::new("go")
            .args(["fmt"])
            .current_dir(&self.path)
            .output()
            .context("Running `go fmt`")?
            .success_ok()
            .map_err(|out| anyhow::anyhow!(out.stderr))?;

        let modified = out
            .stdout
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect::<Vec<&str>>();

        if modified.is_empty() {
            return Ok(());
        }

        let padded = modified
            .into_iter()
            .map(|l| format!("    {l}"))
            .collect::<Vec<_>>()
            .join("\n");
        Err(anyhow::anyhow!("go fmt modified files:\n{padded}"))
    }

    fn perform_build(&self, build: &Build) -> anyhow::Result<()> {
        let current_dir = std::env::current_dir()?;

        Command::new("go")
            .args(&["build", "-o"])
            .arg(&current_dir.join(&build.out))
            .env("GOCACHE", self.cache_dir())
            .current_dir(&self.path)
            .output()?
            .success_ok()
            .map(|_| ())
            .map_err(|out| anyhow::anyhow!(out.stderr))
    }

    fn cache_paths(&self) -> HashSet<PathBuf> {
        [self.cache_dir()].into_iter().collect()
    }

    fn lock_files(&self) -> HashSet<PathBuf> {
        [self.path.join("go.sum")].into_iter().collect()
    }
}
