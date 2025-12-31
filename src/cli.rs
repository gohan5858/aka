use crate::Result;
use crate::commands::{
    add::handle_add_command, init::handle_init_command, list::handle_list_command,
    remove::handle_remove_command,
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
        /// Alias name
        alias: String,
        /// Command to alias
        command: String,
    },
    /// Remove an alias
    Remove {
        /// Alias name
        alias: String,
    },
    /// List all aliases
    List,
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
        Some(Commands::Add { alias, command }) => {
            let mut store = Store::new()?;
            handle_add_command(&mut store, alias, command)?
        }
        Some(Commands::Remove { alias }) => {
            let mut store = Store::new()?;
            handle_remove_command(&mut store, alias)?
        }
        Some(Commands::List) => {
            let store = Store::new()?;
            handle_list_command(&store)?
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
                    handle_add_command(&mut store, alias, command)?
                }
                (Some(alias), None) => {
                    let mut store = Store::new()?;
                    handle_remove_command(&mut store, alias)?
                }
                (None, None) => {
                    let store = Store::new()?;
                    handle_list_command(&store)?
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
