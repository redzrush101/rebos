#![allow(dead_code)]

use colored::Colorize;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use piglog::prelude::*;
use piglog::*;
use serde::{Deserialize, Serialize};
use std::io;

use crate::config::config_for;
use crate::config::{Config, ConfigSide};
use crate::git;
use crate::hook;
use crate::library::*;

use crate::management::load_manager;
use crate::places;




#[derive(PartialEq, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields, default)]
pub struct ManagerOrder {
    pub begin: Vec<String>,
    pub end: Vec<String>,
}

impl Default for ManagerOrder {
    fn default() -> Self {
        Self {
            begin: Vec::new(),
            end: Vec::new(),
        }
    }
}

#[derive(PartialEq, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields, default)]
pub struct Items {
    pub items: Vec<String>,
}

impl Default for Items {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}



#[derive(PartialEq, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields, default)]
pub struct Generation {
    pub imports: Vec<String>,
    pub managers: HashMap<String, Items>,
}

impl Default for Generation {
    fn default() -> Generation {
        Generation {
            imports: Vec::new(),
            managers: HashMap::new(),
        }
    }
}

impl GenerationUtils for Generation {
    fn extend(&mut self, other_gen: Generation) {
        self.imports.extend(other_gen.imports);

        for i in other_gen.managers.keys() {
            match self.managers.get_mut(i) {
                Some(s) => s
                    .items
                    .extend(other_gen.managers.get(i).unwrap().items.clone()),
                None => {
                    self.managers
                        .insert(i.to_string(), Items { items: Vec::new() });
                    self.managers
                        .get_mut(i)
                        .unwrap()
                        .items
                        .extend(other_gen.managers.get(i).unwrap().items.clone());
                }
            };
        }
    }
}

pub trait GenerationUtils {
    /// Extend all of the fields from one Generation object to another, another being the caller
    fn extend(&mut self, other_gen: Generation);
}

// Return generation structure for...
pub fn gen(side: ConfigSide) -> Result<Generation, io::Error> {
    let mut generation = match read_to_gen(&config_for(Config::Generation, side)?) {
        Ok(o) => o,
        Err(e) => return Err(e),
    };

    let system_hostname = match crate::library::hostname() {
        Ok(o) => o,
        Err(e) => return Err(e),
    };

    if side == ConfigSide::User {
        generation.extend(read_to_gen(
            &places::base_user()
                .join("machines")
                .join(&system_hostname)
                .join("gen.toml"),
        )?);
    }

    while generation.imports.len() > 0 {
        let gen_imports = generation.imports.clone();

        for i in gen_imports.iter() {
            let i_gen = read_to_gen(
                &places::base_user()
                    .join("imports")
                    .join(format!("{i}.toml")),
            )?;

            generation.extend(i_gen);
        }

        let after_gen_imports = generation.imports.clone();

        for i in 0..after_gen_imports.len() {
            if gen_imports.contains(&after_gen_imports[i]) {
                generation.imports[i] = String::new();
            }
        }

        generation.imports = generation
            .imports
            .into_iter()
            .filter(|x| *x != String::new())
            .collect();
    }

    Ok(generation)
}



// Read a file and return a Generation object.
fn read_to_gen(path: &Path) -> Result<Generation, io::Error> {
    let gen_string = match std::fs::read_to_string(path) {
        Ok(o) => o,
        Err(e) => {
            // Don't error if file doesn't exist, just return default generation
            if e.kind() == io::ErrorKind::NotFound {
                return Ok(Generation::default());
            }
            error!("Failed to read generation TOML file!");
            return Err(e);
        }
    };

    match toml::from_str(&gen_string) {
        Ok(o) => Ok(o),
        Err(e) => {
            error!("Failed to deserialize generation file:");
            error!("{e:#?}");
            error!("Path: '{}'", path.display());

            Err(custom_error("Failed to deserialize generation!"))
        }
    }
}

// Get generation from Git commit hash
pub fn get_gen_from_hash(hash: &str) -> Result<Generation, io::Error> {
    let repo = git::repo();
    
    // Try to get gen.toml from the commit
    match repo.get_file_content_at_hash(hash, "gen.toml") {
        Ok(content) => {
            match toml::from_str(&content) {
                Ok(gen) => Ok(gen),
                Err(e) => {
                    error!("Failed to deserialize generation from commit {}", hash);
                    Err(io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
                }
            }
        }
        Err(e) => {
            // If gen.toml doesn't exist in this commit, return default generation
            if e.kind() == io::ErrorKind::NotFound {
                return Ok(Generation::default());
            }
            Err(e)
        }
    }
}

// Get current generation hash
pub fn get_current_hash() -> Result<String, io::Error> {
    let repo = git::repo();
    repo.get_current_hash()
}

// Get built generation hash
pub fn get_built_hash() -> Result<String, io::Error> {
    let built_path = places::gens().join("built");
    
    match std::fs::read_to_string(&built_path) {
        Ok(content) => Ok(content.trim().to_string()),
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                return Err(io::Error::new(io::ErrorKind::NotFound, "No built generation found"));
            }
            Err(e)
        }
    }
}

