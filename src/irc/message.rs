use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Hostmask {
    User(String, String, String),
    Server(String),
    None,
}

impl fmt::Display for Hostmask {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Hostmask::User(ref nick, ref user, ref host) => write!(f, "{}!{}@{}", nick, user, host),
            Hostmask::Server(ref server) => write!(f, "{}", server),
            Hostmask::None => write!(f, "")
        }
    }
}

#[derive(Clone, Debug)]
pub enum Command {
    RPL_WELCOME(String),
    PING(String),
    PONG(String),
    USER(String, String, String, String),
    NICK(String),
    PASS(String),
    JOIN(String),
    PART(String, String),
    QUIT(String, String),
    PRIVMSG(String, String),
    NOTICE(String, String),
    UNDEFINED,
}

#[derive(Clone, Debug)]
pub struct Message {
    pub hostmask : Hostmask,
    pub command : Command,
    raw : String
}

impl Message {
    pub fn rpl_welcome(hostmask: Hostmask, param: &str) -> Message {
        let mut raw = format!(":{} 001 {}", hostmask, param);
        if hostmask == Hostmask::None {
            raw = format!("001 {}", param);
        }
        Message { hostmask: hostmask, command: Command::RPL_WELCOME(param.to_string()), raw: raw.to_string() }
    }

    pub fn ping(hostmask: Hostmask, param: &str) -> Message {
        let mut raw = format!(":{} PING {}", hostmask, param);
        if hostmask == Hostmask::None {
            raw = format!("PING {}", param);
        }
        Message { hostmask: hostmask, command: Command::PING(param.to_string()), raw: raw.to_string() }
    }

    pub fn pong(hostmask: Hostmask, param: &str) -> Message {
        let mut raw = format!(":{} PONG {}", hostmask, param);
        if hostmask == Hostmask::None {
            raw = format!("PONG {}", param);
        }
        Message { hostmask: hostmask, command: Command::PONG(param.to_string()), raw: raw.to_string() }
    }

    pub fn user(hostmask: Hostmask, username: &str, realname: &str) -> Message {
        let hostname = "*";
        let servername = "0";
        let mut raw = format!(":{} USER {} {} {} :{}", hostmask, username, hostname, servername, realname);
        if hostmask == Hostmask::None {
            raw = format!("USER {} {} {} :{}", username, hostname, servername, realname);
        }
        Message { hostmask: hostmask, command: Command::USER(username.to_string(), hostname.to_string(), servername.to_string(), realname.to_string()), raw: raw.to_string() }
    }

    pub fn nick(hostmask: Hostmask, nickname: &str) -> Message {
        let mut raw = format!(":{} NICK {}", hostmask, nickname);
        if hostmask == Hostmask::None {
            raw = format!("NICK {}", nickname);
        }
        Message { hostmask: hostmask, command: Command::NICK(nickname.to_string()), raw: raw.to_string() }
    }

    pub fn pass(hostmask: Hostmask, password: &str) -> Message {
        let mut raw = format!(":{} PASS {}", hostmask, password);
        if hostmask == Hostmask::None {
            raw = format!("PASS {}", password);
        }
        Message { hostmask: hostmask, command: Command::PASS(password.to_string()), raw: raw.to_string() }
    }

    pub fn join(hostmask: Hostmask, chan: &str) -> Message {
        let mut raw = format!(":{} JOIN {}", hostmask, chan);
        if hostmask == Hostmask::None {
            raw = format!("JOIN {}", chan);
        }
        Message { hostmask: hostmask, command: Command::JOIN(chan.to_string()), raw: raw.to_string() }
    }

    pub fn part(hostmask: Hostmask, chan: &str, message: &str) -> Message {
        let mut raw = format!(":{} PART {} :{}", hostmask, chan, message);
        if hostmask == Hostmask::None {
            raw = format!("PART {} :{}", chan, message);
        }
        Message { hostmask: hostmask, command: Command::PART(chan.to_string(), message.to_string()), raw: raw.to_string() }
    }

    pub fn quit(hostmask: Hostmask, chan: &str, message: &str) -> Message {
        let mut raw = format!(":{} QUIT {} :{}", hostmask, chan, message);
        if hostmask == Hostmask::None {
            raw = format!("QUIT {} :{}", chan, message);
        }
        Message { hostmask: hostmask, command: Command::QUIT(chan.to_string(), message.to_string()), raw: raw.to_string() }
    }

