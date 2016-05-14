extern crate meltdown;
extern crate url;

use url::Url;
use std::env;
use std::thread;
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Instant;

use meltdown::*;
mod config;


fn main() {
    let _ = config::setup_config_directories();
    let configurations = config::read_config();
    let url_string = env::args().skip(1).collect::<Vec<String>>();
    let submitted_tasks = url_string.len();
    print!("{:?}", url_string);
    if url_string.len() == 0 {
        println!("Welcome to meltdown download manager!");
        println!("Gimme a Url to download");
        return;
    }
    let (main_tx, main_rx) = channel();
    for i in url_string {
        let prefix = config::default_cache_dir().unwrap();
        let mut manager = DownloadManager::new();
        let download_url = Url::parse(&i).unwrap();
        let url_path_vec = download_url.path_segments().unwrap().collect::<Vec<&str>>();
        let file_name = url_path_vec[url_path_vec.len() - 1].to_owned();
        manager.add_url(download_url.clone())
               .max_connection(configurations.max_connection)
               .file(&file_name.clone())
               .finish();
        let complete_name = file_name.clone();
        let (tx, rx) = channel();
        let main_tx_clone = main_tx.clone();
        let _ = thread::spawn(move || {
            match manager.start(rx) {
                State::Completed(bytes) => {
                    let ext = Path::new(&complete_name);
                    let ext_str = ext.extension().unwrap().to_str();
                    join_part_files(&complete_name, prefix.to_str().unwrap(), ext_str.unwrap());
                    main_tx_clone.send(State::Completed(bytes));
                }
                _ => {}
            }
        });
    }
    let mut complete_tasks = 0;
    loop {
        match main_rx.try_recv() {
            Ok(state) => {
                match state {
                    State::Completed(bytes) => {
                        println!("Download Complete of {:?}", bytes);
                    }
                    _ => {}
                }
                complete_tasks += 1;
            }
            Err(_) => {}
        }
        if complete_tasks == submitted_tasks {
            println!("All tasks complete");
            return;
        }
    }
}
