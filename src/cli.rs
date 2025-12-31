use crate::Result;
use crate::commands::{
    add::handle_add_command, init::handle_init_command, list::handle_list_command,
    remove::handle_remove_command, history::handle_history_command,
};
use crate::store::Store;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "aka")]
#[command(about = "Instant terminal alias manager")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Alias name for implicit add/remove
    #[arg(required = false)]
    pub implicit_alias: Option<String>,

    /// Command value for implicit add
    #[arg(required = false)]
    pub implicit_value: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a new alias
    Add {
        /// Alias name (optional for history picker)
        alias: Option<String>,
        /// Command to alias (optional for history picker)
        command: Option<String>,

        /// Directory scope (defaults to current directory if not global)
        #[arg(long, short = 's', num_args(0..=1), default_missing_value = ".")]
        scope: Option<String>,

        /// Make the alias recursive for subdirectories
        #[arg(long, short)]
        recursive: bool,
    },
    /// Remove an alias
    #[command(visible_alias = "rm")]
    Remove {
        /// Alias name (optional with --all)
        #[arg(required_unless_present = "all")]
        alias: Option<String>,

        /// Remove all aliases
        #[arg(long, conflicts_with = "alias")]
        all: bool,

        /// Scope to remove (global or directory path)
        #[arg(long, short = 's')]
        scope: Option<String>,

        /// Skip confirmation prompt
        #[arg(long, short = 'f')]
        force: bool,
    },
    /// List all aliases
    #[command(visible_alias = "ls")]
    List {
        /// Show all aliases regardless of current scope
        #[arg(long, short)]
        all: bool,
    },
    /// Initialize shell integration
    Init {
        #[arg(long, hide = true)]
        dump: bool,
    },
    /// Install completion to shell
    Install,
}

pub async fn run_cli() -> Result<()> {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Add {
            alias,
            command,
            scope,
            recursive,
        }) => {
            let mut store = Store::new()?;
            match (alias, command) {
                (Some(a), Some(c)) => handle_add_command(&mut store, a, c, scope, recursive)?,
                (None, None) => {
                    handle_history_command(&mut store, None, scope, recursive, 200)?
                }
                _ => {
                    return Err(crate::error::AkaError::ConfigError(
                        "Both alias and command are required, or omit both to pick from history"
                            .to_string(),
                    )
                    .into())
                }
            }
        }
        Some(Commands::Remove {
            alias,
            all,
            scope,
            force,
        }) => {
            let mut store = Store::new()?;
            handle_remove_command(&mut store, alias, all, scope, force)?
        }
        Some(Commands::List { all }) => {
            let store = Store::new()?;
            handle_list_command(&store, all)?
        }
        Some(Commands::Init { dump }) => {
            if dump {
                let store = Store::new()?;
                handle_init_command(Some(&store), dump)?
            } else {
                handle_init_command(None, dump)?
            }
        }
        Some(Commands::Install) => crate::commands::install::handle_install_command()?,
        None => {
            // Handle implicit commands
            match (cli.implicit_alias, cli.implicit_value) {
                (Some(alias), Some(command)) => {
                    let mut store = Store::new()?;
                    handle_add_command(&mut store, alias, command, None, false)?
                }
                (Some(alias), None) => {
                    let mut store = Store::new()?;
                    handle_remove_command(&mut store, Some(alias), false, None, false)?
                }
                (None, None) => {
                    let store = Store::new()?;
                    handle_list_command(&store, false)?
                }
                _ => {
                    unreachable!("Invalid argument combination");
                }
            }
        }
    };

    println!("{}", result);

    Ok(())
}
