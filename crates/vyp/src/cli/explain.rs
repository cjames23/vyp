use clap::Args;
use std::path::PathBuf;

use crate::lock::lockfile::LockFile;

#[derive(Args)]
pub struct ExplainArgs {
    /// Package name to explain
    pub package: String,

    /// Path to lock file
    #[arg(short, long, default_value = "pylock.toml")]
    pub lockfile: PathBuf,
}

pub fn run(args: ExplainArgs) -> miette::Result<()> {
    if !args.lockfile.exists() {
        return Err(miette::miette!(
            "Lock file not found: {}. Run `vyp lock` first.",
            args.lockfile.display()
        ));
    }

    let lockfile = LockFile::read_from_file(&args.lockfile)?;

    match lockfile.explain_package(&args.package) {
        Some(explanation) => {
            println!("{}", explanation);
            Ok(())
        }
        None => Err(miette::miette!(
            "Package '{}' not found in lock file",
            args.package
        )),
    }
}
