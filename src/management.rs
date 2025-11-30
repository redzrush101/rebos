#![allow(dead_code)]

use colored::Colorize;
use std::path::{Path, PathBuf};
use piglog::prelude::*;
use piglog::*;
use serde::Deserialize;
use std::io;

use crate::config::ConfigSide;
use crate::generation::{gen, Items};
use crate::library::*;
use crate::obj_print_boilerplate::macros::print_entry;
use crate::{bool_question, places};

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields, default)]
pub struct ManagerConfig {
    #[serde(default = "default_many_args")]
    pub many_args: bool,
    #[serde(default = "default_arg_sep")]
    pub arg_sep: String,
}

fn default_many_args() -> bool { true }
fn default_arg_sep() -> String { String::from(" ") }

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Manager {
    pub add: String,
    pub remove: String,
    pub sync: Option<String>,
    pub upgrade: Option<String>,
    pub list: Option<String>,
    pub config: ManagerConfig,
    pub hook_name: String,
    pub plural_name: String,
}

impl Manager {
    fn join_args(&self, items: &[String]) -> String {
        items.join(&self.config.arg_sep)
    }

    pub fn add(&self, items: &[String]) -> Result<(), io::Error> {
        let many = self.config.many_args;

        crate::hook::run(&format!("pre_{}_add", self.hook_name))?;

        if many {
            self.add_raw(&self.join_args(items))?;
        } else {
            for i in items {
                self.add_raw(i)?;
            }
        }

        crate::hook::run(&format!("post_{}_add", self.hook_name))?;

        Ok(())
    }

    pub fn remove(&self, items: &[String]) -> Result<(), io::Error> {
        let many = self.config.many_args;

        crate::hook::run(&format!("pre_{}_remove", self.hook_name))?;

        if many {
            self.remove_raw(&self.join_args(items))?;
        } else {
            for i in items {
                self.remove_raw(i)?;
            }
        }

        crate::hook::run(&format!("post_{}_remove", self.hook_name))?;

        Ok(())
    }

    fn add_raw(&self, items: &str) -> Result<(), io::Error> {
        if items.trim() == "" {
            return Ok(());
        }

        match run_command(self.add.as_str().replace("#:?", items).as_str()) {
            true => info!("Successfully added {}!", self.plural_name),
            false => {
                error!("Failed to add {}!", self.plural_name);

                return Err(custom_error(
                    format!("Failed to add {}!", self.plural_name).as_str(),
                ));
            }
        };

        Ok(())
    }

    fn remove_raw(&self, items: &str) -> Result<(), io::Error> {
        if items.trim() == "" {
            return Ok(());
        }

        match run_command(self.remove.as_str().replace("#:?", items).as_str()) {
            true => info!("Successfully removed {}!", self.plural_name),
            false => {
                error!("Failed to remove {}!", self.plural_name);

                return Err(custom_error(
                    format!("Failed to remove {}!", self.plural_name).as_str(),
                ));
            }
        };

        Ok(())
    }

    pub fn sync(&self) -> Result<(), io::Error> {
        crate::hook::run(&format!("pre_{}_sync", self.hook_name))?;

        if let Some(ref s) = self.sync {
            match run_command(s) {
                true => info!("Synced manager successfully! ('{}')", self.plural_name),
                false => {
                    error!("Failed to sync manager! ('{}')", self.plural_name);

                    return Err(custom_error("Failed to sync repositories!"));
                }
            };
        }

        crate::hook::run(&format!("post_{}_sync", self.hook_name))?;

        Ok(())
    }

    pub fn upgrade(&self) -> Result<(), io::Error> {
        crate::hook::run(&format!("pre_{}_upgrade", self.hook_name))?;

        if let Some(ref s) = self.upgrade {
            match run_command(s) {
                true => info!("Successfully upgraded {}!", self.plural_name),
                false => {
                    error!("Failed to upgrade {}!", self.plural_name);

                    return Err(custom_error(
                        format!("Failed to upgrade {}!", self.plural_name).as_str(),
                    ));
                }
            };
        }

        crate::hook::run(&format!("post_{}_upgrade", self.hook_name))?;

        Ok(())
    }

    pub fn get_other(&self, items: &[String]) -> Result<Vec<String>, io::Error> {
        if self.list.is_some() {
            let mut others = self.list()?;
            others.retain(|other| !items.contains(other));
            Ok(others)
        } else {
            Ok(Vec::new())
        }
    }

