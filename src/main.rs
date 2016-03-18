extern crate hyper;

use std::fs::OpenOptions;
use std::io::Read;
use std::thread;
use std::fs::File;
use std::io::Write;
use hyper::header::Connection;
use hyper::header::{Headers, Range};
use hyper::Client;

#[derive(Debug)]
struct Downloader {
	url:Url,
    start:u64,
    end:u64
}

impl Downloader {
    fn new(url:&str,start:u64,end:u64) -> Self {
    	Downloader { url: url.to_owned(),
    	start:start,
    	end:end }
    }
    fn download(&self) {
    	let mut headers = Headers::new();
    	headers.set(Range::bytes(0, 100));
    	let client = Client::new();
    	let mut res = client.get(&self.url)
        .header(Range::bytes(self.start, self.end))
        .header(Connection::keep_alive())
        .send().unwrap();
        let mut body = String::new();
    	res.read_to_string(&mut body).unwrap();
    	println!("Response: {}", body);
    	let mut file = DownloadManager::request_file("./index.html");
    	file.write_all(body.as_bytes());
    	println!("Chunk Downloaded");
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
	url:Url,
	max_connection:usize,
	file:Option<File>,
	state:State
}

impl DownloadManager {
    fn new() -> Self {
    	DownloadManager { task_queue:vec![],url:"".to_owned(),max_connection:0,file:None,state:State::Initial}
    }
    fn add_url(&mut self,url:&str) -> &mut DownloadManager {
    	self.url = url.to_owned();
    	self
    }
    fn max_connection(&mut self,max_con:usize) -> &mut DownloadManager {
    	self.max_connection = max_con;
    	self
    }
    fn file(&mut self,file_path:&str) -> &mut DownloadManager {
    	self.file = Some(DownloadManager::request_file(file_path));
    	self
    }
    fn finish(&mut self) {
    	self.state = State::Ready;
    }
    fn start(&mut self) {
    	let mut start_range = 0;
    	let mut end_range = 1024;
    	for i in 0..self.max_connection {
    		self.task_queue.push(Downloader::new(&self.url,start_range,end_range));
    		self.task_queue[i].download();
    		start_range = end_range + 1;
    		end_range = start_range - 1 * 2;
    	}
    }
	fn request_file(path:&str) -> File {
	let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path);
    if let Ok(file) = file {
    	return file;
    } else {
    	panic!("File open error");

    }
}     
}

/*fn handler(req: Request, res: Response) {
    let mut file_list = String::new();
    let paths = fs::read_dir("./").unwrap();
    for path in paths {
        let file_info = format!("File: {}", path.unwrap().path().display());
        file_list.push_str(&file_info);
        file_list.push('\n');
    }
    res.send(file_list.as_bytes()).unwrap();
}*/

fn main() {
	let mut manager = DownloadManager::new();
	manager.add_url("http://rust-lang.org")
   		   .max_connection(4)
   		   .file("./rust-lang.html")
   		   .finish();
    let _download_thread = thread::spawn(move || {
  		manager.start();	
  	}).join();
}
