use std::str::{FromStr};
use std::string::ToString;
use regex::Regex;
use lazy_static::*;
use std::net::*;
use std::sync::*;
use std::io::{Read,Write};
use std::time::Duration;
use std::fs::File;
use chrono::Utc;
use std::path::Path;


/// Removes all dead connections from a vector
/// conns : The list of active connections
/// Returns a vector of all active connections
pub fn remove_dead_connections(conns : &Vec<TcpStream>) -> Vec<TcpStream> {
    conns.into_iter().filter_map( |x| {
        match x.take_error() {
            Err(_) => None,
            Ok(Some(_)) => None,
            Ok(None) => Some(x.try_clone().ok()?),
        }
    }).collect()
}

/// Dissconnects from all active connections, waits 5 seconds and then ends
/// conns : The active connections
pub fn disconnect_all_connections(conns : &Vec<TcpStream>) {
    const TIMEOUT_TIMER : Duration = Duration::from_secs(5);
    let conns : Vec<TcpStream>  = conns.iter().filter_map( |x| x.try_clone().ok()).collect();
    println!("\nDisconnecting from all connections and closeing");
    for conn in conns {
        let conn = Box::new(conn);
        std::thread::spawn( move || {
            //@TODO tell client to shutdown
            conn.shutdown(Shutdown::Both).unwrap_or(());
        });
    };
    std::thread::sleep(TIMEOUT_TIMER);
}

/// Removes a connection from a list of connections
/// conns : The active connections
/// to_remove : The connection to remove
pub fn remove_connection(conns : &mut Vec<TcpStream>, to_remove : &TcpStream) {
    let peer = to_remove.peer_addr().unwrap();
    *conns = conns
        .into_iter()
        .filter(|x| x.peer_addr().unwrap() != peer)
        .map(|x| x.try_clone().unwrap()).collect::<Vec<_>>();
}

/// Removes a nickname from the list of active nicknames
/// nicknames : The list of active nicknames
/// to_remove : The nickname to be removed
pub fn remove_nickname(nicknames : &mut Vec<String>, to_remove : &String) {
    *nicknames = nicknames
        .into_iter()
        .filter( |x| **x != *to_remove )
        .map( |x| x.clone())
        .collect();
}


/// Sends a message from one user to all other users
/// conns : The current open connections.
/// me    : The address of the user sending the message.
/// nick  : The nickname of the user sending the message.
/// message : The message being sent.
pub fn blast_out(conns : &Vec<TcpStream>, me : &SocketAddr, nick : &String, message : &String) -> () {
    for mut connection in conns {
        if connection.peer_addr().unwrap_or(*me) != *me {
            connection.write(
                Message::CHAT(format!("{}:{}",nick,message))
                .to_string()
                .as_bytes()
            ).unwrap_or(0);
        }
    };
}

/// Logs the input to a file and the screen, adds a time stamp
/// log_message : the string to log
pub fn log(log_message : &String) {
    let log_message = format!("{}\t{}\n",Utc::now(),log_message);
    let log_file_name : &Path = Path::new("logfile.log");
    println!("{}",log_message);
    match std::fs::OpenOptions::new()
        .append(true)
        .open(log_file_name) {
            Ok(mut x) => {
                match x.write(log_message.as_bytes()) {
                    Ok(_) => (),
                    Err(x) => println!("Could not write to log file because: {:?}",x),
                }
            }
            Err(x) => println!("Could not open log file because: {:?}",x),
        }
}

/// Gets a valid nickname from the user 
/// and adds the nickname to the list of nicknames in use
/// stream : the tcpstream connected to the user
/// nicknames : the global list of all nicknames
/// returns : a valid nickname or None if the user wants to end the connection or an error occurred
pub fn get_nickname(stream : &mut TcpStream, nicknames : &Arc<Mutex<Vec<String>>>) -> Option<String> {
    const NAME_MAX_SIZE : usize = 32;
    let mut message = [0;NAME_MAX_SIZE];
    while match stream.read(&mut message) {
        Ok(size) => {
            let message : Message =
                std::str::FromStr::from_str(
                std::str::from_utf8(
                    &message[0..size]).ok()?
                )
                .unwrap();
            match message {
                Message::NICK(n) => {
                    // if the nickname is not taken set add it to the list of nicknames in
                    // use and then return the nick
                    if !nicknames.lock().ok()?.contains(&n.clone()) {
                        nicknames.lock().ok()?.push(n.clone());
                        stream.write(Message::READY.to_string().as_bytes()).ok()?;
                        return Some(n);
                    } else {
                        // else ask the client to retry
                        stream.write(Message::RETRY.to_string().as_bytes()).ok()?;
                        true
                    }
                }
                Message::BYE => return None,
                _ => true,
            }
        },
        Err(_) => return None
    } {
        message = [0;NAME_MAX_SIZE];
    };
    None
}