    pub fn list(&self) -> Result<Vec<String>, io::Error> {
        let list_cmd = self.list.as_ref().expect("Command should exist");

        match run_command_with_output(list_cmd) {
            Some(output) => Ok(output.split_whitespace().map(|s| s.to_owned()).collect()),
            None => {
                let error = format!("Failed to get list of {}!", self.plural_name);

                error!("{error}");

                Err(custom_error(&error))
            }
        }
    }

    pub fn set_plural_name(&mut self, pn: &str) {
        self.plural_name = pn.to_string();
    }

    pub fn check_config(&self) -> Result<(), Vec<String>> {
        let mut errors: Vec<String> = Vec::new();

        let valid_hook_name = self.hook_name.chars()
            .map(|c| match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c,
                _ => '_',
            })
            .collect::<String>();

        if self.hook_name != valid_hook_name {
            errors.push(format!(
                "Field 'hook_name' must be filename safe! (Fixed version: {})",
                valid_hook_name
            ));
        }

        if errors.len() > 0 {
            return Err(errors);
        }

        Ok(())
    }
}

pub fn load_manager_no_config_check(man: &str) -> Result<Manager, io::Error> {
    let path = places::base_user().join(format!("managers/{man}.toml"));

    let man_string = match std::fs::read_to_string(&path) {
        Ok(o) => o,
        Err(e) => {
            piglog::fatal!("Failed to read manager file! ({man})");
            piglog::note!(
                "If this error shows up, it is possible the file is missing. ({})",
                path.display()
            );

            return Err(e);
        }
    };

    let manager: Manager = match toml::from_str(&man_string) {
        Ok(o) => o,
        Err(e) => {
            piglog::fatal!("Failed to deserialize manager! ({man})");
            piglog::fatal!("Error: {e:#?}");

            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to deserialize manager!",
            ));
        }
    };

    Ok(manager)
}

pub fn load_manager(man: &str) -> Result<Manager, io::Error> {
    let manager = load_manager_no_config_check(man)?;

    match manager.check_config() {
        Ok(_) => (),
        Err(e) => {
            piglog::fatal!("Manager '{man}' is not configured properly! Errors:");

            for (i, error) in e.into_iter().enumerate() {
                eprintln!(
                    "{}{} {}",
                    i.to_string().bright_red().bold(),
                    ":".bright_black().bold(),
                    error
                );
            }

            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed manager configuration check!",
            ));
        }
    };

    Ok(manager)
}

pub fn get_managers() -> Result<Vec<String>, io::Error> {
    let path = places::base_user().join("managers");

    let man_list: Vec<String> = std::fs::read_dir(&path)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().ok().map(|ft| ft.is_file()).unwrap_or(false))
        .filter_map(|entry| {
            entry.file_name()
                .to_str()
                .map(|s| s.to_string())
                .filter(|name| name.ends_with(".toml"))
                .map(|name| name.replace(".toml", ""))
        })
        .collect();

    Ok(man_list)
}

pub fn sync_managers(managers: &Option<Vec<String>>) -> Result<(), io::Error> {
    crate::library::for_each_manager(managers, |man_name| {
        info!("Syncing manager {man_name}");
        let manager = load_manager(man_name)?;
        manager.sync()
    })?;
    success!("All managers synced successfully");

    Ok(())
}

pub fn upgrade_managers(
    sync_before_upgrade: bool,
    managers: &Option<Vec<String>>,
) -> Result<(), io::Error> {
    if sync_before_upgrade {
        sync_managers(managers)?;
    }

    crate::library::for_each_manager(managers, |man_name| {
        info!("Upgrading manager {man_name}");
        let manager = load_manager(man_name)?;
        manager.upgrade()
    })?;

    success!("All managers upgraded successfully");

    Ok(())
}

pub fn list_others(managers: &Option<Vec<String>>, remove: bool) -> Result<(), io::Error> {
    let curr_gen = gen(ConfigSide::System)?;

    info!("Installed but not specified items");
    match managers {
        Some(man_names) => {
            for man_name in man_names {
                let items = curr_gen
                    .managers
                    .get(man_name)
                    .ok_or(custom_error("Failed to get manager {man_name}!"))?;

                list_others_core(man_name, items, remove)?;
            }
        }
        None => {
            for (man_name, items) in curr_gen.managers.iter() {
                list_others_core(man_name, items, remove)?;
            }
        }
    };

    Ok(())
}

fn list_others_core(man_name: &String, items: &Items, remove: bool) -> Result<(), io::Error> {
    let man = load_manager(man_name)?;

    let others = man.get_other(&items.items)?;

    if !others.is_empty() {
        print_entry!(man_name, others);

        if remove && bool_question("Remove items?", false) {
            man.remove(&others)?;
        }
    };
    Ok(())
}
