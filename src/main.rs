mod cli;
mod core;
mod tui;
mod utils;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List => cli::commands::list::execute()?,
        Commands::Login { skip } => cli::commands::login::execute(skip)?,
        Commands::Switch { email } => cli::commands::switch_cmd::execute(email)?,
        Commands::Import { path, alias } => {
            cli::commands::import::execute(&path, alias.as_deref())?
        }
        Commands::Remove => cli::commands::remove::execute()?,
        Commands::Watch {
            interval,
            threshold,
            auto_switch,
            web,
            port,
        } => cli::commands::watch::execute(interval, threshold, auto_switch, web, port)?,
        Commands::Add { skip, no_login } => {
            eprintln!(
                "\x1b[1;31mwarning:\x1b[0m `add` is deprecated; use \x1b[1;32m`cx-switch login{}`\x1b[0m",
                if skip || no_login { " --skip" } else { "" }
            );
            cli::commands::login::execute(skip || no_login)?;
        }
    }

    Ok(())
}
