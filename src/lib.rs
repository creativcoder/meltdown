
extern crate hyper;
extern crate url;
extern crate crossbeam;
extern crate walkdir;


use std::fs::OpenOptions;
use std::io::Read;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use hyper::Url as DownloadUrl;
use hyper::header::{Connection, AcceptRanges};
use hyper::header::{ByteRangeSpec, Range, ContentLength};
use hyper::Client;
use url::Url;
use walkdir::WalkDir;

mod config;

#[derive(Debug, Copy, Clone)]
pub enum State {
    Initial,
    Ready,
    Downloading,
    Completed(u64),
    Paused,
    Stopped,
}

#[derive(Debug)]
pub enum Msg {
    Start,
    Stop,
    Pause,
    Resume,
}

#[derive(Debug)]
struct Downloader {
    id: usize,
    url: String,
    start: u64,
    end: u64,
    file_name: String,
    cursor: u64,
    content_length: u64,
}

pub enum ReadResult {
    Payload(Vec<u8>, usize),
    EOF,
}

fn read_block<R: Read>(reader: &mut R) -> Result<ReadResult, ()> {
    let mut buf = vec![0;1024];
    match reader.read(&mut buf) {
        Ok(len) if len > 0 => {
            buf.truncate(len);
            Ok(ReadResult::Payload(buf, len))
        }
        Ok(_) => Ok(ReadResult::EOF),
        Err(_) => Err(()),
    }
}

impl Downloader {
    fn new(id: usize,
           url: &str,
           start: u64,
           end: u64,
           file_name: &str,
           cursor: u64,
           content_length: u64)
           -> Self {
        Downloader {
            id: id,
            url: url.to_owned(),
            start: start,
            end: end,
            file_name: file_name.to_owned(),
            cursor: cursor,
            content_length: content_length,
        }
    }
    fn download(&self, sender: Sender<String>) {

        let client = Client::new();
        let mut res = client.get(&self.url)
                            .header(if self.end == 0 {
                                Range::Bytes(vec![ByteRangeSpec::AllFrom(self.start)])
                            } else {
                                Range::bytes(self.start, self.end)
                            })
                            .header(Connection::keep_alive())
                            .send()
                            .unwrap();
        let file_name = format!("{}{}", self.file_name, self.id);
        let mut file = DownloadManager::request_file(&file_name[..]);
        let mut complete_len = 0u64;
        loop {
            match read_block(&mut res) {
                Ok(ReadResult::Payload(bytes, len)) => {
                    print!("{:?} has downloaded {:?} bytes\r", self.id, complete_len);
                    complete_len += len as u64;
                    let _ = file.write(bytes.as_slice());
                }
                Ok(ReadResult::EOF) => {
                    break;
                }
                Err(_) => break,
            }
        }
        println!("\nWorker {} finished", self.id);
        sender.send(file_name).unwrap();
    }
}

#[derive(Debug)]
pub struct DownloadManager {
    task_queue: Vec<Downloader>,
    url: Option<Url>,
    max_connection: usize,
    file_name: Option<PathBuf>,
    state: State,
    block_size: usize,
    resume: bool,
    complete_queue: Vec<String>,
}

