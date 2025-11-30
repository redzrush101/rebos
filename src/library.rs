#![allow(dead_code)]

use colored::Colorize;
use std::collections::{HashMap, HashSet};
use piglog::prelude::*;
use piglog::*;
use std::io;
use std::process::Command;


use crate::generation::Generation;

#[derive(PartialEq)]
pub enum HistoryMode {
    Remove,
    Add,
}

pub struct History {
    pub mode: HistoryMode,
    pub line: String,
}



pub fn run_command(command: &str) -> bool {
    match Command::new("bash").args(["-c", command]).status() {
        Ok(o) => o,
        Err(_e) => return false,
    }
    .success()
}

pub fn run_command_with_output(command: &str) -> Option<String> {
    match Command::new("bash").args(["-c", command]).output() {
        Ok(output) => {
            if !output.status.success() {
                return None;
            }
            String::from_utf8(output.stdout).ok()
        }
        Err(_e) => None,
    }
}



pub fn name_from_path(path: &str) -> String {
    path.split('/').last().unwrap_or("").to_string()
}

pub fn custom_error(error: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Other, error)
}

pub fn ensure_directories_exist(dirs: &[std::path::PathBuf]) -> Result<(), io::Error> {
    for dir in dirs {
        if !dir.exists() {
            std::fs::create_dir_all(dir)
                .map(|_| info!("Created directory: {}", dir.display()))?;
        }
    }
    Ok(())
}

pub fn for_each_manager<F>(managers: &Option<Vec<String>>, mut operation: F) -> Result<(), io::Error>
where
    F: FnMut(&str) -> Result<(), io::Error>,
{
    let man_names = match managers {
        Some(ref man_names) => man_names,
        None => &crate::management::get_managers()?,
    };
    
    for man_name in man_names {
        operation(man_name)?;
    }
    
    Ok(())
}

pub fn log_and_return<T, E>(result: Result<T, E>, error_msg: &str) -> Result<T, E>
where
    E: std::fmt::Debug,
{
    result.map_err(|e| {
        error!("{}", error_msg);
        e
    })
}

pub fn is_root_user() -> bool {
    username() == "root"
}

pub fn username() -> String {
    match std::env::var("USER") {
        Ok(username) => username,
        Err(_) => match std::env::var("USERNAME") {
            Ok(username) => username,
            Err(_) => "user".to_string(), // fallback
        }
    }
}

pub fn remove_array_duplicates<T: Clone + PartialEq + Eq + std::hash::Hash>(dup_vec: &[T]) -> Vec<T> {
    let mut seen: HashSet<&T> = HashSet::new();
    dup_vec.iter().filter(|item| seen.insert(item)).cloned().collect()
}

pub fn history_gen(gen_1: &Generation, gen_2: &Generation) -> HashMap<String, Vec<History>> {
    let mut history_map: HashMap<String, Vec<History>> = HashMap::new();

    for i in gen_2.managers.keys() {
        let items_2 = gen_2.managers.get(i).unwrap();

        match gen_1.managers.get(i) {
            Some(items_1) => {
                history_map.insert(i.to_string(), history(&items_1.items, &items_2.items))
            }
            None => history_map.insert(
                i.to_string(),
                items_2
                    .items
                    .iter()
                    .map(|x| History {
                        mode: HistoryMode::Add,
                        line: x.to_string(),
                    })
                    .collect(),
            ),
        };
    }

    for i in gen_1.managers.keys() {
        let items_1 = gen_1.managers.get(i).unwrap();

        match gen_2.managers.get(i) {
            Some(_) => (),
            None => {
                history_map.insert(
                    i.to_string(),
                    items_1
                        .items
                        .iter()
                        .map(|x| History {
                            mode: HistoryMode::Remove,
                            line: x.to_string(),
                        })
                        .collect(),
                );
            }
        };
    }

    history_map
}

pub fn print_history_gen(history: &HashMap<String, Vec<History>>) {
    for i in history.keys() {
        piglog::info!("{}:", i);

        print_history(history.get(i).unwrap());

        println!("");
    }
}

pub fn print_history(diff_vec: &Vec<History>) {
    for i in diff_vec.iter() {
        match i.mode {
            HistoryMode::Add => println!("{}", format!("+ {}", i.line).bright_green().bold()),
            HistoryMode::Remove => println!("{}", format!("- {}", i.line).bright_red().bold()),
        };
    }
}

pub fn history(array_1: &[String], array_2: &[String]) -> Vec<History> {
    let set_1: HashSet<&String> = array_1.iter().collect();
    let set_2: HashSet<&String> = array_2.iter().collect();

    let mut history_vec: Vec<History> = Vec::new();

    // Find removed items (in set_1 but not in set_2) - O(n) instead of O(n²)
    for item in &set_1 {
        if !item.trim().is_empty() && !set_2.contains(item) {
            history_vec.push(History {
                mode: HistoryMode::Remove,
                line: (*item).clone(),
            });
        }
    }

    // Find added items (in set_2 but not in set_1) - O(n) instead of O(n²)
    for item in &set_2 {
        if !item.trim().is_empty() && !set_1.contains(item) {
            history_vec.push(History {
                mode: HistoryMode::Add,
                line: (*item).clone(),
            });
        }
    }

    history_vec
}

pub fn hostname() -> Result<String, io::Error> {
    hostname::get()
        .and_then(|os_str| os_str.into_string()
            .map_err(|_| {
                error!("Failed to parse hostname OsString into String type!");
                custom_error("Failed to parse OsString into String!")
            }))
        .map_err(|e| {
            if e.kind() == io::ErrorKind::Other {
                e
            } else {
                error!("Failed to get system hostname!");
                e
            }
        })
}
