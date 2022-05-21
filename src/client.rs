use std::net::TcpStream;
use std::io;
use std::io::{BufRead, BufReader, Write};
use log::{debug, warn};
use crate::{MoveDirection, Position, Walls};

pub enum Command<'a> {
    Join(&'a str, &'a str),
    Move(MoveDirection),
    Chat(&'a str),
}

pub enum Answer {
    Motd(String),
    Error(String),
    Goal(Position),
    Pos(Position, Walls),
    Win(u32, u32),
    Lose(u32, u32),
    Game(Position, Position),
}

pub fn get_answer(reader: &mut BufReader<TcpStream>) -> io::Result<Option<Answer>> {
    let mut command = String::new();
    reader.read_line(&mut command)?;
    debug!("Received answer: {}", command.trim());
    let mut parts = command.trim().split("|");
    Ok(match parts.next().unwrap() {
        "motd" => { Some(Answer::Motd(parts.next().unwrap_or("").to_owned())) },
        "error" => { Some(Answer::Error(parts.next().unwrap_or("").to_owned())) },
        "goal" => { Some(Answer::Goal(Position {
            x: parts.next().unwrap_or("0").parse().unwrap_or(0),
            y: parts.next().unwrap_or("0").parse().unwrap_or(0)
        })) },
        "pos" => {
            Some(Answer::Pos(
                Position {
                    x: parts.next().unwrap_or("0").parse().unwrap_or(0),
                    y: parts.next().unwrap_or("0").parse().unwrap_or(0)
                },
                Walls{
                    top: parts.next().unwrap_or("0") == "1",
                    right: parts.next().unwrap_or("0") == "1",
                    bottom: parts.next().unwrap_or("0") == "1",
                    left: parts.next().unwrap_or("0") == "1",
                }
            ))
        },
        "win" => {
            Some(Answer::Win(parts.next().unwrap_or("0").parse().unwrap_or(0),
                             parts.next().unwrap_or("0").parse().unwrap_or(0)))
        },
        "lose" => {
            Some(Answer::Lose(parts.next().unwrap_or("0").parse().unwrap_or(0),
                              parts.next().unwrap_or("0").parse().unwrap_or(0)))
        },
        "game" => {
            Some(Answer::Game(
                Position {
                    x: parts.next().unwrap_or("0").parse().unwrap_or(0),
                    y: parts.next().unwrap_or("0").parse().unwrap_or(0)
                },
                Position {
                    x: parts.next().unwrap_or("0").parse().unwrap_or(0),
                    y: parts.next().unwrap_or("0").parse().unwrap_or(0)
                },
            ))
        },
        "" => { None },
        x => {
            warn!("Unkown message from server: {}", x);
            None
        }
    })
}

pub fn send_command(stream: &mut TcpStream, command: &Command) -> io::Result<()> {
    let data = match command {
        Command::Join(user, password) => format!("join|{}|{}\n", user, password),
        Command::Move(direction) => format!("move|{}\n", match direction {
            MoveDirection::Up => "up",
            MoveDirection::Right => "right",
            MoveDirection::Down => "down",
            MoveDirection::Left => "left",
        }),
        Command::Chat(msg) => format!("chat|{}\n", msg),
    };
    debug!("Sending command: {}", data.trim());
    stream.write(data.as_bytes())?;
    stream.flush()?;
    Ok(())
}
