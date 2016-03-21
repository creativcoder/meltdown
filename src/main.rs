extern crate hyper;
extern crate url;

use std::fs::OpenOptions;
use std::io::Read;
use std::io::SeekFrom;
use std::io::Seek;
use std::thread;
use std::fs::File;
use std::io::Write;
use hyper::Url as DownloadUrl;
use hyper::header::{Connection, AcceptRanges};
use hyper::header::{Headers, Range};
use hyper::Client;

#[derive(Debug)]
struct Downloader {
    url: Url,
    start: u64,
    end: u64,
    file_name: String,
    cursor:u64
}

impl Downloader {
    fn new(url: &str, start: u64, end: u64, file_name: &str,cursor:u64) -> Self {
        Downloader {
            url: url.to_owned(),
            start: start,
            end: end,
            file_name: file_name.to_owned(),
            cursor:cursor
        }
    }
    fn download(&self) {
        let mut headers = Headers::new();
        headers.set(Range::bytes(0, 100));
        let client = Client::new();
        let mut res = client.get(&self.url)
                            .header(Range::bytes(self.start, self.end))
                            .header(Connection::keep_alive())
                            .send()
                            .unwrap();
        let mut body: Vec<u8> = Vec::new();
        res.read_to_end(&mut body).unwrap();
        let mut file = DownloadManager::request_file(&self.file_name[..]);
        file.seek(SeekFrom::Start(self.cursor));
        file.write_all(body.as_slice());
    }
}


type Url = String;

enum State {
    Initial,
    Ready,
    Downloading,
    Completed,
    Paused,
    Stopped,
}

struct DownloadManager {
    task_queue: Vec<Downloader>,
    url: Url,
    max_connection: usize,
    file: Option<File>,
    state: State,
    block_size: usize,
    resume: bool,
}

impl DownloadManager {
    fn new() -> Self {
        DownloadManager {
            task_queue: vec![],
            url: "".to_owned(),
            max_connection: 0,
            file: None,
            state: State::Initial,
            block_size: 1024,
            resume: false,
        }
    }
    fn add_url(&mut self, url: &str) -> &mut DownloadManager {
        self.url = url.to_owned();
        self
    }
    fn max_connection(&mut self, max_con: usize) -> &mut DownloadManager {
        self.max_connection = max_con;
        self
    }
    fn file(&mut self, file_path: &str) -> &mut DownloadManager {
        self.file = Some(DownloadManager::request_file(file_path));
        self
    }
    fn block_size(&mut self, block_size: usize) -> &mut DownloadManager {
        self.block_size = block_size;
        self
    }
    fn finish(&mut self) {
        self.state = State::Ready;
    }
    fn start(&mut self) {
        let mut start_range = 0;
        let mut end_range = self.block_size as u64;
        for i in 0..self.max_connection {
            println!("Start Range {:?}", start_range);
            println!("End Range {:?}", end_range);
            println!("Downloading part {} of {}", i, self.max_connection);
            self.task_queue
                .push(Downloader::new(&self.url, start_range, end_range, "./readme.txt",start_range));
                
            self.task_queue[i].download();
            start_range = end_range + 1;
            end_range = (start_range - 1) * 2;
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
    fn check_resume(&self) -> bool {
        // TODO Add HEAD request to get Accept-Range Header
        false
    }
}

fn main() {
    let mut manager = DownloadManager::new();
    manager.add_url("https://wordpress.org/plugins/about/readme.txt")
           .max_connection(8)
           .file("readme.txt")
           .finish();
    let _download_thread = thread::spawn(move || {
                               manager.start();
                           })
                               .join();
}
