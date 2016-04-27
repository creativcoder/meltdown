#![allow(dead_code)]

extern crate xdg;

use std::path::PathBuf;
use std::fs::File;
use std::io::Read;

pub struct Configuration {
    pub max_connection: usize,
}

impl Configuration {
    fn new() -> Self {
        Configuration { max_connection: 0 }
    }
}

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "windows")))]
pub fn setup_config_directories() -> Result<(), ()> {
    let xdg_dirs = xdg::BaseDirectories::with_profile("meltdown", "default").unwrap();
    let config_dir = default_config_dir().unwrap();
    let data_dir = default_data_dir().unwrap();
    let cache_dir = default_cache_dir().unwrap();
    xdg_dirs.create_config_directory(config_dir).unwrap();
    xdg_dirs.create_data_directory(data_dir).unwrap();
    xdg_dirs.create_cache_directory(cache_dir).unwrap();
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "windows")))]
pub fn default_config_dir() -> Option<PathBuf> {
    let xdg_dirs = xdg::BaseDirectories::with_profile("meltdown", "default").unwrap();
    let config_dir = xdg_dirs.get_config_home();
    Some(config_dir)
}

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "windows")))]
pub fn default_data_dir() -> Option<PathBuf> {
    let xdg_dirs = xdg::BaseDirectories::with_profile("meltdown", "default").unwrap();
    let data_dir = xdg_dirs.get_data_home();
    Some(data_dir)
}

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "windows")))]
pub fn default_cache_dir() -> Option<PathBuf> {
    let xdg_dirs = xdg::BaseDirectories::with_profile("meltdown", "default").unwrap();
    let cache_dir = xdg_dirs.get_cache_home();
    Some(cache_dir)
}

#[cfg(target_os = "macos")]
pub fn bootstrap_default_dirs() -> Result<(), IOError> {
    let config_dir = default_config_dir().unwrap();
    match fs::create_dir_all(config_dir) {
        Ok(_) => Ok(()),
        Err(why) => Err(why),
    }
}

#[cfg(target_os = "macos")]
pub fn default_config_dir() -> Option<PathBuf> {
    let mut config_dir = env::home_dir().unwrap();
    config_dir.push("Library");
    config_dir.push("Application Support");
    config_dir.push("Meltdown");
    Some(config_dir)
}

#[cfg(target_os = "windows")]
pub fn bootstrap_default_dirs() -> Result<(), IOError> {
    let config_dir = default_config_dir().unwrap();
    match fs::create_dir_all(config_dir) {
        Ok(_) => Ok(()),
        Err(why) => Err(why),
    }
}

#[cfg(target_os = "windows")]
pub fn default_config_dir() -> Option<PathBuf> {
    let mut config_dir = match env::var("APPDATA") {
        Ok(appdata_path) => PathBuf::from(appdata_path),
        Err(_) => {
            let mut dir = env::home_dir().unwrap();
            dir.push("Appdata");
            dir.push("Roaming");
            dir
        }
    };
    config_dir.push("Meltdown");
    Some(config_dir)
}

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "windows")))]
pub fn read_config() -> Configuration {
    let mut configuration = Configuration::new();
    let mut buff = String::new();
    let mut path_buff = default_config_dir().unwrap();
    path_buff.push("meltdown.config");
    let mut config_file = File::open(path_buff).unwrap();
    let _ = config_file.read_to_string(&mut buff);
    let entry = buff.split(" ").collect::<Vec<&str>>();
    configuration.max_connection = entry[1].parse::<usize>().unwrap();
    configuration
}
