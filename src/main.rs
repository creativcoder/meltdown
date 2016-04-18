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
use hyper::header::{ByteRangeSpec, Range, ContentLength};
use hyper::Client;

#[derive(Debug)]
struct Downloader {
    id: usize,
    url: Url,
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
    fn download(&self) {
        
        let client = Client::new();
        let mut res = client.get(&self.url)
                            .header(if self.end == 0 {
                                Range::Bytes(vec![ByteRangeSpec::Last(self.end)])
                            } else {
                                Range::bytes(self.start, self.end)
                            })
                            .header(Connection::keep_alive())
                            .send()
                            .unwrap();
        let mut body: Vec<u8> = Vec::new();
        res.read_to_end(&mut body).unwrap();
        // Id incremented file parts
        let mut file = DownloadManager::request_file(&format!("{}{}", self.file_name, self.id)[..]);
        //file.seek(SeekFrom::Start(self.cursor));
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
        let mut content_length: u64 = 0;
        match self.check_resume() {
            (true, len) => {
                let ContentLength(length) = len;
                content_length = length;
            }
            (false, _) => println!("Download does not support resume"),
        }
        let mut start_range: u64 = 0;
        let mut end_range: u64 = (content_length / self.max_connection as u64) - 1;
        println!("Total size of file is {}",content_length);
        println!("Block size to each connection: {}",end_range);
        let mut i = 0;
        while !(end_range > content_length) {
            println!("Starting download from {} to {}",start_range,end_range);
            Downloader::new(i,&self.url,start_range,end_range,"./big_buck_bunny.mp4",start_range).download();
            start_range = end_range + 1;
            end_range = ((start_range - 1) * 2)+1;
            i+=1;  
            }
            let mut remaining_bytes = content_length - start_range;
            if remaining_bytes != 0 {
            Downloader::new(i,&self.url,start_range,0,"./big_buck_bunny.mp4",start_range).download();
            println!("Download remaining bytes of {}",remaining_bytes);
}
    }
    fn check_resume(&self) -> (bool, ContentLength) {
        let client = Client::new();
        let head_req = client.head(DownloadUrl::parse(&self.url).unwrap());
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
    let mut manager = DownloadManager::new();
    manager.add_url("http://www.sample-videos.com/video/mp4/360/big_buck_bunny_360p_50mb.mp4")
           .max_connection(4)
           .file("big_buck_bunny.mp4")
           .finish();
    let download_thread = thread::spawn(move || {
                               manager.start();
                           })
                               .join();
}
