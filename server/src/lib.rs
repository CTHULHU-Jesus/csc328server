use std::str::{FromStr};
use std::string::ToString;
use regex::Regex;
use lazy_static::*;

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

