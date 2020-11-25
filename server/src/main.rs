//   Authors:        Matthew Bartlett              & Arron Harman
//   Major:          (Software Development & Math) & (Software Development)
//   Creation Date:  October  27, 2020
//   Due Date:       November 24, 2020
//   Course:         CSC328
//   Professor Name: Dr. Frye
//   Assignment:     Chat Server
//   Filename:       main.rs
//   Purpose:        Use commands documented in lib.rs to make a chat server
mod lib;
use std::net::*;
use std::sync::*;
use crate::lib::*;
use std::time::Duration;


fn main() {
    // Manage the command line args
    const USAGE : &str = "cargo run [port number]";
    let portnumber = std::env::args().nth(1).unwrap_or("1337".to_string()).parse::<u32>().expect(USAGE); //1337,8008,42069
    // start listening for connections
    let listener = TcpListener::bind(format!("0.0.0.0:{}",portnumber)).expect("Could not bind to desired port number");
    let connections : Arc<Mutex<Vec<(TcpStream,String)>>> = Arc::new(Mutex::new(Vec::new()));
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
    {
        let connections = connections.clone();
        const SLEEP_CLEAN_TIME : Duration = Duration::from_secs(60); // 1 minute
        std::thread::spawn(move || {
            std::thread::sleep(SLEEP_CLEAN_TIME);
            remove_dead_connections(&connections.lock().unwrap());
        });
    }
    for stream in listener.incoming() {
        let stream = Box::new(stream);
        let connections = connections.clone();
        // spin up thread to handle each client
        std::thread::spawn( move || {
            //Start
            let mut stream = (*stream).unwrap();
            let conn_name = match stream.peer_addr().ok() {
                Some(x) => {format!("{:?}",x)},
                None    => "Err_get_addr".to_string()
            };

            // add stream to mutex
            connections.lock().unwrap().push((stream.try_clone().unwrap(),"".to_string()));

            // Send HELLO Message 
            send_message(&mut stream,Message::HELLO, None);
            log(&format!("Start connection with {}",conn_name));

            // Handle Nick Message
            let nick : String = match get_nickname(&mut stream,&connections) {
                Some(s) => s,
                None    => {
                    // None means that the client wants to disconnect
                    remove_connection(&mut connections.lock().unwrap(),&stream);
                    stream.shutdown(Shutdown::Both).expect("Could not shutdown connection");
                    return ();
                }
            };
            log(&format!("{} has the nickname `{}`",conn_name,nick));

            // Wait for messages
            while match rcv_message(&mut stream) {
                // On CHAT blast it out to all connected users
                Some(Message::CHAT(x)) => {
                    let x = x.trim().to_string().replace('\r',"");
                    log(&format!("Blast out {}@{}:`{}`",nick,conn_name,x));
                    blast_out(&connections.lock().unwrap(),&stream.peer_addr().unwrap(),&nick,&x);
                    true
                }
                // on BYE exit loop
                Some(Message::BYE) => false,
                // on Error exit loop
                None => false,
                // Do not process any other messages, but do loop back
                _ => true,
      
            } {}
            
            //End
            log(&format!("Ending connection with {}@{}",nick,conn_name));
            //remove connection from list of connections in use
            remove_connection(&mut connections.lock().unwrap(),&stream);
            stream.shutdown(Shutdown::Both).expect("Could not shutdown connection");
        });
    }
}
