use super::*;

use anyhow::Context;

#[linkme::distributed_slice(TARGET_DISCOVERY)]
fn discover(path: &Path) -> anyhow::Result<Vec<Box<dyn Target>>> {
    if path.join("Cargo.toml").try_exists()? {
        Ok(vec![(Box::new(RustCargoTarget::new(&path)))])
    } else {
        Ok(Vec::new())
    }
}

pub struct RustCargoTarget {
    path: PathBuf,
}

impl RustCargoTarget {
    fn new(path: &Path) -> Self {
        Self { path: path.into() }
    }
}

impl Display for RustCargoTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO(shelbyd): De-duplicate formatting of target addresses.
        let package = self.path.display().to_string().replacen("./", "", 1);
        write!(f, "//{package}:rust_crate")
    }
}

impl Target for RustCargoTarget {
    fn perform_test(&self) -> anyhow::Result<()> {
        Command::new("cargo")
            .args(&[
                "test",
                "--manifest-path",
                &self.path.join("Cargo.toml").to_string_lossy(),
                "--color=always",
            ])
            .output()?
            .success_ok()
            .map(|_| ())
            .map_err(|out| anyhow::anyhow!("{}\n{}", out.stderr, out.stdout))
    }

    fn perform_lint(&self) -> anyhow::Result<()> {
        Command::new("cargo")
            .args([
                "clippy",
                "--manifest-path",
                &self.path.join("Cargo.toml").to_string_lossy(),
                "--no-deps",
                "--color=always",
                "--",
                "--deny=warnings",
            ])
            .output()?
            .success_ok()
            .map(|_| ())
            .map_err(|out| anyhow::anyhow!("{}\n{}", out.stderr, out.stdout))
    }

    fn perform_format(&self) -> anyhow::Result<()> {
        Command::new("cargo")
            .args([
                "fmt",
                "--manifest-path",
                &self.path.join("Cargo.toml").to_string_lossy(),
                "--check",
            ])
            .output()?
            .success_ok()
            .map(|_| ())
            .map_err(|out| anyhow::anyhow!("{}\n{}", out.stderr, out.stdout))
    }

    fn perform_build(&self, build: &Build) -> anyhow::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        Command::new("cargo")
            .args(&[
                "build",
                "--release",
                "--manifest-path",
                &self.path.join("Cargo.toml").to_string_lossy(),
                "--color=always",
            ])
            .output()?
            .success_ok()
            .map_err(|out| anyhow::anyhow!("{}\n{}", out.stderr, out.stdout))?;

        let release_dir = self.path.join("target/release");
        for entry in std::fs::read_dir(&release_dir)
            .context(format!("Listing contents of {release_dir:?}"))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                continue;
            }
            let permissions = path.metadata()?.permissions();
            let is_executable = permissions.mode() & 0o100 != 0;

            if !is_executable {
                continue;
            }

            let filename = path.file_name().expect("inside target/release");
            std::fs::copy(&path, build.out.join(filename))?;
        }

        Ok(())
    }

    fn cache_paths(&self) -> HashSet<PathBuf> {
        [self.path.join("target")].into_iter().collect()
    }

    fn lockfiles(&self) -> HashSet<PathBuf> {
        [self.path.join("Cargo.lock")]
            .into_iter()
            .filter(|p| p.exists())
            .collect()
    }
}
