use clap::Args;
use std::path::PathBuf;

use crate::lock::lockfile::LockFile;

#[derive(Args)]
pub struct DiffArgs {
    /// First lock file (old)
    pub old: PathBuf,

    /// Second lock file (new)
    pub new: PathBuf,
}

pub fn run(args: DiffArgs) -> miette::Result<()> {
    let old = LockFile::read_from_file(&args.old)?;
    let new = LockFile::read_from_file(&args.new)?;

    let diff = crate::lock::diff::diff_lockfiles(&old, &new);
    println!("{}", diff);

    Ok(())
}
