use std::cmp::Ordering::Equal;
use std::collections::{HashMap, HashSet};
use std::{fs, thread, time};
use std::fs::read;
use std::io::{BufRead, BufReader, Write};
use std::io;
use std::net::TcpStream;
use log::{debug, error, info, warn};
use serde::Deserialize;
use rand::Rng;
use rand::rngs::ThreadRng;

#[derive(Deserialize)]
struct ServerConfig {
    address: String,
}

#[derive(Deserialize)]
struct UserConfig {
    user: String,
    password: String,
}

#[derive(Deserialize)]
struct AlgorithmConfig {
    heuristic_cut: f32,
}

#[derive(Deserialize)]
struct Config {
    server: ServerConfig,
    user: UserConfig,
    algorithm: AlgorithmConfig,
}

fn get_connection(config: &ServerConfig) -> TcpStream {
    loop {
        match TcpStream::connect(&config.address) {
            Ok(s) => return s,
            Err(e) => error!("Could not connect: {}", e)
        }
        thread::sleep(time::Duration::from_millis(200));
    }
}

fn main() {
    env_logger::init();

    let config_file = "config.toml";
    let config_string = fs::read_to_string(config_file).unwrap();
    let config: Config = toml::from_str(&config_string).unwrap();
    let mut rng = rand::thread_rng();
    loop {
        let mut stream = get_connection(&config.server);
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        send_command(&mut stream, &Command::Join(&config.user.user, &config.user.password));
        let res = game_loop(&config.algorithm, &mut stream, &mut reader, &mut rng);
        if let Err(e) = res {
            error!("IO error: {}", e);
        }
    }
}

fn game_loop(config: &AlgorithmConfig, stream: &mut TcpStream, stream_reader: &mut BufReader<TcpStream>, rng: &mut ThreadRng) -> io::Result<()> {
    let mut state = State::default();
    info!("Starting game loop.");
    loop {
        if let Some(answer) = get_answer(stream_reader)? {
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
        if let Some(command) = decide_action(&state, rng, config) {
            send_command(stream, &command)?;
        }
    }
}

#[derive(Default,Clone,Eq,PartialEq,Hash)]
struct Position {
    x: u32,
    y: u32,
}

#[derive(Default,Clone,Debug)]
struct Walls {
    top: bool,
    right: bool,
    bottom: bool,
    left: bool,
}

#[derive(Clone, Debug)]
enum MoveDirection {
    Up,
    Right,
    Down,
    Left,
}

enum Command<'a> {
    Join(&'a str, &'a str),
    Move(MoveDirection),
    Chat(&'a str),
}

enum Answer {
    Motd(String),
    Error(String),
    Goal(Position),
    Pos(Position, Walls),
    Win(u32, u32),
    Lose(u32, u32),
}

fn get_answer(reader: &mut BufReader<TcpStream>) -> io::Result<Option<Answer>> {
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
        "" => { None },
        x => {
            warn!("Unkown message from server: {}", x);
            None
        }
    })
}

fn send_command(stream: &mut TcpStream, command: &Command) -> io::Result<()> {
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


#[derive(Default)]
struct State {
    last_pos: Option<Position>,
    start_pos: Option<Position>,
    current_pos: Option<Position>,
    current_goal: Option<Position>,
    current_walls: Option<Walls>,
    visited_positions: HashMap<Position, Option<Position>>,
}

impl State {
    fn update_from_answer(&mut self, answer: &Answer) {
        match answer {
            Answer::Pos(p, w) => {
                if self.current_pos.is_none() {
                    self.start_pos = Some(p.clone());
                };
                if self.current_pos.is_none() || p != self.current_pos.as_ref().unwrap() {
                    if !self.visited_positions.contains_key(p) {
                        self.visited_positions.insert(p.clone(), self.current_pos.clone());
                    }
                    self.last_pos = self.current_pos.take();
                    self.current_pos = Some(p.clone());
                    self.current_walls = Some(w.clone());
                    debug!("Current walls: {:?}", self.current_walls.as_ref().unwrap());
                }
            },
            Answer::Goal(p) => {
                self.reset();
                self.current_goal.replace(p.clone());
            },
            _ => {},
        }
    }

    fn reset(&mut self) {
        self.current_pos = None;
        self.last_pos = None;
        self.current_walls = None;
        self.current_goal = None;
        self.visited_positions.clear();
    }
}

fn decide_action(state: &State, rng: &mut ThreadRng, config: &AlgorithmConfig) -> Option<Command<'static>> {
    if let None = state.current_pos {
        return None;
    }

    let walls = state.current_walls.as_ref().unwrap();
    let pos = state.current_pos.as_ref().unwrap();
    let goal = state.current_goal.as_ref().unwrap();
    let start = state.start_pos.as_ref().unwrap();
    let size = Position{ x: start.y, y: start.y };

    let valid_directions: Vec<(&MoveDirection, Position, Option<u32>, f32)> =
        [MoveDirection::Up, MoveDirection::Right, MoveDirection::Down, MoveDirection::Left]
        .iter()
        .filter(|d| {
            !has_wall(walls, d)
        })
        .map(|d| {
            let p = move_by_direction(pos, d);
            let way = possible_way_to_goal(&p, &size, &state.visited_positions, goal);
            info!("Way length: {:?}", way);
            let heuristic = calculate_position_heuristic(&p, goal, &size, way);
            (d, p, way, heuristic)
        })
        .filter(|(_d, _pos, way, _heuristic)| {
            way.is_some()
        })
        .filter(|(_d, _pos, _way, heuristic)| *heuristic <= config.heuristic_cut)
        .collect();
    // debug!("Valid directions: {:?}", valid_directions);
    let mut unvisited_valid_directions:Vec<(&MoveDirection, Position, Option<u32>, f32)> = valid_directions.iter()
        .filter(|(_d, pos, _way, _heuristic)| {
            !state.visited_positions.contains_key(pos)
        })
        .map(|d| d.clone())
        .collect();
    // debug!("Unvisited directions: {:?}", unvisited_valid_directions);
    if unvisited_valid_directions.is_empty() {
        let back = state.visited_positions.get(pos).unwrap();
        if let Some(back_pos) = back {
            info!("Stepping backwards");
            return Some(Command::Move(direction_from_move(pos, &back_pos)));
        } else {
            info!("No way for stepping backwards. Game seems to be over?");
            return None;
        }
    }

    unvisited_valid_directions.sort_by(|a, b| {
        a.3.partial_cmp(&b.3).unwrap_or(Equal)
    });
    info!("Heuristic: {}", unvisited_valid_directions[0].3);

    return Some(Command::Move(unvisited_valid_directions.into_iter().next().unwrap().0.clone()));
}

