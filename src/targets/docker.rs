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

    fn src_files(&self) -> anyhow::Result<Option<FileSelector>> {
        let dockerfile = self.path.join("Dockerfile");
        let mut builder = FileSelector::builder().path(&dockerfile);

        let docker_contents = std::fs::read_to_string(&dockerfile)?;
        for line in docker_contents.lines() {
            let line = line.trim();
            let mut args = match line.strip_prefix("COPY ").or(line.strip_prefix("ADD ")) {
                Some(suf) => suf,
                None => continue,
            }
            .split(' ')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

            let _dest = args.pop().ok_or(anyhow::anyhow!("COPY with no args"))?;

            for arg in args {
                builder = builder.path(arg);
            }
        }

        Ok(Some(builder.build()))
    }
}