    pub fn privmsg(hostmask: Hostmask, chan: &str, message: &str) -> Message {
        let mut raw = format!(":{} PRIVMSG {} :{}", hostmask, chan, message);
        if hostmask == Hostmask::None {
            raw = format!("PRIVMSG {} :{}", chan, message);
        }
        Message { hostmask: hostmask, command: Command::PRIVMSG(chan.to_string(), message.to_string()), raw: raw.to_string() }
    }

    pub fn notice(hostmask: Hostmask, chan: &str, message: &str) -> Message {
        let mut raw = format!(":{} NOTICE {} :{}", hostmask, chan, message);
        if hostmask == Hostmask::None {
            raw = format!("NOTICE {} :{}", chan, message);
        }
        Message { hostmask: hostmask, command: Command::NOTICE(chan.to_string(), message.to_string()), raw: raw.to_string() }
    }

    pub fn read_hostmask(hostmask: &str) -> Hostmask {
        let sections : Vec<&str> = hostmask.split(|c| c == '!' || c == '@').collect();
        if sections.len() == 3 {
            Hostmask::User(sections[0].to_string(), sections[1].to_string(), sections[2].to_string())
        } else {
            Hostmask::Server(sections[0].to_string())
        }
    }

    pub fn from_string(line: &str) -> Message {
        let mut words = line.split_whitespace();
        let hostmask = if line.chars().next() == Some(':') {
            let sections : Vec<&str> = words.next().unwrap().split(|c| c == '!' || c == '@').collect();
            if sections.len() == 3 {
                Hostmask::User(sections[0].to_string().split_off(1), sections[1].to_string(), sections[2].to_string())
            } else {
                Hostmask::Server(sections[0].to_string().split_off(1))
            }
        } else {
            Hostmask::None
        };
        let command = match words.next() {
            Some("001") => {
                let params = words.collect::<Vec<&str>>().join(" ");
                Command::RPL_WELCOME(params.to_string())
            }
            Some("PING") => {
                let param = words.collect::<Vec<&str>>().join(" ");
                Command::PING(param.to_string())
            }
            Some("PONG") => {
                let param = words.collect::<Vec<&str>>().join(" ");
                Command::PONG(param.to_string())
            }
            Some("USER") => {
                let username = words.next().unwrap_or("");
                let hostname = words.next().unwrap_or("");
                let servername = words.next().unwrap_or("");
                let rest = words.collect::<Vec<&str>>().join(" ");
                let realname = rest.splitn(2, ':').collect::<Vec<&str>>().pop().unwrap_or("");
                Command::USER(username.to_string(), hostname.to_string(), servername.to_string(), realname.to_string())
            }
            Some("NICK") => {
                let nick = words.next().unwrap_or("");
                Command::NICK(nick.to_string())
            }
            Some("PASS") => {
                let pass = words.next().unwrap_or("");
                Command::PASS(pass.to_string())
            }
            Some("JOIN") => {
                let chan = words.next().unwrap_or("");
                Command::JOIN(chan.to_string())
            }
            Some("PART") => {
                let chan = words.next().unwrap_or("");
                let rest = words.collect::<Vec<&str>>().join(" ");
                let msg = rest.splitn(2, ':').collect::<Vec<&str>>().pop().unwrap_or("");
                Command::PART(chan.to_string(), msg.to_string())
            }
            Some("QUIT") => {
                let chan = words.next().unwrap_or("");
                let rest = words.collect::<Vec<&str>>().join(" ");
                let msg = rest.splitn(2, ':').collect::<Vec<&str>>().pop().unwrap_or("");
                Command::QUIT(chan.to_string(), msg.to_string())
            }
            Some("PRIVMSG") => {
                let chan = words.next().unwrap_or("");
                let rest = words.collect::<Vec<&str>>().join(" ");
                let msg = rest.splitn(2, ':').collect::<Vec<&str>>().pop().unwrap_or("");
                Command::PRIVMSG(chan.to_string(), msg.to_string())
            }
            Some("NOTICE") => {
                let chan = words.next().unwrap_or("");
                let rest = words.collect::<Vec<&str>>().join(" ");
                let msg = rest.splitn(2, ':').collect::<Vec<&str>>().pop().unwrap_or("");
                Command::NOTICE(chan.to_string(), msg.to_string())
            }
            _ => Command::UNDEFINED
        };
        Message { hostmask: hostmask, command: command, raw: line.to_string() }
    }

    pub fn to_string(&self) -> String {
        self.raw.clone()
    }
}
