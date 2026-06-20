use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args)]
pub struct ConflictArgs {
    #[command(subcommand)]
    pub command: ConflictCommands,
}

#[derive(Subcommand)]
pub enum ConflictCommands {
    /// List inherited conflict declarations from resolved dependencies
    List {
        /// Path to pyproject.toml
        #[arg(short, long, default_value = "pyproject.toml")]
        project: PathBuf,
    },
}

pub fn run(args: ConflictArgs) -> miette::Result<()> {
    match args.command {
        ConflictCommands::List { project } => run_list(&project),
    }
}

fn run_list(project: &std::path::Path) -> miette::Result<()> {
    let config = crate::config::settings::VypConfig::from_file(project)?;

    let mut builder = vyp_core::ResolverBuilder::new()
        .with_overrides(config.overrides.clone())
        .with_substitutions(config.substitutions.clone())
        .with_resolution_strategy(config.core_resolution_strategy())
        .with_pre_release_policy(config.core_pre_release_policy());

    config.load_plugins(builder.plugin_loader_mut());

    for req_str in &config.dependencies {
        let req: vyp_api::Requirement = req_str
            .parse()
            .map_err(|e| miette::miette!("Invalid requirement '{}': {}", req_str, e))?;
        builder = builder.add_dependency(req);
    }

    let (providers, router) = config.create_providers(None, None)?;
    for provider in providers {
        builder = builder.with_provider(provider);
    }
    builder = builder.with_index_router(router);

    let result = builder
        .resolve()
        .map_err(|e| miette::miette!("Resolution failed: {}", e))?;

    if result.inherited_conflicts.is_empty() {
        println!("No inherited conflicts from resolved dependencies.");
        return Ok(());
    }

    println!("Inherited conflict declarations:");
    for (pkg, conflicts) in &result.inherited_conflicts {
        for decl in &conflicts.declarations {
            println!("  {} (from {})", decl, pkg);
        }
    }
    Ok(())
}
