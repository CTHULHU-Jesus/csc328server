mod lib;
use std::net::*;
use std::sync::*;
use crate::lib::*;
use std::io::{Read,Write};
use std::time::Duration;


fn main() {
    // start listening for connections
    const USAGE : &str = "cargo run [port number]";
    const MESSAGE_MAX_SIZE : usize = 4000;
    let portnumber = std::env::args().nth(1).unwrap_or("1337".to_string()).parse::<u32>().expect(USAGE); //1337,8008,42069
    let listener = TcpListener::bind(format!("0.0.0.0:{}",portnumber)).expect("Could not bind to desired port number");
    let connections : Arc<Mutex<Vec<TcpStream>>> = Arc::new(Mutex::new(Vec::new()));
    let nicknames : Arc<Mutex<Vec<String>>>   = Arc::new(Mutex::new(Vec::new()));
    log(&"Server starts".to_string());
    // set up signal handler for ^C
    {
        let connections = connections.clone();
        ctrlc::set_handler( move || {
            match connections.lock() {
                Ok(x) => {
                    lib::disconnect_all_connections(&x);
                }
                Err(_) => {}
            }
            log(&"Server shuting down".to_string());
            std::process::exit(0);
        }).expect("Error seting Ctrl-C handler");
    }
    // set up thread to clean mutex every so often
    // @TODO see if needed
    {
        let connections = connections.clone();
        const SLEEP_CLEAN_TIME : Duration = Duration::from_secs(5*60); // 5 minutes 
        std::thread::spawn(move || {
            std::thread::sleep(SLEEP_CLEAN_TIME);
            remove_dead_connections(&connections.lock().unwrap());
        });
    }
    for stream in listener.incoming() {
        let stream = Box::new(stream);
        let connections = connections.clone();
        let nicknames = nicknames.clone();
        // spin up thread to handle each client
        std::thread::spawn( move || {
            //Start
            let mut stream = (*stream).unwrap();
            let conn_name = match stream.peer_addr().ok() {
                Some(x) => {format!("{:?}",x)},
                None    => "Err_get_addr".to_string()
            };

            (*connections.lock().unwrap()).push(stream.try_clone().unwrap());

            // Send HELLO Message 
            stream.write(&Message::HELLO.to_string().as_bytes()).unwrap();
            log(&format!("Start connection with {}",conn_name));

            // Handle Nick Message
            let nick : String = match get_nickname(&mut stream,&nicknames) {
                Some(s) => s,
                None    => {
                    // None means that the client wants to disconnect
                    remove_connection(&mut connections.lock().unwrap(),&stream);
                    stream.shutdown(Shutdown::Both).expect("Could not shutdown connection");
                    std::process::exit(0);
                }
            };
            log(&format!("{} has the nickname `{}`",conn_name,nick));

            // Wait for messages
            let mut message = [0u8;MESSAGE_MAX_SIZE];
            while match stream.read(&mut message) {
                Ok(size) => {
                    let message : Message =
                        std::str::FromStr::from_str(
                        std::str::from_utf8(&message[0..size])
                        .unwrap_or(""))
                        .unwrap_or(Message::BYE);
                    match message {
                        // On CHAT blast it out to all connected users
                        Message::CHAT(x) => {
                            blast_out(&connections.lock().unwrap(),&stream.peer_addr().unwrap(),&nick,&x);
                            log(&format!("{}@{}:`{}`",nick,conn_name,x));
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
            log(&format!("Ending connection with {}@{}",nick,conn_name));
            //remove nickname from list of nicknames in use nicknames
            remove_nickname(&mut nicknames.lock().unwrap(),&nick);
            //remove connection from list of connections in use
            remove_connection(&mut connections.lock().unwrap(),&stream);
            stream.shutdown(Shutdown::Both).expect("Could not shutdown connection");
        });
    }
}
