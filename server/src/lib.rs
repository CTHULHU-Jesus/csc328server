use std::str::{FromStr,from_utf8};
use std::string::ToString;
use regex::Regex;
use lazy_static::*;
use std::net::*;
use std::sync::*;
use std::io::Write;
use std::time::Duration;
use chrono::Utc;
use std::path::Path;
extern crate libc;
use libc::*;
use std::os::unix::io::AsRawFd;

/// The Max size of message
const MESSAGE_MAX_SIZE : usize = 1024;
/// The Max size of a name
const NAME_MAX_SIZE : usize = 32;


/// The structure to describe Messages passed from client to server
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

// Library Bindings

// This is a reproduction of messageInfo in the library
#[derive(Clone,PartialEq,Eq)]
#[repr(C)]
struct messageInfo {
    protocol : c_int,
    name : [c_uchar; NAME_MAX_SIZE],
    msg : [c_uchar; MESSAGE_MAX_SIZE],
    msg_size : c_int,
    name_size : c_int,
} 
// This is initial state for messageInfo types because we have to initialize them before we pass them
const MESSAGEINFOINIT : messageInfo =
    messageInfo{
    protocol : 0,
    name : [0;NAME_MAX_SIZE],
    msg : [0;MESSAGE_MAX_SIZE],
    msg_size : 0,
    name_size : 0,
};
// here's the actual bindings to library functions
#[link(name = "cs", kind = "static")]
extern "C" {
    fn sendMessage(sockfd : c_int,
                   proto : c_int,
                   name : *mut c_char,
                   message : *mut c_char,
                   nameSize : c_int,
                   messageSize : c_int) -> c_int;
    fn receiveMessage(sockfd : c_int,
                      buf : *mut c_void,
                      size: c_int) -> c_int;
    fn getInfo(msgStruct : *mut messageInfo,
               buffer : *mut c_char) -> c_int;
}
// and now functions to make them easier to use
/// Sends a message to a client
/// stream: The stream to send a message to
/// msg: The message to send
/// returns () on success and None on failure
pub fn send_message(stream : &mut TcpStream, msg : Message, nickname : Option<String>) -> Option<()> {
    let proto : c_int;
    let mut string : Option<String> = None;
    let mut name = false;
    match msg {
        Message::HELLO   => proto = 0,
        Message::BYE     => proto = 1,
        Message::NICK(s) => {proto = 2;
                             string = Some(s);
                             name = true;}
        Message::READY   => proto = 3,
        Message::RETRY   => proto = 4,
        Message::CHAT(s) => {proto = 5;
                             string = Some(s);}
    }
    let message : *mut c_char;
    let message_size : c_int;
    match string {
        Some(s) => {message = s.clone().as_mut_str().as_mut_ptr() as *mut i8;
                    message_size = s.len() as c_int;}
        None    => {message = 0 as *mut i8;
                    message_size = 0;}
    }
    unsafe {
        if name {// Sending a nickname
            if sendMessage(stream.as_raw_fd(), proto, message, 0 as *mut i8, message_size, 0) == -1 {
                None
            } else {
                Some(())
            }
        } else {// Sending anything other than a nickname
            match nickname {
                Some(s) => {let ref mut nick_ar : [i8; NAME_MAX_SIZE] = [0; NAME_MAX_SIZE];
                            let s_as_bytes = s.as_bytes();
                            for x in 0..s.len() {
                                nick_ar[x] = s_as_bytes[x] as i8;
                            }
                            if sendMessage(stream.as_raw_fd(), proto, &nick_ar[0], message, s.len() as i32, message_size) == -1 {
                                None
                            } else {
                                Some(())
                            }}
                None    => {if sendMessage(stream.as_raw_fd(), proto, 0 as *mut i8, message, 0, message_size) == -1 {
                                None
                            } else {
                                Some(())
                            }}
            }
            
        }
    }
}

/// receves a message
/// stream: the stream to receve from
/// returns the message reveved
pub fn rcv_message(stream : &mut TcpStream) -> Option<Message> {
    unsafe {
        let buf : *mut c_void = &mut [0u8;MESSAGE_MAX_SIZE] as *mut _ as *mut c_void;
        let ref mut msg_info : messageInfo = MESSAGEINFOINIT.clone();
        let bytes_read = receiveMessage(stream.as_raw_fd(), buf, MESSAGE_MAX_SIZE as c_int); 
        if bytes_read == -1 {
            None
        } else {
            if getInfo(msg_info, buf.cast::<c_char>()) == -1 {return None}
            match msg_info.protocol {
                0 => Some(Message::HELLO),
                1 => Some(Message::BYE),
                2 => Some(Message::NICK(from_utf8(&msg_info.name).ok()?.trim_end().to_string())),
                3 => Some(Message::READY),
                4 => Some(Message::RETRY),
                5 => Some(Message::CHAT(from_utf8(&msg_info.msg).ok()?.trim_end().to_string())),
                _ => None,
            }
        }
    }
}


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
    for connection in conns {
        if connection.peer_addr().unwrap_or(*me) != *me {
            let message = Message::CHAT(format!("{}:{}",nick,message));
            send_message(&mut connection.try_clone().unwrap(), message, Some(*nick));
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
    while match rcv_message(stream) {
                Some(Message::NICK(n)) => {
                    // if the nickname is not taken set add it to the list of nicknames in
                    // use and then return the nick
                    if !nicknames.lock().ok()?.contains(&n.clone()) {
                        nicknames.lock().ok()?.push(n.clone());
                        send_message(stream, Message::READY, None);
                        // stream.write(Message::READY.to_string().as_bytes()).ok()?;
                        return Some(n);
                    } else {
                        // else ask the client to retry
                        // stream.write(Message::RETRY.to_string().as_bytes()).ok()?;
                        send_message(stream, Message::RETRY, None);
                        true
                    }
                }
                Some(Message::BYE) => return None,
                None => return None,
                _ => true,
        } {
    };
    None
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

