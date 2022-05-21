use std::net::TcpStream;
use std::{io, thread, time};
use std::io::BufReader;
use log::{error, info, warn};
use rand::prelude::ThreadRng;
use crate::algorithm::{decide_action, State};
use crate::client::{Answer, Command, send_command};
use serde::Deserialize;

mod client;
mod algorithm;
mod helper;

#[derive(Default,Clone,Eq,PartialEq,Hash)]
pub struct Position {
    pub x: u32,
    pub y: u32,
}

#[derive(Default,Clone,Debug)]
pub struct Walls {
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
}

#[derive(Clone, Debug)]
pub enum MoveDirection {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub address: String,
}

#[derive(Deserialize)]
struct UserConfig {
    user: String,
    password: String,
}

#[derive(Deserialize)]
pub struct AlgorithmConfig {
    heuristic_cut: f32,
}

#[derive(Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    user: UserConfig,
    algorithm: AlgorithmConfig,
}

pub fn get_connection(config: &ServerConfig) -> TcpStream {
    loop {
        match TcpStream::connect(&config.address) {
            Ok(s) => return s,
            Err(e) => error!("Could not connect: {}", e)
        }
        thread::sleep(time::Duration::from_millis(200));
    }
}

pub fn game_loop(config: &Config, stream: &mut TcpStream, stream_reader: &mut BufReader<TcpStream>, rng: &mut ThreadRng) -> io::Result<()> {
    let mut state = State::default();
    info!("Joining game as {}", config.user.user);
    send_command(stream, &Command::Join(&config.user.user, &config.user.password))?;
    info!("Starting game loop.");
    loop {
        if let Some(answer) = client::get_answer(stream_reader)? {
            match answer {
                Answer::Motd(msg) => {
                    warn!("Message of the day: {}", msg);
                },
                Answer::Error(msg) => {
                    warn!("Error from Server: {}", msg);
                },
                _ => {
                    state.update_from_answer(&answer);
                },
            }
        }
        if let Some(command) = decide_action(&state, rng, &config.algorithm) {
            client::send_command(stream, &command)?;
        }
    }
}
