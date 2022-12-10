use anyhow::Context;
use std::{collections::BTreeSet, fs::File, path::PathBuf};

pub fn hash() -> anyhow::Result<blake3::Hash> {
    let mut hasher = blake3::Hasher::new();

    let lockfiles: BTreeSet<PathBuf> = crate::targets::targets()?
        .into_iter()
        .flat_map(|t| t.lockfiles())
        .collect();

    for path in lockfiles {
        let mut file = File::open(&path).context(format!("Opening {}", path.display()))?;
        std::io::copy(&mut file, &mut hasher)?;
    }

    Ok(hasher.finalize())
}
