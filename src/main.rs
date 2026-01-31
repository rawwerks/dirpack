use std::fs;
use std::io::{self, Write};

use clap::Parser;

use dirpack::budget::BudgetTarget;
use dirpack::cli::{Cli, Commands, PackArgs};
use dirpack::config::{Config, OutputFormat};
use dirpack::packer;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let command = cli.command.unwrap_or_else(|| Commands::Pack(PackArgs::default()));

    match command {
        Commands::Pack(args) => run_pack(args)?,
        Commands::Init(args) => run_init(args)?,
        Commands::Tree(args) => run_tree(args)?,
    }

    Ok(())
}

fn run_pack(args: PackArgs) -> anyhow::Result<()> {
    // Resolve path
    let root = args.path.canonicalize().unwrap_or(args.path.clone());

    if !root.exists() {
        anyhow::bail!("Path does not exist: {}", root.display());
    }

    // Load config
    let config = if let Some(config_path) = &args.config {
        Config::load(config_path)?
    } else {
        // Try local config first, then global, then default
        let local_config = root.join("dirpack.toml");
        if local_config.exists() {
            Config::load(&local_config)?
        } else {
            Config::default()
        }
    };

    // Determine budget target
    let budget_target = if let Some(tokens) = args.target_tokens {
        BudgetTarget::Tokens(tokens)
    } else if let Some(bytes) = args.target_bytes {
        BudgetTarget::Bytes(bytes)
    } else {
        BudgetTarget::Tokens(config.output.default_budget_tokens)
    };

    // Determine output format
    let format = args.format.unwrap_or(config.output.format);

    // Run packer
    let use_git = !args.no_git;
    let include_signatures = !args.no_signatures;

    let result = packer::pack(&root, &config, budget_target, use_git, include_signatures);

    // Output
    let output = match format {
        OutputFormat::Pipe => result.output,
        OutputFormat::Full => format_full(&result),
        OutputFormat::Json => format_json(&result),
    };

    // Write output
    if let Some(output_path) = args.output {
        fs::write(&output_path, &output)?;
        if !args.quiet {
            eprintln!("Wrote {} bytes to {}", output.len(), output_path.display());
        }
    } else {
        print!("{}", output);
        io::stdout().flush()?;
    }

    // Print stats to stderr if verbose
    if args.verbose {
        eprintln!(
            "\n--- Stats ---\nFiles: {}\nBudget: {}/{} ({:.1}%)",
            result.files_included,
            result.budget_used,
            result.budget_limit,
            (result.budget_used as f64 / result.budget_limit as f64) * 100.0
        );
    }

    Ok(())
}

fn run_init(args: dirpack::cli::InitArgs) -> anyhow::Result<()> {
    let output_path = if args.global {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
            .join("dirpack");
        fs::create_dir_all(&config_dir)?;
        config_dir.join("config.toml")
    } else {
        args.output
    };

    if output_path.exists() && !args.force {
        anyhow::bail!(
            "Config file already exists: {}. Use --force to overwrite.",
            output_path.display()
        );
    }

    let default_config = include_str!("../default_config.toml");
    fs::write(&output_path, default_config)?;
    println!("Created config at: {}", output_path.display());

    Ok(())
}

fn run_tree(args: dirpack::cli::TreeArgs) -> anyhow::Result<()> {
    let root = args.path.canonicalize().unwrap_or(args.path.clone());
    let config = Config::default();

    let entries = dirpack::scanner::scan(&root, &config.scanning, true);

    let tree = dirpack::packer::spine::format_tree_ascii(&entries);
    println!("{}", tree);

    if args.show_priority {
        println!("\n--- Priorities ---");
        for entry in entries.iter().filter(|e| !e.is_dir) {
            let priority =
                dirpack::priority::calculate_priority(entry, &config.priority_rules, &config.categories);
            println!("{}: {}", entry.relative_path.display(), priority);
        }
    }

    Ok(())
}

fn format_full(result: &packer::PackResult) -> String {
    format!(
        "# Directory Index\n\nBudget: {}/{}\nFiles: {}\n\n{}",
        result.budget_used, result.budget_limit, result.files_included, result.output
    )
}

fn format_json(result: &packer::PackResult) -> String {
    format!(
        r#"{{"budget_used":{},"budget_limit":{},"files":{},"output":{}}}"#,
        result.budget_used,
        result.budget_limit,
        result.files_included,
        serde_json::to_string(&result.output).unwrap_or_else(|_| "\"\"".to_string())
    )
}
