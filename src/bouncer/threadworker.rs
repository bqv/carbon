
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::borrow::Borrow;
use std::net::{TcpListener, TcpStream};
use std::ops::{Deref, DerefMut};
use std::thread;
use std::time::Duration;

use bouncer::Event;
use irc::server::Server;
use irc::client::Client;
use irc::message::Message;
use irc::Connection;

pub struct PingWorker<T: Connection> {
    conn : T,
}

impl<T: Connection> PingWorker<T> {
    pub fn new(conn: T) -> PingWorker<T> {
        PingWorker { conn: conn }
    }

    pub fn work(&mut self) {
        while self.conn.is_connected() {
            if self.conn.try_ping() {
                thread::sleep(Duration::from_secs(255));
            } else {
                break;
            }
        }
        println!("Dropping ping thread {:?}", thread::current().name());
    }
}

pub struct SendWorker<T: Connection> {
    conn : T,
    rx : Receiver<String>
}

impl<T: Connection> SendWorker<T> {
    pub fn new(conn: T, rx: Receiver<String>) -> SendWorker<T> {
        SendWorker { conn: conn, rx: rx }
    }

    pub fn work(&mut self) {
        while self.conn.is_connected() {
            match self.rx.recv() {
                Ok(string) => {
                    println!("[{}] <= {}", self.conn.name(), string);
                    self.conn.send(string);
                }
                Err(err) => println!("Error: {}", err)
            }
        }
        println!("Dropping send thread {:?}", thread::current().name());
    }
}

pub struct ReadWorker<T: Connection> {
    conn : T,
    tx : Sender<Event>,
}

impl ReadWorker<Server> {
    pub fn new(conn: Server, tx: Sender<Event>) -> ReadWorker<Server> {
        ReadWorker { conn: conn, tx: tx }
    }

    pub fn work(&mut self) {
        if let Ok(mut iter) = self.conn.read() {
            for line in iter {
                println!("[{}] => {}", self.conn.name(), line);
                self.tx.send(Event::ServerRead(self.conn.id(), Message::from_string(line.as_str())));
            }
        } else {
            println!("Error reading from IRC");
        }
        self.conn.set_connected(false);
        println!("Dropping read thread");
    }
}

impl ReadWorker<Client> {
    pub fn new(conn: Client, tx: Sender<Event>) -> ReadWorker<Client> {
        ReadWorker { conn: conn, tx: tx }
    }

    pub fn work(&mut self) {
        if let Ok(mut iter) = self.conn.read() {
            for line in iter {
                println!("[{}] => {}", self.conn.name(), line);
                self.tx.send(Event::ClientRead(self.conn.id(), Message::from_string(line.as_str())));
            }
        } else {
            println!("Error reading from IRC");
        }
        self.conn.set_connected(false);
        println!("Dropping read thread {:?}", thread::current().name());
    }
}

pub struct Listener {
    listener : TcpListener,
    eventtx : Sender<Event>,
}

impl Listener {
    pub fn new(listener: TcpListener, eventtx: Sender<Event>) -> Listener {
        Listener { listener: listener, eventtx: eventtx }
    }

    pub fn work(&mut self) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    self.eventtx.send(Event::AcceptConn(Arc::new(Mutex::new(stream))));
                }
                Err(e) => {
                    println!("Failed to accept connection");
                }
            }
        }
    }
}
