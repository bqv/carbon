
use std::io::{self, BufReader, BufRead, Lines, Write};
use std::iter::FilterMap;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::ops::{Deref, DerefMut};

use irc::Config;
use irc::Connection;
use irc::message::{Message, Hostmask};

pub struct Server {
    ping_active : Arc<Mutex<bool>>,
    connected : Arc<Mutex<bool>>,
    userdata : Arc<Mutex<Userdata>>,
    nick : Arc<Mutex<String>>,
    id : usize,
    pub config : Config,
    stream : TcpStream,
    channels : Arc<Mutex<Vec<String>>>,
}

struct Userdata {
    username: String,
    hostname: String,
    realname: String,
}

impl Server {
    pub fn connect(id: usize, config: Config) -> io::Result<Server> {
        let userdata = Userdata { username: "".to_string(), hostname: "".to_string(), realname: "".to_string() };
        match TcpStream::connect((config.host.as_str(), config.port)) {
            Ok(sock) => {
                Ok(Server { id: id, config: config, stream: sock, connected: Arc::new(Mutex::new(true)), userdata: Arc::new(Mutex::new(userdata)), nick: Arc::new(Mutex::new("".to_string())), ping_active: Arc::new(Mutex::new(false)), channels: Arc::new(Mutex::new(Vec::new())) })
            },
            Err(err) => Err(err)
        }
    }

    pub fn try_clone(&self) -> io::Result<Server> {
        match self.stream.try_clone() {
            Ok(stream) => Ok(Server { id: self.id, config: self.config.clone(), stream: stream, connected: self.connected.clone(), userdata: self.userdata.clone(), nick: self.nick.clone(), ping_active: self.ping_active.clone(), channels: self.channels.clone() }),
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

    pub fn set_userdata(&mut self, username: String, hostname: String) {
        let mut userdata = self.userdata.lock().unwrap();
        userdata.deref_mut().username = username;
        userdata.deref_mut().hostname = hostname;
    }

    pub fn hostmask(&self) -> Hostmask {
        let mut nick = self.nick.lock().unwrap();
        let mut userdata = self.userdata.lock().unwrap();
        Hostmask::User(nick.deref().clone(), userdata.deref().username.clone(), userdata.deref().hostname.clone())
    }

    pub fn has_channel(&self, channel: &str) -> bool {
        let mut channels = self.channels.lock().unwrap();
        channels.deref().contains(&channel.to_string())
    }

    pub fn add_channel(&mut self, channel: &str) {
        let mut channels = self.channels.lock().unwrap();
        channels.deref_mut().push(channel.to_string());
    }

    pub fn remove_channel(&mut self, channel: &str) {
        let mut channels = self.channels.lock().unwrap();
        let index = channels.deref().iter().position(|ref r| *r == channel).unwrap();
        channels.deref_mut().swap_remove(index);
    }
}

impl Connection for Server {
    fn id(&self) -> usize {
        self.id
    }

    fn name(&self) -> String {
        self.config.name.clone()
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
        let newline = |mut this: Server| this.stream.write_all(b"\r\n");
        self.stream.write_all(string.as_bytes()).and(clone).and_then(newline)
    }

    fn read(&mut self) -> io::Result<FilterMap<Lines<BufReader<TcpStream>>, fn(io::Result<String>) -> Option<String>>> {
        match self.stream.try_clone() {
            Ok(stream) => Ok(BufReader::new(stream).lines().filter_map(Result::ok)),
            Err(err) => Err(err)
        }
    }
}
