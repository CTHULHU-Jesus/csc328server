use std::str::FromStr;
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

#[derive(Clone,PartialEq,Eq)]
pub struct ParseMessageError(String);

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
            _ if NICK.is_match(s)  => Ok(Message::NICK("".to_string())),
            _ if BYE.is_match(s)   => Ok(Message::BYE),
            _ if READY.is_match(s) => Ok(Message::READY),
            _ if RETRY.is_match(s) => Ok(Message::RETRY),
            _ if CHAT.is_match(s)  => Ok(Message::CHAT("".to_string())),
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
