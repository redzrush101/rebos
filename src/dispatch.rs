use crate::cli::{self, Commands};
use crate::generation;
use crate::lock;
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
        Commands::ForceUnlock => handle_force_unlock()?,
        Commands::IsUnlocked => handle_is_unlocked()?,
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
    match lock::lock_on() {
        Ok(_) => (),
        Err(_) => return Err("Failed to acquire lock".into()),
    };

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
        cli::GenCommands::CleanDups => {
            match generation::management::clean_dups(true) {
                Ok(o) => success!("Deleted {o} generations!"),
                Err(_) => return Err("Failed to clean duplicate generations".into()),
            };
        }
        cli::GenCommands::Align => {
            match generation::management::align(true) {
                Ok(o) => success!("Aligned {o} generations!"),
                Err(_) => return Err("Failed to align generations".into()),
            };
        }
        cli::GenCommands::TidyUp => {
            match generation::management::tidy_up() {
                Ok(_) => (),
                Err(_) => return Err("Failed to tidy up generations".into()),
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
            info!(
                "Latest generation number is: {}",
                match generation::latest_number() {
                    Ok(o) => o,
                    Err(_) => return Err("Failed to get latest generation number".into()),
                }
            );
        }
        cli::GenCommands::DeleteOld(h) => {
            info!("Deleting old generations...");

            match generation::delete_old(h.how_many, true) {
                Ok(_) => success!("Successfully deleted {} generations!", h.how_many),
                Err(_) => return Err("Failed to delete old generations".into()),
            };
        }
        cli::GenCommands::Delete(g) => {
            match generation::delete(g.generation, true) {
                Ok(_) => (), // Handled by delete().
                Err(_) => return Err("Failed to delete generation".into()),
            };
        }
        cli::GenCommands::Diff { old, new } => {
            if generation::gen_exists(*old) == false
                || generation::gen_exists(*new) == false
            {
                fatal!("Generation not found!");
                return Err("Generation not found".into());
            }

            let gen_1 = generation::get_gen_from_usize(*old).unwrap();
            let gen_2 = generation::get_gen_from_usize(*new).unwrap();

            let commit_1 = generation::get_gen_commit_from_usize(*old).unwrap();
            let commit_2 = generation::get_gen_commit_from_usize(*new).unwrap();

            let history = library::history_gen(&gen_1, &gen_2);

            println!(
                "\n{} {} {}",
                commit_1.bright_cyan().bold(),
                "->".bright_black().bold(),
                commit_2.bright_cyan().bold()
            );

            println!("");

            library::print_history_gen(&history);
        }
        cli::GenCommands::Current { command } => {
            handle_current_command(command)?;
        }
    };

    match lock::lock_off() {
        Ok(_) => (),
        Err(_) => return Err("Failed to release lock".into()),
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

            match generation::set_current(s.to, true) {
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

fn handle_force_unlock() -> Result<(), Box<dyn std::error::Error>> {
    if lock::is_lock_on() {
        piglog::warning!(
            "Force unlocking could harm the system if done with the wrong reason!"
        );
        piglog::warning!(
            "You should only force unlock if you know that you ABSOLUTELY need to!"
        );
        piglog::warning!(
            "{} {} {}",
            "Really the ONLY time you should do this is if there is only",
            "one Rebos process running, but the locking file was never",
            "cleaned up, so Rebos thinks there is another Rebos process!",
        );

        if crate::bool_question("Are you REALLY sure you want to do this?", false) {
            piglog::warning!(
                "Force unlocking... use {} to cancel...",
                "CTRL + C".bright_red().bold(),
            );

            let countdown_from: u8 = 5;

            print!("Countdown: ");
            for i in 0..countdown_from {
                print!("{} ", format!("{}", countdown_from - i).bright_red().bold());
                std::io::stdout().flush().unwrap();
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            print!("\n");

            match lock::lock_off_force() {
                Ok(_) => piglog::success!("Unlocked Rebos!"),
                Err(e) => {
                    piglog::fatal!("Failed to unlock: {e}");
                    return Err("Failed to force unlock".into());
                }
            };
        } else {
            piglog::info!("Aborting...");
            return Err("Aborted by user".into());
        }
    } else {
        piglog::info!("Not locked... skipping...");
    }
    Ok(())
}

fn handle_is_unlocked() -> Result<(), Box<dyn std::error::Error>> {
    match lock::is_lock_on() {
        false => Ok(()),
        true => Err("System is locked".into()),
    }
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