extern crate meltdown;
extern crate url;

use url::Url;
use std::thread;
use std::process::Command;

use meltdown::*;


fn main() {
    let mut manager = DownloadManager::new();
    let download_url = Url::parse("https://www.python.org/ftp/python/3.5.1/Python-3.5.1.tgz")
                           .unwrap();
    manager.add_url(download_url.clone())
           .max_connection(8)
           .file(match download_url.path() {
               Some(path_vec) => &path_vec[path_vec.len() - 1],
               None => "python.tgz",
           })
           .finish();

    let download_thread = thread::spawn(move || {
                              match manager.start() {
                                  State::Completed(bytes) => {
                                      println!("Download complete of {} bytes", bytes);
                                  }
                                  _ => {}
                              }
                              let joiner = Command::new("python")
                                               .arg("join.py")
                                               .arg("5")
                                               .output()
                                               .unwrap_or_else(|e| {
                                                   panic!("failed to execute process: {}", e)
                                               });
                          })
                              .join();
}
