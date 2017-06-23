extern crate yaml_rust;
extern crate rand;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use yaml_rust::{YamlLoader, YamlEmitter, yaml};
use rand::Rng;

mod irc;
mod bouncer;

fn main() {
    let args: Vec<_> = env::args().collect();
    let defaultconf = "conf.yaml".to_string();
    let conffile = args.get(1).unwrap_or_else(|| &defaultconf);
    let mut f = File::open(&conffile).unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();

    let mut cfgs = Vec::new();

    let docs = YamlLoader::load_from_str(s.as_str()).unwrap();

    // Multi document support, doc is a yaml::Yaml
    let doc = &docs[0];

    match *doc {
        yaml::Yaml::Hash(ref h) => {
            let mut rng = rand::thread_rng();
            for (k, v) in h {
                match *k {
                    yaml::Yaml::String(ref name) => {
                        let mut nick = format!("carbon{}", rng.gen::<u16>());
                        let mut host = String::new();
                        let mut port = 6667;
                        let mut pass = String::new();
                        let mut ssl = false;
                        let mut chans = Vec::new();
                        match *v {
                            yaml::Yaml::Hash(ref h) => {
                                for (k, v) in h {
                                    match *k {
                                        yaml::Yaml::String(ref key) => {
                                            match key.as_ref() {
                                                "nick" => {
                                                    match *v {
                                                        yaml::Yaml::String(ref s) => {
                                                            nick = s.clone();
                                                        }
                                                        _ => println!("Malformed config file: Expected string for nick")
                                                    }
                                                }
                                                "host" => {
                                                    match *v {
                                                        yaml::Yaml::String(ref s) => {
                                                            host = s.clone();
                                                        }
                                                        _ => println!("Malformed config file: Expected string for host")
                                                    }
                                                }
                                                "port" => {
                                                    match *v {
                                                        yaml::Yaml::Integer(ref i) => {
                                                            port = *i as u16;
                                                        }
                                                        _ => println!("Malformed config file: Expected integer for port")
                                                    }
                                                }
                                                "pass" => {
                                                    match *v {
                                                        yaml::Yaml::String(ref s) => {
                                                            pass = s.clone();
                                                        }
                                                        _ => println!("Malformed config file: Expected string for pass")
                                                    }
                                                }
                                                "ssl" => {
                                                    match *v {
                                                        yaml::Yaml::Boolean(ref b) => {
                                                            ssl = *b;
                                                        }
                                                        _ => println!("Malformed config file: Expected boolean for ssl")
                                                    }
                                                }
                                                "chans" => {
                                                    match *v {
                                                        yaml::Yaml::Array(ref a) => {
                                                            for x in a {
                                                                match *x {
                                                                    yaml::Yaml::String(ref s) => {
                                                                        chans.push(s.clone());
                                                                    }
                                                                    _ => println!("Malformed config file: Expected string in server channel list")
                                                                }
                                                            }
                                                        }
                                                        _ => println!("Malformed config file: Expected array of server channels")
                                                    }
                                                }
                                                _ => println!("Malformed config file: Unexpected server parameter")
                                            }
                                        }
                                        _ => println!("Malformed config file: Expected string in server parameters")
                                    }
                                }
                            }
                            _ => println!("Malformed config file: Expected hash of server parameters")
                        }
                        if !host.is_empty() {
                            let cfg = irc::Config {name: name.clone(), nick: nick, host: host, port: port, pass: pass, ssl: ssl, chans: chans};
                            cfgs.push(cfg);
                        }
                    }
                    _ => println!("Malformed config file: Expected server name string")
                }
            }
        }
        _ => {
            println!("Malformed config file: Expected a hash of servers");
        }
    }

    let bnc = bouncer::Bouncer::new(cfgs).unwrap();
    bnc.run();

    // Chained key/array access is checked and won't panic,
    // return BadValue if they are not exist.
    assert!(doc["INVALID_KEY"][100].is_badvalue());
    
    // Dump the YAML object
    let mut out_str = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut out_str);
        emitter.dump(doc).unwrap(); // dump the YAML object to a String
    }
    println!("{}", out_str);
}
