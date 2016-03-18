extern crate hyper;

use std::io::Read;

use hyper::Client;
use hyper::header::Connection;

fn main() {
    // Create a client.
    let mut client = Client::new();
    let mut res = client.get("http://127.0.0.1:3000")
        .header(Connection::close())
        .send().unwrap();
    let mut body = String::new();
    res.read_to_string(&mut body).unwrap();
    println!("{}",body);
}