
use std::thread;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::ops::{Deref, DerefMut};
use std::net::{TcpListener, TcpStream};
use std::collections::HashMap;
use std::io;

use irc::Config;
use irc::server::Server;
use irc::client::Client;
use irc::message::{Message, Command, Hostmask};
use irc::Connection;

mod threadworker;

#[derive(Clone, Debug)]
pub enum Event {
    ServerRead(usize, Message),
    ClientRead(usize, Message),
    AcceptConn(Arc<Mutex<TcpStream>>),
}

pub struct Bouncer {
    hostmask : Hostmask,
    configs : Vec<Config>,
    servers : Vec<Server>,
    names : HashMap<String, usize>,
    clients : Vec<Client>,
    srvsendtxs : Vec<Sender<String>>,
    clntsendtxs : Vec<Sender<String>>,
    eventrx : Receiver<Event>,
    eventtx : Sender<Event>,
}

impl Bouncer {
    pub fn new(cfgs: Vec<Config>) -> io::Result<Bouncer> {
        let (eventtx, eventrx) = channel();
        match TcpListener::bind("0.0.0.0:6677") {
            Ok(listener) => {
                let listeneventtx = eventtx.clone();
                thread::Builder::new().name("LISTEN".to_string()).spawn(move || {
                    threadworker::Listener::new(listener, listeneventtx).work();
                });
                Ok(Bouncer { hostmask: Hostmask::Server("carbon.fron.io".to_string()), configs: cfgs, srvsendtxs: Vec::new(), clntsendtxs: Vec::new(), servers: Vec::new(), clients: Vec::new(), names: HashMap::new(), eventrx: eventrx, eventtx: eventtx })
            },
            Err(err) => Err(err)
        }
    }

    pub fn run(mut self) {
        for (i, cfg) in self.configs.clone().iter().enumerate() {
            self.start_server(i, cfg.clone());
        }
        loop {
            let msgresult = self.eventrx.recv().clone();
            match msgresult {
                Ok(Event::ServerRead(id, msg)) => {
                    self.handlesrv(id, msg);
                }
                Ok(Event::ClientRead(id, msg)) => {
                    self.handleclnt(id, msg);
                }
                Ok(Event::AcceptConn(rc)) => {
                    match rc.lock().unwrap().try_clone() {
                        Ok(stream) => {
                            self.start_client(stream);
                        }
                        Err(err) => {
                            println!("Error accepting connection");
                        }
                    }
                }
                Err(err) => {
                    println!("Error in read: {}", err);
                }
            }
        }
    }

    fn start_client(&mut self, stream: TcpStream) {
        let (sendtx, sendrx) = channel();
        self.clntsendtxs.push(sendtx.clone());
        let readeventtx = self.eventtx.clone();
        let clientname = format!("{}", stream.peer_addr().unwrap());
        let client = Client::from_stream(self.clients.len(), stream);
        match client.try_clone() {
            Ok(client_clone) => {
                let readthreadname = format!("{}-IN", clientname);
                let readthread = thread::Builder::new().name(readthreadname).spawn(move || {
                    threadworker::ReadWorker::<Client>::new(client_clone, readeventtx).work();
                });
            }
            Err(err) => {
                println!("Error creating read thread");
            }
        }
        match client.try_clone() {
            Ok(client_clone) => {
                let sendthreadname = format!("{}-OUT", clientname);
                let sendthread = thread::Builder::new().name(sendthreadname).spawn(move || {
                    threadworker::SendWorker::new(client_clone, sendrx).work();
                });
            }
            Err(err) => {
                println!("Error creating send thread");
            }
        }
        match client.try_clone() {
            Ok(client_clone) => {
                let pingthreadname = format!("{}-PING", clientname);
                let pingthread = thread::Builder::new().name(pingthreadname).spawn(move || {
                    threadworker::PingWorker::new(client_clone).work();
                });
            }
            Err(err) => {
                println!("Error creating send thread");
            }
        }
        self.clients.push(client);
    }