#[derive(Clone,PartialEq,Eq)]
pub enum Message {
    HELLO,
    NICK(String),
    BYE,
    READY,
    RETRY,
    CHAT(String),
}


impl FromStr for Message {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const ERR : &str = "could not parse message from user";
        lazy_static!{
            static ref HELLO : Regex = Regex::new(r"HELLO").unwrap();
            static ref NICK  : Regex = Regex::new(r"NICK*").unwrap();
            static ref BYE   : Regex = Regex::new(r"BYE").unwrap();
            static ref READY : Regex = Regex::new(r"READY").unwrap();
            static ref RETRY : Regex = Regex::new(r"RETRY").unwrap();
            static ref CHAT  : Regex = Regex::new(r"CHAT*").unwrap();
        }
        match s {
            _ if HELLO.is_match(s) => Ok(Message::HELLO),
            _ if NICK.is_match(s)  => {
                let mut mut_s = s.to_string();
                mut_s.replace_range(..4,"");
                Ok(Message::NICK(mut_s))
            }
            _ if BYE.is_match(s)   => Ok(Message::BYE),
            _ if READY.is_match(s) => Ok(Message::READY),
            _ if RETRY.is_match(s) => Ok(Message::RETRY),
            _ if CHAT.is_match(s)  => {
                let mut mut_s = s.to_string();
                mut_s.replace_range(..4,"");
                Ok(Message::CHAT(mut_s))
            }
            _ => Err(ERR), 

        }
    }
}

impl ToString for Message {
    fn to_string(&self) -> String {
        match self {
            Message::HELLO => "HELLO".to_string(),
            Message::NICK(n) => format!("NICK{}",n),
            Message::BYE     => "BYE".to_string(),
            Message::READY   => "READY".to_string(),
            Message::RETRY   => "RETRY".to_string(),
            Message::CHAT(m) => format!("CHAT{}",m),
        }
    }
}

/// tests to_sting and from_str for the Message type
pub fn test_message() -> () {
    // test HELLO,
    println!("test HELLO to_string '{}'",Message::HELLO.to_string());
    let hello_str : &str = "HELLO";
    println!("test HELLO from_str '{}' -> '{}'", hello_str, Message::from_str(hello_str).unwrap().to_string());
    // test NICK(String),
    println!("test NICK to_string '{}'",Message::NICK("name".to_string()).to_string());
    let nick_str : &str = "NICKname";
    println!("test NICK from_str '{}' -> '{}'", nick_str, Message::from_str(nick_str).unwrap().to_string());
    // test BYE,
    println!("test BYE to_string '{}'",Message::BYE.to_string());
    let bye_str : &str = "BYE";
    println!("test BYE from_str '{}' -> '{}'", bye_str, Message::from_str(bye_str).unwrap().to_string());
    // test READY,
    println!("test READY to_string '{}'",Message::READY.to_string());
    let ready_str : &str = "READY";
    println!("test READY from_str '{}' -> '{}'", ready_str, Message::from_str(ready_str).unwrap().to_string());
    // test RETRY,
    println!("test RETRY to_string '{}'",Message::RETRY.to_string());
    let retry_str : &str = "RETRY";
    println!("test RETRY from_str '{}' -> '{}'", retry_str, Message::from_str(retry_str).unwrap().to_string());
    // test CHAT(String),
    println!("test CHAT to_string '{}'",Message::CHAT("words words".to_string()).to_string());
    let chat_str : &str = "CHATwords words";
    println!("test CHAT from_str '{}' -> '{}'", chat_str, Message::from_str(chat_str).unwrap().to_string());
 
}

