use anyhow::Context;
use std::{collections::BTreeSet, fs::File, path::PathBuf};

pub fn hash_files(files: impl IntoIterator<Item = PathBuf>) -> anyhow::Result<blake3::Hash> {
    let mut hasher = blake3::Hasher::new();

    let ordered_files: BTreeSet<_> = files.into_iter().collect();
    for path in ordered_files {
        let mut file = File::open(&path).context(format!("Opening {path:?}"))?;
        std::io::copy(&mut file, &mut hasher)?;
    }

    Ok(hasher.finalize())
}
