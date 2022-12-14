use super::*;

#[linkme::distributed_slice(TARGET_DISCOVERY)]
fn discover(path: &Path) -> anyhow::Result<Vec<Box<dyn Target>>> {
    if path.join("Dockerfile").try_exists()? {
        Ok(vec![(Box::new(DockerfileTarget::new(&path)))])
    } else {
        Ok(Vec::new())
    }
}

pub struct DockerfileTarget {
    path: PathBuf,
}

impl DockerfileTarget {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }
}

impl Display for DockerfileTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO(shelbyd): De-duplicate formatting of target addresses.
        let package = self.path.display().to_string().replacen("./", "", 1);
        write!(f, "//{package}:docker_image")
    }
}

impl Target for DockerfileTarget {
    fn perform_lint(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn perform_format(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn perform_build(&self, _: &Build) -> anyhow::Result<()> {
        Ok(())
    }

    fn perform_test(&self) -> anyhow::Result<()> {
        Command::new("docker")
            .args([
                "build",
                &format!("--file={}", self.path.join("Dockerfile").display()),
                ".",
            ])
            .output()?
            .success_ok()
            .map(|_| ())
            .map_err(|out| anyhow::anyhow!("{}", out.stderr))
    }
}
