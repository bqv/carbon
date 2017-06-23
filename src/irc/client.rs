
use std::io::{self, BufReader, BufRead, Lines, Write};
use std::iter::FilterMap;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::ops::{Deref, DerefMut};

use irc::Connection;
use irc::message::{Message, Hostmask};

struct Userdata {
    username: String,
    hostname: String,
    realname: String,
}

pub struct Client {
    ping_active : Arc<Mutex<bool>>,
    connected : Arc<Mutex<bool>>,
    userdata : Arc<Mutex<Userdata>>,
    nick : Arc<Mutex<String>>,
    pub id : usize,
    name : String,
    stream : TcpStream,
    channels : Arc<Mutex<Vec<String>>>,
}

impl Client {
    pub fn from_stream(id: usize, stream: TcpStream) -> Client {
        let userdata = Userdata { username: "".to_string(), hostname: format!("{}", stream.peer_addr().unwrap().ip()), realname: "".to_string() };
        Client { id: id, stream: stream, connected: Arc::new(Mutex::new(true)), userdata: Arc::new(Mutex::new(userdata)), nick: Arc::new(Mutex::new("".to_string())), ping_active: Arc::new(Mutex::new(false)), name: "Client".to_string(), channels: Arc::new(Mutex::new(Vec::new())) }
    }

    pub fn try_clone(&self) -> io::Result<Client> {
        match self.stream.try_clone() {
            Ok(stream) => Ok(Client { id: self.id, stream: stream, connected: self.connected.clone(), userdata: self.userdata.clone(), nick: self.nick.clone(), ping_active: self.ping_active.clone(), name: self.name.clone(), channels: self.channels.clone() }),
            Err(err) => Err(err)
        }
    }

    pub fn get_nick(&self) -> String {
        self.nick.lock().unwrap().deref().clone()
    }

    pub fn set_nick(&mut self, value: String) {
        let mut nick = self.nick.lock().unwrap();
        *nick.deref_mut() = value;
    }

    pub fn set_userdata(&mut self, username: String, realname: String) {
        let mut userdata = self.userdata.lock().unwrap();
        userdata.deref_mut().username = username;
        userdata.deref_mut().realname = realname;
    }

    pub fn is_registered(&self) -> bool {
        !self.nick.lock().unwrap().deref().is_empty() &&
        !self.userdata.lock().unwrap().deref().username.is_empty()
    }

    pub fn hostmask(&self) -> Hostmask {
        let mut nick = self.nick.lock().unwrap();
        let mut userdata = self.userdata.lock().unwrap();
        Hostmask::User(nick.deref().clone(), userdata.deref().username.clone(), userdata.deref().hostname.clone())
    }

    pub fn nick(&self) -> String {
        let mut nick = self.nick.lock().unwrap();
        nick.deref().clone()
    }

    pub fn welcome_msg(&self, hostmask: Hostmask) -> Message {
        let param = format!("{} :Welcome to the Internet Relay Network {}", self.nick(), self.hostmask());
        Message::rpl_welcome(hostmask, param.as_str())
    }

    pub fn has_channel(&self, channel: &str) -> bool {
        let mut channels = self.channels.lock().unwrap();
        channels.deref().contains(&channel.to_string())
    }

    pub fn add_channel(&mut self, channel: &str) {
        let mut channels = self.channels.lock().unwrap();
        channels.deref_mut().push(channel.to_string());
    }
}

impl Connection for Client {
    fn id(&self) -> usize {
        self.id
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn is_connected(&self) -> bool {
        *self.connected.lock().unwrap().deref()
    }

    fn set_connected(&mut self, value: bool) {
        let mut connected = self.connected.lock().unwrap();
        *connected.deref_mut() = value;
    }

    fn try_ping(&mut self) -> bool {
        let mut ping_active = self.ping_active.clone();
        if *ping_active.lock().unwrap().deref() {
            self.set_connected(false);
            false
        } else {
            let string = Message::ping(Hostmask::None, ":carbon").to_string();
            println!("[{}] <= {}", self.name(), string);
            *ping_active.lock().unwrap().deref_mut() = true;
            self.send(string);
            true
        }
    }

    fn register_pong(&mut self) {
        let mut ping_active = self.ping_active.lock().unwrap();
        *ping_active.deref_mut() = false;
    }

    fn send(&mut self, string: String) -> io::Result<()> {
        let clone = self.try_clone();
        let newline = |mut this: Client| this.stream.write_all(b"\r\n");
        self.stream.write_all(string.as_bytes()).and(clone).and_then(newline)
    }

    fn read(&mut self) -> io::Result<FilterMap<Lines<BufReader<TcpStream>>, fn(io::Result<String>) -> Option<String>>> {
        match self.stream.try_clone() {
            Ok(stream) => Ok(BufReader::new(stream).lines().filter_map(Result::ok)),
            Err(err) => Err(err)
        }
    }
}