    fn start_server(&mut self, id: usize, cfg: Config) {
        let (sendtx, sendrx) = channel();
        self.srvsendtxs.push(sendtx.clone());
        let readtx = self.eventtx.clone();
		if let Ok(server) = Server::connect(id, cfg) {
            if let Ok(server_clone) = server.try_clone() {
                let readthreadname = format!("{}-IN", server.name());
                let readthread = thread::Builder::new().name(readthreadname).spawn(move || {
                    threadworker::ReadWorker::<Server>::new(server_clone, readtx).work();
                });
            } else {
                println!("Error starting read thread");
            }
            if let Ok(server_clone) = server.try_clone() {
                let sendthreadname = format!("{}-OUT", server.name());
                let sendthread = thread::Builder::new().name(sendthreadname).spawn(move || {
                    sendtx.send(Message::user(Hostmask::None, server_clone.config.nick.as_str(), "carbon").to_string());
                    sendtx.send(Message::nick(Hostmask::None, server_clone.config.nick.as_str()).to_string());
                    match server_clone.config.pass.as_str() {
                        "" => (),
                        pass => {
                            sendtx.send(Message::pass(Hostmask::None, pass).to_string());
                        }
                    }
                    threadworker::SendWorker::new(server_clone, sendrx).work();
                });
            } else {
                println!("Error starting send thread");
            }
            self.names.insert(server.name(), self.servers.len());
            self.servers.push(server);
        } else {
            println!("Error connecting to IRC");
        }
    }

    fn handlesrv(&mut self, id: usize, msg: Message) {
        match msg.command {
            Command::RPL_WELCOME(ref params) => {
                if let Ok(server) = self.servers[id].try_clone() {
                    let pingthreadname = format!("{}-PING", server.name());
                    let pingthread = thread::Builder::new().name(pingthreadname).spawn(move || {
                        threadworker::PingWorker::new(server).work();
                    });
                } else {
                    println!("Error starting ping thread");
                    self.servers[id].set_connected(false);
                }

                let nick = params.split(' ').next().unwrap();
                self.servers[id].set_nick(nick.to_string());

                for chan in self.servers[id].config.chans.iter() {
                    self.send_srv(id, Message::join(Hostmask::None, chan.as_str()).to_string())
                }
            }
            Command::PING(ref param) => {
                self.send_srv(id, Message::pong(Hostmask::None, param).to_string());
            }
            Command::PONG(ref param) => {
                self.servers[id].register_pong();
            }
            Command::JOIN(ref chan) => {
                match msg.hostmask {
                    Hostmask::User(ref nick, _, _) => {
                        if *nick == self.servers[id].get_nick() {
                            self.servers[id].add_channel(chan);
                        }
                        else {
                            for client in &self.clients {
                                let rawchan = "#".to_string()+self.servers[id].name().as_str()+chan;
                                if client.has_channel(rawchan.as_str()) {
                                    self.send_clnt(client.id, Message::join(msg.hostmask.clone(), rawchan.as_str()).to_string());
                                }
                            }
                        }
                    }
                    Hostmask::Server(_) => (),
                    Hostmask::None => ()
                }
            }
            Command::PART(ref chan, ref message) => {
                match msg.hostmask {
                    Hostmask::User(ref nick, _, _) => {
                        if *nick == self.servers[id].get_nick() {
                            self.servers[id].remove_channel(chan);
                        }
                        else {
                            for client in &self.clients {
                                let rawchan = "#".to_string()+self.servers[id].name().as_str()+chan;
                                if client.has_channel(rawchan.as_str()) {
                                    self.send_clnt(client.id, Message::part(msg.hostmask.clone(), rawchan.as_str(), message).to_string());
                                }
                            }
                        }
                    }
                    Hostmask::Server(_) => (),
                    Hostmask::None => ()
                }
            }
            Command::QUIT(ref chan, ref message) => {
                match msg.hostmask {
                    Hostmask::User(ref nick, _, _) => {
                        if *nick == self.servers[id].get_nick() {
                            /* handle quit */
                        }
                        else {
                            for client in &self.clients {
                                let rawchan = "#".to_string()+self.servers[id].name().as_str()+chan;
                                if client.has_channel(rawchan.as_str()) {
                                    self.send_clnt(client.id, Message::quit(msg.hostmask.clone(), rawchan.as_str(), message).to_string());
                                }
                            }
                        }
                    }
                    Hostmask::Server(_) => (),
                    Hostmask::None => ()
                }
            }
            Command::PRIVMSG(ref chan, ref message) => {
                for client in &self.clients {
                    let rawchan = "#".to_string()+self.servers[id].name().as_str()+chan;
                    if client.has_channel(rawchan.as_str()) {
                        self.send_clnt(client.id, Message::privmsg(msg.hostmask.clone(), rawchan.as_str(), message).to_string());
                    }
                }
            }
            Command::NOTICE(ref chan, ref message) => {
                for client in &self.clients {
                    let rawchan = "#".to_string()+self.servers[id].name().as_str()+chan;
                    if client.has_channel(rawchan.as_str()) {
                        self.send_clnt(client.id, Message::notice(msg.hostmask.clone(), rawchan.as_str(), message).to_string());
                    }
                }
            }
            _ => ()
        }
    }