// Create a new generation commit
pub fn commit(msg: &str) -> Result<String, io::Error> {
    

    let user_gen = match gen(ConfigSide::User) {
        Ok(o) => o,
        Err(e) => return Err(e),
    };

    let user_gen_string = match toml::to_string(&user_gen) {
        Ok(o) => o,
        Err(_e) => {
            error!("Failed to convert user generation to string!");
            return Err(custom_error("Failed to convert user generation to string!"));
        }
    };

    // Write generation to file
    let gen_path = places::gens().join("gen.toml");
    match std::fs::write(&gen_path, &user_gen_string) {
        Ok(_) => info!("Wrote generation file"),
        Err(e) => {
            error!("Failed to write generation file!");
            return Err(e);
        }
    };

    // Commit to Git
    let repo = git::repo();
    let hash = repo.commit(msg)?;
    
    if hash.is_empty() {
        warning!("No changes to commit");
        return Ok(String::new());
    }

    // Set as current
    set_current_hash(&hash, true)?;

    Ok(hash)
}

fn get_order(gen: &Generation) -> Result<Vec<String>, io::Error> {
    let return_order = {
        let path = places::base_user().join("manager_order.toml");

        if path.exists() {
            info!("Reading order rules from manager_order.toml...");

            let order_obj: ManagerOrder = match toml::from_str(&std::fs::read_to_string(&path)?) {
                Ok(o) => o,
                Err(e) => {
                    error!("Failed to deserialize manager_order.toml!");
                    error!("TOML Error: {e:#?}");

                    return Err(custom_error("Failed to deserialize manager_order.toml!"));
                }
            };

            let mut order: Vec<String> = order_obj.begin.clone();

            for k in gen.managers.keys() {
                if order_obj.begin.contains(k) || order_obj.end.contains(k) {
                    continue;
                }

                order.push(k.to_string());
            }

            order.extend(order_obj.end);

            let mut dup_track: HashMap<String, usize> = HashMap::new();

            for o in order.iter() {
                if dup_track.get(o) == None {
                    dup_track.insert(o.to_string(), 1);

                    continue;
                }

                *dup_track.get_mut(o).unwrap() += 1;
            }

            for (key, value) in dup_track.into_iter() {
                if value == 1 {
                    continue;
                }

                warning!("Duplicates in manager_order.toml! (Found {value} of: '{key}')");
            }

            order
                .into_iter()
                .filter(|x| gen.managers.contains_key(x))
                .collect()
        } else {
            gen.managers
                .keys()
                .into_iter()
                .map(|x| x.to_string())
                .collect()
        }
    };

    Ok(return_order)
}

// Apply differences between two generations
fn apply_diffs(built_gen: &Generation, curr_gen: &Generation) -> Result<(), io::Error> {
    let curr_order: Vec<String> = get_order(curr_gen)?;

    // Remove old items, add new items
    for i in curr_order.iter() {
        let man = load_manager(i)?;

        let curr_items = curr_gen.managers.get(i).unwrap();

        match built_gen.managers.get(i) {
            Some(built_items) => {
                let diffs = history(&built_items.items, &curr_items.items);

                let mut to_install: Vec<String> = Vec::new();
                let mut to_remove: Vec<String> = Vec::new();

                for j in diffs.iter() {
                    match j.mode {
                        HistoryMode::Add => to_install.push(j.line.to_string()),
                        HistoryMode::Remove => to_remove.push(j.line.to_string()),
                    };
                }

                man.remove(&to_remove)?;
                man.add(&to_install)?;
            }
            None => {
                man.add(&curr_items.items)?;
            }
        }
    }

    let built_order: Vec<String> = get_order(built_gen)?;

    // Remove items from managers that were removed from the generation
    for i in built_order.iter() {
        let built_items = built_gen.managers.get(i).unwrap();

        match curr_gen.managers.get(i) {
            Some(_) => (),
            None => {
                let man = load_manager(i)?;
                man.remove(&built_items.items)?;
            }
        };
    }

    Ok(())
}

// Apply full generation (for first-time builds)
fn apply_full(curr_gen: &Generation) -> Result<(), io::Error> {
    let curr_order = get_order(curr_gen)?;

    for i in curr_order.iter() {
        let curr_items = curr_gen.managers.get(i).unwrap();

        let man = load_manager(i)?;

        man.add(&curr_items.items)?;
    }

    Ok(())
}

// Build the current system generation
pub fn build() -> Result<(), io::Error> {
    

    hook::run("pre_build")?;

    let curr_gen = match gen(ConfigSide::System) {
        Ok(o) => o,
        Err(e) => return Err(e),
    };

    let current_hash = match get_current_hash() {
        Ok(hash) => hash,
        Err(_) => {
            error!("No current generation found!");
            return Err(custom_error("No current generation found!"));
        }
    };

    match get_built_hash() {
        Ok(built_hash) => {
            let built_gen = get_gen_from_hash(&built_hash)?;

            apply_diffs(&built_gen, &curr_gen)?;

            println!("");
            println!("");
            println!("");

            info!("#################");
            info!("#    SUMMARY    #");
            info!("#################");

            println!("");

            // TODO: Implement summary printing for Git-based system
            note!("Summary printing not yet implemented for Git-based system");

            println!("");
            println!("");
        }
        Err(_) => {
            apply_full(&curr_gen)?;
            note!("There is no summary. (First time building.)");
        }
    };

    // Set built hash
    set_built_hash(&current_hash, true)?;

    hook::run("post_build")?;

    Ok(())
}

