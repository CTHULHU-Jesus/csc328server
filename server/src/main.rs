mod lib;
use std::net::*;
use std::sync::*;
use crate::lib::*;
use std::io::{Read,Write};
use std::time::Duration;
use std::fs::File;
use std::path::Path;
use chrono::Utc;

/// Dissconnects from all active connections, waits 5 seconds and then ends
/// conns : The active connections
fn disconnect_all_connections(conns : Vec<TcpStream>) {
    const TIMEOUT_TIMER : Duration = Duration::from_secs(5);
    // let mut conns : Vec<TcpStream>  = conns.clone();
    println!("\nDisconnecting from all connections and closeing");
    for conn in conns {
        let conn = Box::new(conn);
        std::thread::spawn( move || {
            //@TODO tell client to shutdown
            conn.shutdown(Shutdown::Both).unwrap();
        });
    };
    std::thread::sleep(TIMEOUT_TIMER);
}

/// Removes a connection from a list of connections
/// conns : The active connections
/// to_remove : The connection to remove
fn remove_connection(conns : &mut Vec<TcpStream>, to_remove : &TcpStream) {
    let peer = to_remove.peer_addr().unwrap();
    *conns = conns
        .into_iter()
        .filter(|x| x.peer_addr().unwrap() != peer)
        .map(|x| x.try_clone().unwrap()).collect::<Vec<_>>();
}

/// Removes a nickname from the list of active nicknames
/// nicknames : The list of active nicknames
/// to_remove : The nickname to be removed
fn remove_nickname(nicknames : &mut Vec<String>, to_remove : &String) {
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
fn blast_out(conns : &Vec<TcpStream>, me : &SocketAddr, nick : &String, message : &String) -> () {
    let log_file_name : &Path = Path::new("logfile.txt");
    for mut connection in conns {
        if connection.peer_addr().unwrap() != *me {
            connection.write(
                Message::CHAT(format!("{}:{}",nick,message))
                .to_string()
                .as_bytes()
            ).unwrap_or(0);
        }
    };
    // @TODO add timestamp to log output
    let log = format!("{}\t{}:`{}`\n",Utc::now(),nick,message);
    print!("{}",log);
    std::fs::OpenOptions::new()
        .append(true)
        .open(log_file_name)
        .unwrap_or(File::create(log_file_name).unwrap())
        .write(log.as_bytes()).unwrap();
}

/// Gets a valid nickname from the user 
/// and adds the nickname to the list of nicknames in use
/// stream : the tcpstream connected to the user
/// nicknames : the global list of all nicknames
/// returns : a valid nickname or None if the user wants to end the connection
fn get_nickname(stream : &mut TcpStream, nicknames : &Arc<Mutex<Vec<String>>>) -> Option<String> {
    const NAME_MAX_SIZE : usize = 32;
    let mut message = [0;NAME_MAX_SIZE];
    while match stream.read(&mut message) {
        Ok(size) => {
            let message : Message =
                std::str::FromStr::from_str(
                std::str::from_utf8(
                    &message[0..size]).unwrap()
                )
                .unwrap();
            match message {
                Message::NICK(n) => {
                    // if the nickname is not taken set add it to the list of nicknames in
                    // use and then return the nick
                    if !nicknames.lock().unwrap().contains(&n.clone()) {
                        nicknames.lock().unwrap().push(n.clone());
                        stream.write(Message::READY.to_string().as_bytes()).unwrap();
                        return Some(n);
                    } else {
                        // else ask the client to retry
                        stream.write(Message::RETRY.to_string().as_bytes()).unwrap();
                        true
                    }
                }
                Message::BYE => return None,
                _ => true,
            }
        },
        Err(_) => true
    } {
        message = [0;NAME_MAX_SIZE];
    };
    None
}


fn main() {
    // start listening for connections
    const USAGE : &str = "cargo run [port number]";
    const MESSAGE_MAX_SIZE : usize = 4000;
    let portnumber = std::env::args().nth(1).unwrap_or("1337".to_string()).parse::<u32>().expect(USAGE); //1337,8008,42069
    let listener = TcpListener::bind(format!("0.0.0.0:{}",portnumber)).expect("Could not bind to desired port number");
    let connections : Arc<Mutex<Vec<TcpStream>>> = Arc::new(Mutex::new(Vec::new()));
    let nicknames : Arc<Mutex<Vec<String>>>   = Arc::new(Mutex::new(Vec::new()));
    // set up signal handler for ^C
    {
        let connections = connections.clone();
        ctrlc::set_handler( move || {
            disconnect_all_connections((*connections.lock().unwrap()).iter().map( |x| x.try_clone().unwrap() ).collect());
            std::process::exit(0);
        }).expect("Error seting Ctrl-C handler");
    }
    for stream in listener.incoming() {
        let stream = Box::new(stream);
        let connections = connections.clone();
        let nicknames = nicknames.clone();
        // spin up thread to handle each client
        std::thread::spawn( move || {
            //Start
            let mut stream = (*stream).unwrap();
            (*connections.lock().unwrap()).push(stream.try_clone().unwrap());

            // Send HELLO Message 
            stream.write(&Message::HELLO.to_string().as_bytes()).unwrap();

            // Handel Nick Message
            let nick : String = match get_nickname(&mut stream,&nicknames) {
                Some(s) => s,
                None    => {
                    // None means that the client wants to disconnect
                    remove_connection(&mut connections.lock().unwrap(),&stream);
                    stream.shutdown(Shutdown::Both).expect("Could not shutdown connection");
                    std::process::exit(0);
                }
            };

            // Wait for messages
            let mut message = [0u8;MESSAGE_MAX_SIZE];
            while match stream.read(&mut message) {
                Ok(size) => {
                    let message : Message =
                        std::str::FromStr::from_str(
                        std::str::from_utf8(&message[0..size])
                        .unwrap())
                        .unwrap();
                    match message {
                        // On CHAT blast it out to all connected users
                        Message::CHAT(x) => {
                            blast_out(&connections.lock().unwrap(),&stream.peer_addr().unwrap(),&nick,&x);
                            true
                        }
                        // on BYE exit loop
                        Message::BYE => false,
                        // Do not process any other messages, but do loop back
                        _ => true,
                    }
                },
                // On Error do nothing but loop back
                // @TODO decide whether or not to just exit from the program
                Err(_) => true,
            } {
                // clears buffer
                message = [0;MESSAGE_MAX_SIZE];
            };

            //End
            //remove nickname from list of nicknames in use nicknames
            remove_nickname(&mut nicknames.lock().unwrap(),&nick);
            //remove connection from list of connections in use
            remove_connection(&mut connections.lock().unwrap(),&stream);
            stream.shutdown(Shutdown::Both).expect("Could not shutdown connection");
        });
    }
}
