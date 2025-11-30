#![allow(dead_code)]

use clap::{Parser, Subcommand, ValueEnum};
use piglog::LogMode;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Gen {
        #[command(subcommand)]
        command: GenCommands,
    },
    Setup,
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    
    Managers {
        #[command(subcommand)]
        command: ManagerCommands,
        #[arg(long = "manager", short, value_name = "MANAGER")]
        managers: Option<Vec<String>>,
    },
    API {
        #[command(subcommand)]
        command: APICommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum APICommands {
    Echo { log_mode: LogMode, message: String },
    EchoGeneric { message: String },
    BoolQuestion {
        question: String,
        fallback: CLIBoolean,
    },
}

#[derive(Subcommand, Debug)]
pub enum ManagerCommands {
    Sync,
    Upgrade {
        #[clap(long)]
        sync: bool,
    },
    ListOthers {
        #[clap(long)]
        remove: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    Init,
    Check,
}

#[derive(Subcommand, Debug)]
pub enum GenCommands {
    Commit(Commit),
    List,
    Info,
    Latest,
    
    Diff {
        old: usize,
        new: usize,
    },
    Current {
        #[command(subcommand)]
        command: CurrentCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum CurrentCommands {
    Build,
    Rollback(Rollback),
    ToLatest,
    Set(SetCurrent),
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum CLIBoolean {
    Yes,
    No,
}

impl CLIBoolean {
    #[inline(always)]
    pub fn bool(&self) -> bool {
        match self {
            Self::Yes => true,
            Self::No => false,
        }
    }
}

#[derive(Parser, Debug)]
pub struct Commit {
    pub msg: String,
}

#[derive(Parser, Debug)]
pub struct SetCurrent {
    pub to: usize,
}

#[derive(Parser, Debug)]
pub struct Rollback {
    pub by: isize,
}
