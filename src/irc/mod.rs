
use std::io::{self, BufReader, Lines};
use std::iter::FilterMap;
use std::net::TcpStream;

pub mod message;
pub mod server;
pub mod client;

#[derive(Clone)]
pub struct Config {
    pub name : String,
    pub nick : String,
    pub host : String,
    pub port : u16,
    pub pass : String,
    pub ssl : bool,
    pub chans : Vec<String>,
}

pub trait Connection {
    fn id(&self) -> usize;
    fn name(&self) -> String;
    fn is_connected(&self) -> bool;
    fn set_connected(&mut self, value: bool);
    fn try_ping(&mut self) -> bool;
    fn register_pong(&mut self);
    fn send(&mut self, string: String) -> io::Result<()>;
    fn read(&mut self) -> io::Result<FilterMap<Lines<BufReader<TcpStream>>, fn(io::Result<String>) -> Option<String>>>;
}

