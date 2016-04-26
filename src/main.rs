extern crate meltdown;
extern crate url;

use std::fs;
use std::path::PathBuf;
use url::Url;
use std::thread;

use meltdown::*;
mod config;


fn main() {
    let _ = config::setup_config_directories();
    let mut manager = DownloadManager::new();
    let mut prefix = PathBuf::from("./temp");
    let mut file_name = String::new();
    let download_url = Url::parse("https://www.python.org/ftp/python/3.5.1/Python-3.5.1.tar.xz")
                           .unwrap();
    manager.add_url(download_url.clone())
           .max_connection(8)
           .file(match download_url.path() {
               Some(path_vec) => {file_name = path_vec[path_vec.len() - 1].clone();&path_vec[path_vec.len() - 1]},
               None => "python.tar.xz",
           })
           .finish();
    let _ = thread::spawn(move || {
                              match manager.start() {
                                  State::Completed(bytes) => {
                                      println!("Download complete of {} bytes", bytes);
                                  }
                                  _ => {}
                              }
                              prefix.push(&file_name);
                              join_part_files(prefix.to_str().unwrap(), "./temp");
                          }).join();
    
    
}
