mod lib;
use std::net::*;
use std::sync::*;
use crate::lib::*;
use std::time::Duration;


fn main() {
    // start listening for connections
    const USAGE : &str = "cargo run [port number]";
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
            send_message(&mut stream,Message::HELLO, None);
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
            while match rcv_message(&mut stream) {
                // On CHAT blast it out to all connected users
                Some(Message::CHAT(x)) => {
                    blast_out(&connections.lock().unwrap(),&stream.peer_addr().unwrap(),&nick,&x);
                    log(&format!("{}@{}:`{}`",nick,conn_name,x));
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
            //remove nickname from list of nicknames in use nicknames
            remove_nickname(&mut nicknames.lock().unwrap(),&nick);
            //remove connection from list of connections in use
            remove_connection(&mut connections.lock().unwrap(),&stream);
            stream.shutdown(Shutdown::Both).expect("Could not shutdown connection");
        });
    }
}