fn move_by_direction(pos: &Position, dir: &MoveDirection) -> Position {
    match dir {
        MoveDirection::Up => Position { x: pos.x, y: pos.y-1 },
        MoveDirection::Right => Position { x: pos.x+1, y: pos.y },
        MoveDirection::Down => Position { x: pos.x, y: pos.y+1 },
        MoveDirection::Left => Position { x: pos.x-1, y: pos.y },
    }
}

fn is_move_over_playground_border(pos: &Position, dir: &MoveDirection, size: &Position) -> bool {
    match dir {
        MoveDirection::Up => pos.y == 0,
        MoveDirection::Right => pos.x >= size.x,
        MoveDirection::Down => pos.y >= size.y,
        MoveDirection::Left => pos.x == 0,
    }
}

fn direction_from_move(pos1: &Position, pos2: &Position) -> MoveDirection {
    if pos2.x > pos1.x {
        MoveDirection::Right
    } else if pos2.x < pos1.x {
        MoveDirection::Left
    } else if pos2.y < pos1.y {
        MoveDirection::Up
    } else if pos2.y > pos1.y {
        MoveDirection::Down
    } else {
        panic!();
    }
}

fn has_wall(walls: &Walls, dir: &MoveDirection) -> bool {
    match dir {
        MoveDirection::Up => walls.top,
        MoveDirection::Right => walls.right,
        MoveDirection::Down => walls.bottom,
        MoveDirection::Left => walls.left,
    }
}

fn calculate_distance(pos1: &Position, pos2: &Position) -> f32 {
    ((pos1.x as f32 - pos2.x as f32).powi(2) + (pos1.y as f32 - pos2.y as f32).powi(2)).sqrt()
}

fn calculate_position_heuristic(pos: &Position, goal: &Position, size: &Position, potential_way_length: Option<u32>) -> f32 {
    let playground_diagonal = ((size.x as f32).powi(2) + (size.y as f32).powi(2)).sqrt();
    let offset_from_diag = (pos.x as f32 - pos.y as f32).abs() / (playground_diagonal/2f32);
    let distance_to_goal = calculate_distance(pos, goal) / playground_diagonal;
    let way_length = match potential_way_length {
        Some(len) => (len as f32 / (8.0 * size.x as f32 + 8.0 * size.y as f32)).sqrt(),
        None => 1.0,
    };
    info!("Goal: {}, Diag: {}, way: {}", offset_from_diag, distance_to_goal, way_length);

    distance_to_goal * 0.5 + way_length * 0.5
}


fn possible_way_to_goal(position: &Position, size: &Position, visited: &HashMap<Position, Option<Position>>, goal: &Position) -> Option<u32> {
    if position == goal {
        return Some(0);
    }
    let mut search_stack = vec![(position.clone(), 0)];
    let mut search_set: HashSet<Position> = HashSet::new();

    while let Some((pos, count)) = search_stack.pop() {
        search_set.insert(pos.clone());
        let search_positions = [
            MoveDirection::Up, MoveDirection::Right, MoveDirection::Down, MoveDirection::Left
        ].iter()
            .filter(|d| !is_move_over_playground_border(&pos, d, size))
            .map(|d| move_by_direction(&pos, &d))
            .filter(|p| !search_set.contains(p))
            .filter(|p| !visited.contains_key(p));
        for pos in search_positions {
            if pos == *goal {
                return Some(count + 1);
            }
            search_stack.push((pos, count + 1));
        }
    }
    return None;
}
