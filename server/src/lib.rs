//   Authors:        Matthew Bartlett              & Arron Harman
//   Major:          (Software Development & Math) & (Software Development)
//   Creation Date:  October  27, 2020
//   Due Date:       November 24, 2020
//   Course:         CSC328
//   Professor Name: Dr. Frye
//   Assignment:     Chat Server
//   Filename:       main.rs
//   Purpose:        Create and document functions to be used to create a chat server
use std::str::from_utf8;
use std::string::ToString;
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

/// Implements the to_string method for Message, ONLY TO BE USED FOR LOGGING.
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

/// This is a reproduction of the C-struct messageInfo in the C library
#[derive(Clone)]
#[repr(C)]
struct messageInfo {
    protocol : c_int,
    name : [c_uchar; NAME_MAX_SIZE],
    msg : [c_uchar; MESSAGE_MAX_SIZE],
    msg_size : c_int,
    name_size : c_int,
} 

/// This is initial state for messageInfo types because we have to initialize them before we pass them
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
    /// Send a correctly formated message.
    /// socfd: The file descriptor for the socket.
    /// proto: The number of the protocol
    ///     0 -> HELLO
    ///     1 -> BYE
    ///     2 -> NICK
    ///     3 -> READY
    ///     4 -> RETRY
    ///     5 -> CHAT
    /// name: C-string that is the name of the person sending a CHAT or the Name portion of a
    /// NICK message otherwise 0.
    /// message: C-string that is the message of a CHAT otherwise 0.
    /// nameSizeL: NAME_MAX_SIZE for CHAT messages otherwise 0.
    /// messageSize: MESSAGE_MAX_SIZE for CHAT messages NAME_MAX_SIZE for NICK messages otherwise
    /// 0.
    /// returns -1 if there was an error, otherwise the number of bytes sent.
    fn sendMessage(
        sockfd : c_int,
        proto : c_int,
        name : *mut c_char,
        message : *mut c_char,
        nameSize : c_int,
        messageSize : c_int) -> c_int;
    
    /// Receives a correctly formated message.
    /// socfd: The file descriptor for the socket.
    /// buf: The buffer for the message.
    /// size: The max size of the buffer.
    /// returns -1 if there was an error, otherwise the number of bytes read.
    fn receiveMessage(
        sockfd : c_int,
        buf : *mut c_void,
        size: c_int) -> c_int;
    
    /// Parses a received message.
    /// msgStruct: The message struct to be filled with info from the buffer.
    /// buffer: The received message buffer.
    /// returns -1 if there was an error, otherwise 0
    fn getInfo(
        msgStruct : *mut messageInfo,
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
    let mut message = ['\0' as i8; MESSAGE_MAX_SIZE];
    fn min(a : usize, b : usize) -> usize {
        if a < b 
            {a}
        else 
            {b}
    };
    match string.clone() {
        Some(s) => {for i in 0..min(MESSAGE_MAX_SIZE,s.as_bytes().len()) {
            message[i] = s.as_bytes()[i] as i8;
        };}
                    
        None    => (),
    }
    // fn sendMessage(sockfd : c_int,proto : c_int,name : *mut c_char,message : *mut c_char,nameSize : c_int,messageSize : c_int) -> c_int;
    unsafe {
        if name {// Sending a nickname
            if sendMessage(stream.as_raw_fd(), proto, &mut message[0], 0 as *mut i8, NAME_MAX_SIZE as i32, 0) == -1 {
                None
            } else {
                Some(())
            }
        } else {// Sending anything other than a nickname
            match nickname {
                Some(s) => {let ref mut nick_ar : [i8; NAME_MAX_SIZE] = ['\0' as i8; NAME_MAX_SIZE];
                            for x in 0..s.len() {
                                nick_ar[x] = s.as_bytes()[x] as i8;
                            }
                            if sendMessage(stream.as_raw_fd(), proto, 
                                &mut nick_ar[0], &mut message[0], 
                                NAME_MAX_SIZE as i32, MESSAGE_MAX_SIZE as i32) == -1 {
                                None
                            } else {
                                Some(())
                            }}
                None    => {if sendMessage(stream.as_raw_fd(), proto, 
                                0 as *mut i8, &mut message[0],
                                0, MESSAGE_MAX_SIZE as i32) == -1 {
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
        let buf = &mut ['\0' as u8;MESSAGE_MAX_SIZE] as *mut _ as *mut c_void;
        let ref mut msg_info : messageInfo = MESSAGEINFOINIT.clone();
        let bytes_read = receiveMessage(stream.as_raw_fd(), buf , MESSAGE_MAX_SIZE as c_int); 
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
pub fn remove_dead_connections(conns : &Vec<(TcpStream,String)>) -> Vec<TcpStream> {
    conns.into_iter().filter_map( |(x,_)| {
        match x.take_error() {
            Err(_) => None,
            Ok(Some(_)) => None,
            Ok(None) => Some(x.try_clone().ok()?),
        }
    }).collect()
}

/// Dissconnects from all active connections, waits 5 seconds and then ends
/// conns : The active connections
pub fn disconnect_all_connections(conns : &Vec<(TcpStream,String)>) {
    const TIMEOUT_TIMER : Duration = Duration::from_secs(5);
    let conns : Vec<(TcpStream,String)>  = conns.iter().filter_map( |(x,y)| {
        match x.try_clone().ok() {
            Some(x) => Some((x,y.clone())),
            None    => None,
        }
    }).collect();
    println!("\nDisconnecting from all connections and closeing");
    for (conn, name) in conns {
        let mut conn = Box::new(conn);
        std::thread::spawn( move || {
            log(&format!("disconnecting from {}@{:?}",name,conn.peer_addr()));
            send_message(&mut conn,Message::BYE,None);
            conn.shutdown(Shutdown::Both).unwrap_or(());
        });
    };
    std::thread::sleep(TIMEOUT_TIMER);
}

/// Removes a connection from a list of connections
/// conns : The active connections
/// to_remove : The connection to remove
pub fn remove_connection(conns : &mut Vec<(TcpStream,String)>, to_remove : &TcpStream) {
    let peer = to_remove.peer_addr().unwrap();
    *conns = conns
        .into_iter()
        .filter(|(x,_)| x.peer_addr().unwrap() != peer)
        .map(|(x,y)| (x.try_clone().unwrap(),y.clone())).collect::<Vec<_>>();
}


/// Sends a message from one user to all other users
/// conns : The current open connections.
/// me    : The address of the user sending the message.
/// nick  : The nickname of the user sending the message.
/// message : The message being sent.
pub fn blast_out(conns : &Vec<(TcpStream,String)>, me : &SocketAddr, nick : &String, message : &String) -> () {
    for (connection,name) in conns {
        let addr = connection.peer_addr().unwrap_or(*me);
        if addr != *me && *name != "".to_string() {
            log(&format!("{}@{}:`{}` -> {}@{}",nick,me,message,name,addr));
            let message = Message::CHAT(message.clone());
            send_message(&mut connection.try_clone().unwrap(), message, Some(nick.clone()));
        }
    };
}

/// Logs the input to a file and the screen, adds a time stamp
/// log_message : the string to log
pub fn log(log_message : &String) {
    let log_message = format!("{}\t{}\n",Utc::now(),log_message);
    let log_file_name : &Path = Path::new("logfile.log");
    print!("{}",log_message);
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
pub fn get_nickname(stream : &mut TcpStream, conns : &Arc<Mutex<Vec<(TcpStream,String)>>>) -> Option<String> {
    fn nicknames(conns : &Arc<Mutex<Vec<(TcpStream,String)>>>) -> Option<Vec<String>> {
        Some((*conns.lock().ok()?).iter().map( |(_,y)| y.clone()).collect())
    };
    while match rcv_message(stream) {
                Some(Message::NICK(n)) => {
                    log(&format!("Wants `{}` as nickname",n));
                    // if the nickname is not taken set add it to the list of nicknames in
                    // use and then return the nick
                    if !nicknames(conns)?.contains(&n.clone()) {
                        conns.lock().unwrap().push((stream.try_clone().unwrap(),n.clone()));
                        send_message(stream, Message::READY, None);
                        return Some(n);
                    } else {
                        // else ask the client to retry
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
