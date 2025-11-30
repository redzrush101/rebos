use crate::cli::{self, Commands};
use crate::generation;

use crate::management;
use crate::config;
use crate::places;
use crate::library;
use piglog::prelude::*;
use piglog::*;
use colored::Colorize;
use std::io::Write;

pub fn handle_command(args: cli::Cli) -> Result<(), Box<dyn std::error::Error>> {
    match &args.command {
        cli::Commands::Setup => {
            return handle_setup();
        }
        _ => {
            if places::base().exists() == false {
                error!("It seems that the program is not set up!");
                return Err("Program not set up".into());
            }
        }
    }

    match &args.command {
        Commands::Gen { command } => handle_gen_command(command)?,
        Commands::Config { command } => handle_config_command(command)?,
        
        Commands::Managers { command, managers } => handle_managers_command(command, managers)?,
        Commands::API { command } => handle_api_command(command)?,
        _ => {
            error!("Command not usable yet!");
            return Err("Command not usable yet".into());
        }
    }

    Ok(())
}

fn handle_setup() -> Result<(), Box<dyn std::error::Error>> {
    info!("Beginning setup...");

    match setup() {
        Ok(_) => success!("Set up the program successfully!"),
        Err(_) => return Err("Failed to setup program".into()),
    };
    Ok(())
}

fn setup() -> Result<(), std::io::Error> {
    match places::setup() {
        Ok(_) => success!("Core directories verified successfully!"),
        Err(e) => return Err(e),
    };
    Ok(())
}

fn handle_gen_command(command: &cli::GenCommands) -> Result<(), Box<dyn std::error::Error>> {

    match command {
        cli::GenCommands::Commit(c) => {
            info!("Committing user generation...");

            match generation::commit(c.msg.as_str()) {
                Ok(_) => success!("Committed generation successfully! (\"{}\")", c.msg),
                Err(_) => return Err("Failed to commit generation".into()),
            };
        }
        cli::GenCommands::List => {
            match generation::list_print() {
                Ok(_) => (),
                Err(_) => return Err("Failed to list generations".into()),
            };
        }
        
        cli::GenCommands::Info => {
            let generation = match generation::gen(crate::config::ConfigSide::User) {
                Ok(o) => o,
                Err(_) => return Err("Failed to get generation".into()),
            };

            crate::obj_print::generation(&generation);
        }
        cli::GenCommands::Latest => {
            match generation::list() {
                Ok(generations) => {
                    if !generations.is_empty() {
                        info!("Latest generation is: {}", generations[0].0);
                    } else {
                        warning!("No generations found");
                    }
                }
                Err(_) => return Err("Failed to get latest generation".into()),
            };
        }
        
        cli::GenCommands::Diff { old, new } => {
            let hash_1 = match generation::get_hash_from_number(*old) {
                Ok(hash) => hash,
                Err(_) => {
                    fatal!("Generation {} not found!", old);
                    return Err("Generation not found".into());
                }
            };
            
            let hash_2 = match generation::get_hash_from_number(*new) {
                Ok(hash) => hash,
                Err(_) => {
                    fatal!("Generation {} not found!", new);
                    return Err("Generation not found".into());
                }
            };

            let gen_1 = match generation::get_gen_from_hash(&hash_1) {
                Ok(gen) => gen,
                Err(_) => return Err("Failed to get generation".into()),
            };
            
            let gen_2 = match generation::get_gen_from_hash(&hash_2) {
                Ok(gen) => gen,
                Err(_) => return Err("Failed to get generation".into()),
            };

            let history = library::history_gen(&gen_1, &gen_2);

            println!(
                "
{} {} {}",
                format!("gen:{}", old).bright_cyan().bold(),
                "->".bright_black().bold(),
                format!("gen:{}", new).bright_cyan().bold()
            );

            println!("");

            library::print_history_gen(&history);
        }
        cli::GenCommands::Current { command } => {
            handle_current_command(command)?;
        }
    };

    Ok(())
}

fn handle_current_command(command: &cli::CurrentCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        cli::CurrentCommands::Build => {
            info!("Building 'current' generation...");

            match generation::build() {
                Ok(_) => success!("Built generation successfully!"),
                Err(_) => return Err("Failed to build current generation".into()),
            };
        }
        cli::CurrentCommands::Rollback(r) => {
            info!("Rolling back by {} generations...", r.by);

            match generation::rollback(r.by, true) {
                Ok(_) => success!("Rolled back successfully!"),
                Err(_) => return Err("Failed to rollback generation".into()),
            };
        }
        cli::CurrentCommands::ToLatest => {
            info!("Jumping to latest generation...");

            match generation::latest(true) {
                Ok(_) => success!("Jumped to latest successfully!"),
                Err(_) => return Err("Failed to set to latest generation".into()),
            };
        }
        cli::CurrentCommands::Set(s) => {
            info!("Jumping to generation {}...", s.to);
            
            let hash = match generation::get_hash_from_number(s.to) {
                Ok(hash) => hash,
                Err(_) => return Err("Failed to get generation hash".into()),
            };

            match generation::set_current_hash(&hash, true) {
                Ok(_) => success!("Jumped to generation {} successfully!", s.to),
                Err(_) => return Err("Failed to set current generation".into()),
            };
        }
    }
    Ok(())
}

fn handle_config_command(command: &cli::ConfigCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        cli::ConfigCommands::Init => {
            info!("Creating user configuration...");

            match config::init_user_config() {
                Ok(_) => success!("Created user configuration successfully!"),
                Err(_) => return Err("Failed to initialize config".into()),
            };
        }
        cli::ConfigCommands::Check => {
            let result = match config::check_config() {
                Ok(o) => o,
                Err(_) => return Err("Failed to check config".into()),
            };

            match result {
                Ok(misc_info) => config::print_misc_info(&misc_info),
                Err((e, misc_info)) => {
                    config::print_errors_and_misc_info(&e, &misc_info);
                    return Err("Config check failed".into());
                }
            };
        }
    }
    Ok(())
}



fn handle_managers_command(command: &cli::ManagerCommands, managers: &Option<Vec<String>>) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        cli::ManagerCommands::Sync => {
            match management::sync_managers(managers) {
                Ok(_) => (),
                Err(_) => return Err("Failed to sync managers".into()),
            };
        }
        cli::ManagerCommands::Upgrade { sync } => {
            match management::upgrade_managers(*sync, managers) {
                Ok(_) => (),
                Err(_) => return Err("Failed to upgrade managers".into()),
            };
        }
        cli::ManagerCommands::ListOthers { remove } => {
            match management::list_others(managers, *remove) {
                Ok(_) => (),
                Err(_) => return Err("Failed to list others".into()),
            };
        }
    }
    Ok(())
}

fn handle_api_command(command: &cli::APICommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        cli::APICommands::Echo { log_mode, message } => {
            piglog::log_core_print(message.to_string(), *log_mode);
        }
        cli::APICommands::EchoGeneric { message } => {
            piglog::log_generic_print(message.to_string());
        }
        cli::APICommands::BoolQuestion { question, fallback } => {
            match crate::bool_question(question, fallback.bool()) {
                true => return Ok(()),
                false => return Err("Boolean question returned false".into()),
            }
        }
    }
    Ok(())
}