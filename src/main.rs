extern crate hyper;
extern crate url;

use std::fs::OpenOptions;
use std::io::Read;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::io::SeekFrom;
use std::io::Seek;
use std::thread;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use hyper::client::RequestBuilder;
use hyper::Url as DownloadUrl;
use std::collections::HashMap;
use hyper::header::{Connection, AcceptRanges};
use hyper::header::{ByteRangeSpec, Range, ContentLength};
use hyper::Client;
use url::Url;

#[derive(Debug)]
pub enum State {
    Initial,
    Ready,
    Downloading,
    Completed(u64),
    Paused,
    Stopped,
}

#[derive(Debug)]
struct Downloader {
    id: usize,
    url: String,
    start: u64,
    end: u64,
    file_name: String,
    cursor: u64,
}


impl Downloader {
    fn new(id: usize, url: &str, start: u64, end: u64, file_name: &str, cursor: u64) -> Self {
        Downloader {
            id: id,
            url: url.to_owned(),
            start: start,
            end: end,
            file_name: file_name.to_owned(),
            cursor: cursor,
        }
    }
    fn download(&self, sender:Sender<String>) {
        let client = Client::new();
        println!("Worker {} Downloading", self.id);
        let mut res = client.get(&self.url)
                            .header(if self.end == 0 {
                                Range::Bytes(vec![ByteRangeSpec::AllFrom(self.start)])
                            } else {
                                Range::bytes(self.start, self.end)
                            })
                            .header(Connection::keep_alive())
                            .send()
                            .unwrap();
        let mut body: Vec<u8> = Vec::new();
        res.read_to_end(&mut body).unwrap();
        // Id incremented file parts
        let file_name = format!("{}{}", self.file_name, self.id);
        let mut file = DownloadManager::request_file(&file_name[..]);
        file.write_all(body.as_slice());
        sender.send(file_name).unwrap();
    }
}

#[derive(Debug)]
pub struct DownloadManager {
    task_queue: Vec<Downloader>,
    url: Option<Url>,
    max_connection: usize,
    file_path: Option<PathBuf>,
    state: State,
    block_size: usize,
    resume: bool,
    complete_queue: Vec<String>
}

impl DownloadManager {
    pub fn new() -> Self {
        DownloadManager {
            task_queue: vec![],
            url: None,
            max_connection: 0,
            file_path: None,
            state: State::Initial,
            block_size: 1024,
            resume: false,
            complete_queue: vec![]
        }
    }
    pub fn add_url(&mut self, url: Url) -> &mut DownloadManager {
        self.url = Some(url);
        self
    }
    pub fn max_connection(&mut self, max_con: usize) -> &mut DownloadManager {
        self.max_connection = max_con;
        self
    }
    pub fn file(&mut self, file_path: &str) -> &mut DownloadManager {
        self.file_path = Some(PathBuf::from(file_path));
        self
    }

    pub fn finish(&mut self) {
        self.state = State::Ready;
    }
    pub fn start(&mut self) -> State {

        let mut content_length: u64 = 0;
        let (tx, rx) = channel();

        match self.check_resume() {
            (true, len) => {
                let ContentLength(length) = len;
                content_length = length;
            }
            (false, _) => println!("Download does not support resume"),
        }

        let mut start_range: u64 = 0;
        let mut end_range: u64 = (content_length / self.max_connection as u64) - 1;
        let mut parts_suffix = 0;
        let file_path = "./".to_owned() + self.file_path.clone().unwrap().to_str().unwrap();
        let url_str = self.url.clone().unwrap().to_string();
        while !(end_range > content_length) {
            let worker = Downloader::new(parts_suffix,&url_str,start_range,end_range,&file_path,start_range);
            self.task_queue.push(worker);
            start_range = end_range + 1;
            end_range = ((start_range - 1) * 2)+1;
            parts_suffix+=1;
            }

            let mut remaining_bytes = content_length - start_range;
            if remaining_bytes != 0 {
                let last_range = (start_range+remaining_bytes)-1;
                let last_worker = Downloader::new(parts_suffix, &url_str, start_range, last_range, &file_path, start_range);
                self.task_queue.push(last_worker);
            }

            for i in 0..self.task_queue.len() {
                self.task_queue[i].download(tx.clone());
                self.complete_queue.push(rx.recv().unwrap());
            }
        State::Completed(content_length)
    }

    fn join(&self,file_path:&String) {
        
        let final_download = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .append(true)
            .open(&file_path).unwrap();

        let joiner = Command::new("python")
                                 .arg("join.py")
                                 .arg(&self.complete_queue.len().to_string())
                                 .output()
                                 .unwrap_or_else(|e| { panic!("failed to execute process: {}", e) });
        for i in self.complete_queue.iter().map(|ref i|{
                println!("{:?}",i );
                fs::remove_file(i);
            }){};

    }

    fn check_resume(&self) -> (bool, ContentLength) {
        let client = Client::new();
        let url_str = self.url.clone().unwrap().to_string();
        let head_req = client.head(DownloadUrl::parse(&url_str).unwrap());
        match head_req.send() {
            Ok(res) => {
                (res.headers.has::<AcceptRanges>(),
                 *res.headers.get::<ContentLength>().unwrap())
            }
            Err(_) => (false, ContentLength(0)),
        }
    }

    fn request_file(path: &str) -> File {
        let file = OpenOptions::new()
                       .read(true)
                       .write(true)
                       .create(true)
                       .open(path);
        if let Ok(file) = file {
            file
        } else {
            panic!("File open error");
        }
    }
}

fn main() {

}

#[test]
fn test_download_sublime_deb_package() {
    let mut manager = DownloadManager::new();
    let download_url = Url::parse("https://download.sublimetext.com/sublime-text_build-3103_amd64.deb").unwrap();
    manager.add_url(download_url.clone())
           .max_connection(4)
           .file(
            match download_url.path() {
                Some(path_vec) => &path_vec[path_vec.len()-1],
                None => "file.txt"
                }
            )
           .finish();

    let download_thread = thread::spawn(move || { 
        match manager.start() {
            State::Completed(bytes) => { println!("Download complete of {} bytes",bytes ); },
            _ => {}
        }
        let joiner = Command::new("python")
                                 .arg("join.py")
                                 .arg("6")
                                 .output()
                                 .unwrap_or_else(|e| { panic!("failed to execute process: {}", e) });
    }).join();
}