impl DownloadManager {
    pub fn new() -> Self {
        DownloadManager {
            task_queue: vec![],
            url: None,
            max_connection: 0,
            file_name: None,
            state: State::Initial,
            block_size: 1024,
            resume: false,
            complete_queue: vec![],
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
    pub fn file(&mut self, file_name: &str) -> &mut DownloadManager {
        self.file_name = Some(PathBuf::from(file_name));
        self
    }

    pub fn finish(&mut self) {
        self.state = State::Ready;
    }

    pub fn start_as_unit(&self) -> State {
        let client = Client::new();
        let mut url_str = "";
        let mut complete_len = 0u64;
        if let Some(ref url) = self.url {
            url_str = url.as_str().clone();
            println!("Connecting to {}", url_str);
        };

        let mut res = client.get(url_str)
                            .header(Connection::keep_alive())
                            .send()
                            .unwrap();
        println!("Connecting established");
        if let Some(ref file_name) = self.file_name {
            let extension = file_name.extension();
            let download_directory = config::map_ext_location(extension.unwrap().to_str().unwrap());
            let full_path = download_directory.join(file_name);
            let _ = fs::create_dir_all(download_directory);
            let mut file = OpenOptions::new()
                               .read(true)
                               .write(true)
                               .create(true)
                               .append(false)
                               .open(full_path.clone())
                               .unwrap();
            loop {
                match read_block(&mut res) {
                    Ok(ReadResult::Payload(bytes, len)) => {
                        complete_len += len as u64;
                        print!("Single Thread downloading: {:?} bytes\r", complete_len);
                        let _ = file.write(bytes.as_slice());
                    }
                    Ok(ReadResult::EOF) => {
                        break;
                    }
                    Err(_) => break,
                }
            }
        }
        State::Completed(complete_len)
    }

    pub fn start_as_part(&mut self, content_length: u64, sender: Sender<String>) {
        let mut start_range: u64 = 0;
        let mut end_range: u64 = (content_length / self.max_connection as u64) - 1;
        let mut parts_suffix = 0;
        let mut cache_dir = config::default_cache_dir().unwrap();
        cache_dir.push(self.file_name.clone().unwrap().clone());
        let file_path = cache_dir.to_str().unwrap();
        let url_str = self.url.clone().unwrap().to_string();
        while !(end_range > content_length) {
            let worker = Downloader::new(parts_suffix,
                                         &url_str,
                                         start_range,
                                         end_range,
                                         &file_path,
                                         start_range,
                                         content_length);
            self.task_queue.push(worker);
            println!("Worker {}: Byte Range {}, {}",
                     parts_suffix,
                     start_range,
                     end_range);
            start_range = end_range + 1;
            end_range = ((start_range - 1) * 2) + 1;
            parts_suffix += 1;
        }

        let remaining_bytes = content_length - start_range;
        if remaining_bytes != 0 {
            let last_range = (start_range + remaining_bytes) - 1;
            let last_worker = Downloader::new(parts_suffix,
                                              &url_str,
                                              start_range,
                                              last_range,
                                              &file_path,
                                              start_range,
                                              content_length);
            self.task_queue.push(last_worker);
        }

        crossbeam::scope(|scope| {
            for i in &self.task_queue {
                let tx = sender.clone();
                scope.spawn(move || {
                    i.download(tx);
                });
            }
        });

    }

    pub fn start(&mut self, msg_port: Receiver<Msg>) -> State {
        // to check receive of events from main thread
        thread::spawn(move || {
            loop {
                match msg_port.try_recv() {
                    Ok(msg) => {
                        match msg {
                            Msg::Pause => {
                                panic!("Download was interrupted from a Pause Msg from main thread")
                            }
                            _ => {}
                        }
                    }
                    Err(_) => {}
                }
            }
        });

        let mut content_length: u64 = 0;
        let (tx, rx) = channel();
        match self.check_resume() {
            (true, len) => {
                println!("Connection supports Resume");
                let ContentLength(length) = len;
                content_length = length;
                self.start_as_part(content_length, tx);
                State::Completed(content_length)
            }
            (false, _) => {
                println!("Connection does not support Resume");
                self.start_as_unit()
            }
        }
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

pub fn join_part_files(file_name: &str, file_path: &str, extension: &str) {
    let download_directory = config::map_ext_location(extension);
    let full_path = download_directory.join(file_name);
    let _ = fs::create_dir_all(download_directory);
    let mut completed = OpenOptions::new()
                            .read(true)
                            .write(true)
                            .create(true)
                            .append(true)
                            .open(full_path.clone())
                            .unwrap();
    let mut buffer: Vec<u8> = Vec::new();
    let mut fd_vec: Vec<String> = Vec::new();
    for entry in WalkDir::new(file_path) {
        let entry = entry.unwrap();
        if !entry.path().is_dir() {
            let path = entry.path().display().to_string();
            fd_vec.push(path.clone());
        }
    }
    // sort by part file ids
    fd_vec.sort_by(|a, b| {
        let fst = a.chars().rev().take(1).collect::<Vec<char>>()[0].to_digit(10).unwrap() as usize;
        let snd = b.chars().rev().take(1).collect::<Vec<char>>()[0].to_digit(10).unwrap() as usize;
        fst.cmp(&snd)
    });
    for i in fd_vec {
        let mut part_fd = File::open(i).unwrap();
        let _ = part_fd.read_to_end(&mut buffer);
        let _ = completed.write_all(&buffer);
        buffer.clear();
    }
    println!("File downloaded at {:?}",
             full_path.to_str().unwrap().replace("creativcoder", "autobot"));
    // let _ = fs::remove_dir_all(file_path);
}

#[test]
fn test_combine_part_files() {
    fs::create_dir_all("./temp");
    let mut f1 = File::create("./temp/part1.txt").unwrap();
    let part1 = b"meltdown\n";
    f1.write_all(part1);
    let mut f1 = File::create("./temp/part2.txt").unwrap();
    let part1 = b"a download manager\n";
    f1.write_all(part1);
    let mut f1 = File::create("./temp/part3.txt").unwrap();
    let part1 = b"written in Rust\n";
    f1.write_all(part1);
    join_part_files("./temp/joined.txt", "./temp");
    let mut complete_fd = File::open("./temp/joined.txt").unwrap();
    let mut complete_buff: Vec<u8> = Vec::new();
    complete_fd.read_to_end(&mut complete_buff);
    assert_eq!("meltdown\na download manager\nwritten in Rust\n".len(),
               complete_buff.len());
    fs::remove_dir_all("./temp");
}

#[test]
fn test_download_sublime_deb_package() {
    use std::thread;
    let mut manager = DownloadManager::new();
    let download_url = Url::parse("https://download.sublimetext.\
                                   com/sublime-text_build-3103_amd64.deb")
                           .unwrap();
    manager.add_url(download_url.clone())
           .max_connection(4)
           .file(match download_url.path() {
               Some(path_vec) => &path_vec[path_vec.len() - 1],
               None => "subl.deb",
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
            })
                .join();
}
