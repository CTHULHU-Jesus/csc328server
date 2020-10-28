mod lib;
use std::net::*;
use std::sync::*;
use crate::lib::*;
use std::io::{Read,Write};

/// Sends a message from one user to all other users
/// conns : The current open connections.
/// me    : The address of the user sending the message.
/// nick  : The nickname of the user sending the message.
/// message : The message being sent.
fn blast_out(conns : &Vec<TcpStream>, me : &SocketAddr, nick : &String, message : &String) -> () {
    //@TODO this
}

/// Gets a valid nickname from the user 
/// and adds the nickname to the list of nicknames in use
/// stream : the tcpstream connected to the user
/// nicknames : the global list of all nicknames
/// returns : a valid nickname
fn getNickname(stream : &TcpStream, nicknames : Arc<Mutex<Vec<String>>>) -> String {
    let mut message = [0;1024];
    let nick : String;
    while match stream.read(&mut message) {
        Ok(size) => {
            let message : Message =
                std::str::FromStr::from_str(
                std::str::from_utf8(
                    &message[0..size]).unwrap()
                )
                .unwrap();
            match message {
                Message::NICK(n) => true, //@TODO FINISH THIS
                _ => true,
            }
        },
        Err(_) => true
    } {};
    nick;
}


fn main() {
    // start listening for connections
    // @TODO change portnumber to come from a command line argument
    let portnumber = 1337; //1337,8008,42069
    let listener = TcpListener::bind(format!("0.0.0.0:{}",portnumber)).unwrap();
    let connections : Arc<Mutex<Vec<TcpStream>>> = Arc::new(Mutex::new(Vec::new()));
    let nicknames : Arc<Mutex<Vec<String>>>   = Arc::new(Mutex::new(Vec::new()));
    for stream in listener.incoming() {
        let stream = Box::new(stream);
        let connections = connections.clone();
        let mut message : [u8;1024] = [0;1024];
        // spin up thread to handel each client
        std::thread::spawn( move || {
            //Start
            let mut stream = (*stream).unwrap();
            (*connections.lock().unwrap()).push(stream.try_clone().unwrap());
            // Send HELLO Message 
            stream.write(&Message::HELLO.to_string().as_bytes()).unwrap();
            // Handel Nick Message
            let nick : String = getNickname(&stream,nicknames);

            // Wait for messages
            while match stream.read(&mut message) {
                Ok(size) => {
                    let message : Message =
                        std::str::FromStr::from_str(
                        std::str::from_utf8(
                            &message[0..size]).unwrap()
                        )
                        .unwrap();
                    match message {
                        Message::CHAT(x) => {
                            blast_out(&connections.lock().unwrap(),&stream.peer_addr().unwrap(),&nick);
                            true
                        }
                        Message::BYE => false,
                        _ => true,
                    }
                },
                Err(_) => true,
            } {
                // clears buffer
                message = [0;1024];
            };

            //End
            //remove nickname from list of nicknames in use nicknames
            *nicknames.lock().unwrap() = nicknames.lock().unwrap()
                .iter()
                .filter( |x| **x != nick )
                .map( |x| *x )
                .collect();
            //remove connection from list of connections in use
            let peer = stream.peer_addr().unwrap();
            *connections.lock().unwrap() = (*connections.lock().unwrap())
                .iter()
                .filter(|x| x.peer_addr().unwrap() != peer)
                .map(|x| x.try_clone().unwrap()).collect();
        });
    }
}
