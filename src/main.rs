extern crate meltdown;
extern crate url;

use url::Url;
use std::env;
use std::thread;
use std::path::Path;

use meltdown::*;
mod config;


fn main() {
    let _ = config::setup_config_directories();
    let configurations = config::read_config();
    let mut manager = DownloadManager::new();
    let prefix = config::default_cache_dir().unwrap();
    let url_string = env::args().skip(1).collect::<Vec<String>>();
    if url_string.len() == 0 {
        println!("Welcome to meltdown download manager!");
        println!("Gimme a Url to download");
        return ();
    }
    let download_url = Url::parse(&url_string[0]).unwrap();
    let url_path_vec = download_url.path_segments().unwrap().collect::<Vec<&str>>();
    let file_name = url_path_vec[url_path_vec.len() - 1].to_owned();
    manager.add_url(download_url.clone())
           .max_connection(configurations.max_connection)
           .file(&file_name.clone())
           .finish();
    let complete_name = file_name.clone();
    let _ = thread::spawn(move || {
                match manager.start() {
                    State::Completed(bytes) => {
                        println!("Download complete of {} bytes", bytes);
                    }
                    _ => {}
                }
                let ext = Path::new(&complete_name);
                let ext_str = ext.extension().unwrap().to_str();
                join_part_files(&complete_name, prefix.to_str().unwrap(), ext_str.unwrap());
            })
                .join();
}