    fn handleclnt(&mut self, id: usize, msg: Message) {
        match msg.command {
            Command::USER(ref username, _, _, ref realname) => {
                self.clients[id].set_userdata(username.clone(), realname.clone());
                if self.clients[id].is_registered() {
                    let welcomemsg = self.clients[id].welcome_msg(self.hostmask.clone());
                    self.send_clnt(id, welcomemsg.to_string());
                }
            }
            Command::NICK(ref nick) => {
                self.clients[id].set_nick(nick.clone());
                if self.clients[id].is_registered() {
                    let welcomemsg = self.clients[id].welcome_msg(self.hostmask.clone());
                    self.send_clnt(id, welcomemsg.to_string());
                }
            }
            Command::PING(ref param) => {
                self.send_clnt(id, Message::pong(self.hostmask.clone(), param).to_string());
            }
            Command::PONG(ref param) => {
                self.clients[id].register_pong();
            }
            Command::JOIN(ref chans) => {
                let chanlist = chans.split(',').map(|x| x.trim().split_at(1));
                for (_, serverchan) in chanlist {
                    let rawchan = "#".to_string()+serverchan;
                    match serverchan.find('#') {
                        Some(pos) => {
                            let (server, chan) = serverchan.split_at(pos);
                            match self.names.get(server) {
                                Some(sid) => {
                                    if !self.servers[*sid].has_channel(chan) {
                                        self.send_srv(*sid, Message::join(Hostmask::None, chan).to_string());
                                    }
                                    self.clients[id].add_channel(rawchan.as_str());
                                }
                                None => println!("Server {} is unresolved", server)
                            }
                        }
                        None => {
                            println!("No channel #{} found", serverchan);
                        }
                    }
                    self.send_clnt(id, Message::join(self.clients[id].hostmask(), rawchan.as_str()).to_string());
                }
            }
            Command::PRIVMSG(ref chan, ref message) => {
                let (_, serverchan) = chan.split_at(1);
                match serverchan.find('#') {
                    Some(pos) => {
                        let (server, target) = serverchan.split_at(pos);
                        if self.clients[id].has_channel(chan.as_str()) {
                            match self.names.get(server) {
                                Some(sid) => self.send_srv(*sid, Message::privmsg(Hostmask::None, target, message).to_string()),
                                None => println!("Server {} is unresolved", server)
                            }
                        }
                    }
                    None => {
                        println!("No channel #{} found", serverchan);
                    }
                }
            }
            Command::NOTICE(ref chan, ref message) => {
                let (_, serverchan) = chan.split_at(1);
                match serverchan.find('#') {
                    Some(pos) => {
                        let (server, target) = serverchan.split_at(pos);
                        if self.clients[id].has_channel(chan.as_str()) {
                            match self.names.get(server) {
                                Some(sid) => self.send_srv(*sid, Message::notice(Hostmask::None, target, message).to_string()),
                                None => println!("Server {} is unresolved", server)
                            }
                        }
                    }
                    None => {
                        println!("No channel #{} found", serverchan);
                    }
                }
            }
            _ => ()
        }
    }

    fn send_srv(&self, id: usize, line: String) {
        self.srvsendtxs[id].send(line);
    }

    fn send_clnt(&self, id: usize, line: String) {
        self.clntsendtxs[id].send(line);
    }
}
