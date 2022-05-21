use std::fs;
use std::io::{BufReader};
use log::{error};
use gpn20_maze::{Config, MoveDirection, Position, Walls};

mod helper;

fn main() {
    env_logger::init();

    let config_file = "config.toml";
    let config_string = fs::read_to_string(config_file).unwrap();
    let config: Config = toml::from_str(&config_string).unwrap();
    let mut rng = rand::thread_rng();
    loop {
        let mut stream = gpn20_maze::get_connection(&config.server);
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let res = gpn20_maze::game_loop(&config, &mut stream, &mut reader, &mut rng);
        if let Err(e) = res {
            error!("IO error: {}", e);
        }
    }
}
