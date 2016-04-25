extern crate meltdown;
extern crate url;

use url::Url;
use std::thread;
use std::process::Command;

use meltdown::*;


fn main() {
    let mut manager = DownloadManager::new();
    let download_url = Url::parse("http://releases.ubuntu.com/16.04/ubuntu-16.04-desktop-amd64.iso?_ga=1.88553821.715049987.1461163389")
                           .unwrap();
    manager.add_url(download_url.clone())
           .max_connection(8)
           .file(match download_url.path() {
               Some(path_vec) => &path_vec[path_vec.len() - 1],
               None => "python.tgz",
           })
           .finish();

    let _ = thread::spawn(move || {
                              match manager.start() {
                                  State::Completed(bytes) => {
                                      println!("Download complete of {} bytes", bytes);
                                  }
                                  _ => {}
                              }
                              let _ = Command::new("python")
                                               .arg("join.py")
                                               .arg("5")
                                               .output()
                                               .unwrap_or_else(|e| {
                                                   panic!("failed to execute process: {}", e)
                                               });
                          })
                              .join();
}
