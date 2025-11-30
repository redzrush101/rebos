#![allow(dead_code)]

use std::io;
use std::path::{Path, PathBuf};
use std::env;
use piglog::prelude::*;
use piglog::*;

pub fn setup() -> Result<(), io::Error> {
    let directories = vec![
        base(),
        gens(),
    ];

    crate::library::ensure_directories_exist(&directories)?;

    Ok(())
}



pub fn base_legacy() -> PathBuf {
    env::var("HOME")
        .map(|home| PathBuf::from(home).join(".rebos-base"))
        .unwrap_or_else(|_| PathBuf::from("/tmp/.rebos-base"))
}

pub fn base() -> PathBuf {
    env::var("XDG_STATE_HOME")
        .map(|state| PathBuf::from(state).join("rebos"))
        .unwrap_or_else(|_| {
            let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".local/state").join("rebos")
        })
}

pub fn gens() -> PathBuf {
    base().join("generations")
}

pub fn base_user() -> PathBuf {
    env::var("XDG_CONFIG_HOME")
        .map(|config| PathBuf::from(config).join("rebos"))
        .unwrap_or_else(|_| {
            let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".config").join("rebos")
        })
}