// Rollback to a previous commit
pub fn rollback(by: isize, verbose: bool) -> Result<(), io::Error> {
    

    let repo = git::repo();
    let _current_hash = get_current_hash()?;
    
    let log = repo.log(None)?;
    
    if by >= log.len() as isize {
        error!("Cannot rollback that far!");
        return Err(custom_error("Rollback out of range!"));
    }
    
    let target_index = by as usize;
    if target_index >= log.len() {
        error!("Rollback target out of range!");
        return Err(custom_error("Rollback target out of range!"));
    }
    
    let target_hash = &log[target_index].0;
    
    repo.checkout(target_hash)?;
    set_current_hash(target_hash, verbose)?;
    
    Ok(())
}

// Set current to latest commit
pub fn latest(verbose: bool) -> Result<(), io::Error> {
    

    let repo = git::repo();
    let log = repo.log(Some(1))?;
    
    if log.is_empty() {
        error!("No generations found!");
        return Err(custom_error("No generations found!"));
    }
    
    let latest_hash = &log[0].0;
    repo.checkout(latest_hash)?;
    set_current_hash(latest_hash, verbose)?;
    
    Ok(())
}

// Set current generation hash
pub fn set_current_hash(hash: &str, verbose: bool) -> Result<(), io::Error> {
    let current_path = places::gens().join("current");
    
    match std::fs::write(&current_path, hash) {
        Ok(_) => {
            if verbose {
                info!("Set 'current' to: {}", hash);
            }
            Ok(())
        }
        Err(e) => {
            error!("Failed to create/write 'current' tracking file!");
            Err(e)
        }
    }
}

// Set built generation hash
pub fn set_built_hash(hash: &str, verbose: bool) -> Result<(), io::Error> {
    let built_path = places::gens().join("built");
    
    match std::fs::write(&built_path, hash) {
        Ok(_) => {
            if verbose {
                info!("Set 'built' to: {}", hash);
            }
            Ok(())
        }
        Err(e) => {
            error!("Failed to create/write 'built' tracking file!");
            Err(e)
        }
    }
}

// List all generations (Git commits)
pub fn list() -> Result<Vec<(String, String, bool, bool)>, io::Error> {
    let repo = git::repo();
    let commits = repo.log(None)?;
    
    let current_hash = match get_current_hash() {
        Ok(hash) => hash,
        Err(_) => String::new(),
    };
    
    let built_hash = match get_built_hash() {
        Ok(hash) => hash,
        Err(_) => String::new(),
    };
    
    let mut gens: Vec<(String, String, bool, bool)> = Vec::new();
    
    for (i, (hash, message)) in commits.iter().enumerate() {
        gens.push((
            format!("{}", i + 1), // Display as 1-based index
            message.clone(),
            hash == &current_hash,
            hash == &built_hash,
        ));
    }
    
    Ok(gens)
}

// Print out the list of generations
pub fn list_print() -> Result<(), io::Error> {
    let list_items = list()?;
    
    let mut max_digits: usize = 0;
    
    if list_items.len() > 0 {
        max_digits = list_items[list_items.len() - 1]
            .0
            .to_string()
            .trim()
            .len();
    }
    
    for i in list_items.iter() {
        let mut misc_text = String::new();
        
        if i.2 {
            misc_text.push_str(
                format!(
                    " {}{}{}",
                    "[".bright_black().bold(),
                    "CURRENT".bright_green().bold(),
                    "]".bright_black().bold()
                )
                .as_str(),
            );
        }
        
        if i.3 {
            misc_text.push_str(
                format!(
                    " {}{}{}",
                    "[".bright_black().bold(),
                    "BUILT".bright_yellow().bold(),
                    "]".bright_black().bold()
                )
                .as_str(),
            );
        }
        
        let mut tabbed = String::new();
        
        for _j in 0..(max_digits - i.0.trim().len()) {
            tabbed.push_str(" ");
        }
        
        generic!("{}{} ... ({}){}", tabbed, i.0, i.1, misc_text);
    }
    
    Ok(())
}

// Get generation hash from display number
pub fn get_hash_from_number(num: usize) -> Result<String, io::Error> {
    let repo = git::repo();
    let commits = repo.log(None)?;
    
    if num == 0 || num > commits.len() {
        error!("Generation number {} out of range!", num);
        return Err(custom_error("Generation number out of range!"));
    }
    
    Ok(commits[num - 1].0.clone()) // Convert to 0-based index
}

// Get the current generation TOML file path
pub fn current_gen() -> Result<PathBuf, io::Error> {
    let _current_hash = get_current_hash()?;
    let gen_path = places::gens().join("gen.toml");
    Ok(gen_path)
}

// Check if a generation has been built
pub fn been_built() -> bool {
    places::gens().join("built").exists()
}