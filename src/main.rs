mod cli;
mod config;
mod dispatch;
mod generation;
mod git;
mod hook;
mod library;

mod management;
mod obj_print;
mod obj_print_boilerplate;
mod places;



use clap::Parser;
use colored::Colorize;

use std::path::{Path, PathBuf};
use library::*;
use piglog::prelude::*;
use piglog::*;
use std::io::{self, Write};

#[derive(PartialEq)]
enum ExitCode {
    Success,
    Fail,
}

fn test_code() {}

fn error_cleanup() {
}

fn main() -> std::process::ExitCode {
    match app() {
        ExitCode::Success => std::process::ExitCode::SUCCESS,
        ExitCode::Fail => {
            error_cleanup();

            std::process::ExitCode::FAILURE
        }
    }
}

fn app() -> ExitCode {

    test_code();

    match is_root_user() {
        true => {
            error!("Cannot run as root! Please run as the normal user!");
            return ExitCode::Fail;
        }

        false => {}
    };

    if places::base_legacy().exists() {
        warning!("Detected Rebos base at legacy location, moving it to new location...");
        generic!(
            "'{}' -> '{}'",
            places::base_legacy().display().to_string(),
            places::base().display().to_string()
        );

        if places::base().exists() {
            match std::fs::remove_dir_all(&places::base()) {
                Ok(_) => (),
                Err(e) => {
                    fatal!(
                        "Failed to delete directory: '{}'",
                        places::base().display().to_string()
                    );
                    println!("{e:#?}");

                    return ExitCode::Fail;
                }
            };
        }

        match std::fs::rename(&places::base_legacy(), &places::base()) {
            Ok(_) => (),
            Err(e) => {
                fatal!(
                    "Failed to move directory ('{}') to new location: '{}'",
                    places::base_legacy().display().to_string(),
                    places::base().display().to_string()
                );
                println!("{e:#?}");

                return ExitCode::Fail;
            }
        };

        success!("Moved Rebos base directory to new location!");
    }

    let args = cli::Cli::parse();

    match &args.command {
        cli::Commands::Setup => (),
        _ => {
            if places::base().exists() == false {
                error!("It seems that the program is not set up!");
                return ExitCode::Fail;
            }
        }
    }

    match dispatch::handle_command(args) {
        Ok(_) => return ExitCode::Success,
        Err(_) => return ExitCode::Fail,
    };
}



pub fn bool_question<S: AsRef<str>>(question: S, fallback: bool) -> bool {
    let question = question.as_ref();

    let (yes, no) = match fallback {
        true => ("Y".bright_green().bold().underline(), "n".bright_red()),
        false => ("y".bright_green(), "N".bright_red().bold().underline()),
    };

    loop {
        let answer = input(format!(
            "{question} [{yes}/{no}]: ",
            question = question.bright_cyan(),
        ));

        let match_on = answer.trim().to_lowercase();

        match match_on.as_str() {
            "yes" | "y" | "yeah" | "yeh" | "true" => return true,
            "no" | "n" | "nope" | "nah" | "false" => return false,
            "" => return fallback,
            _ => {
                eprintln!("Invalid response: '{}'", match_on);
            }
        }
    }
}

pub fn input<S: AsRef<str>>(prefix: S) -> String {
    let mut answer = String::new();

    print!("{}", prefix.as_ref());

    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut answer).unwrap();

    answer
}
