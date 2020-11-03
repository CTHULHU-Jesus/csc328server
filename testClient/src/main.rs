use std::io::{self, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::Duration;

const MSG_SIZE: usize = 1024;
const USAGE : &str = "Usage: cargo run [server ip:portnumber]";

fn main() {
    let ip = std::env::args().nth(1).expect(USAGE);
    let mut client = TcpStream::connect(ip).expect("Stream failed to connect");
    client.set_nonblocking(true).expect("failed to initiate non-blocking");

    let (tx, rx) = mpsc::channel::<String>();

    thread::spawn(move || loop {
        let mut buff = vec![0; MSG_SIZE];
        match client.read(&mut buff) {
            Ok(size) if size > 0 => {
                let msg = std::str::from_utf8(&buff[..size]).unwrap();
                println!("SERVER: {:?}", msg);
            },
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
            _ => {
                println!("connection with server was severed");
                break;
            }
        }

        match rx.try_recv() {
            Ok(msg) => {
                let buff = msg.clone().into_bytes();
                client.write(&buff).expect("writing to socket failed");
                println!("message sent {:?}", msg);
            }, 
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => break
        }

        thread::sleep(Duration::from_millis(100));
    });

    loop {
        let mut buff = String::new();
        io::stdin().read_line(&mut buff).expect("reading from stdin failed");
        let msg = buff.trim().to_string();
        if msg == ":quit" || tx.send(msg).is_err() {break}
    }
    println!("bye bye!");

}
