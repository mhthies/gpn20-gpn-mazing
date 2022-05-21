use core::cmp::Ordering::Equal;
use core::option::Option;
use core::option::Option::{None, Some};
use crate::{AlgorithmConfig, helper, MoveDirection, Position, Walls};
use crate::client::{Answer, Command};
use rand::rngs::ThreadRng;
use std::collections::HashMap;
use std::collections::HashSet;
use log::{debug, info};
use crate::helper::{direction_from_move, has_wall, move_by_direction};

#[derive(Default)]
pub struct State {
    last_pos: Option<Position>,
    start_pos: Option<Position>,
    current_pos: Option<Position>,
    current_goal: Option<Position>,
    current_walls: Option<Walls>,
    visited_positions: HashMap<Position, Option<Position>>,
}

impl State {
    pub fn update_from_answer(&mut self, answer: &Answer) {
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

pub fn decide_action(state: &State, rng: &mut ThreadRng, config: &AlgorithmConfig) -> Option<Command<'static>> {
    if let None = state.current_pos {
        return None;
    }

    let walls = state.current_walls.as_ref().unwrap();
    let pos = state.current_pos.as_ref().unwrap();
    let goal = state.current_goal.as_ref().unwrap();
    let start = state.start_pos.as_ref().unwrap();
    let size = Position{ x: start.y, y: start.y };

    let valid_directions: Vec<(&MoveDirection, Position, bool, f32)> =
        [MoveDirection::Up, MoveDirection::Right, MoveDirection::Down, MoveDirection::Left]
        .iter()
        .filter(|d| {
            !has_wall(walls, d)
        })
        .map(|d| {
            let p = move_by_direction(pos, d);
            let (way, size_of_space) = explore_space_for_goal(&p, &size, &state.visited_positions, goal);
            debug!("Way length: {:?}, Size of Space: {}", way, size_of_space);
            let heuristic = calculate_position_heuristic(&p, goal, &size, way, size_of_space);
            info!("Heuristic is {}", heuristic);
            (d, p, way.is_some(), heuristic)
        })
        .filter(|(_d, _pos, way, _heuristic)| { *way })
        .filter(|(_d, _pos, _way, heuristic)| *heuristic <= config.heuristic_cut)
        .collect();
    // debug!("Valid directions: {:?}", valid_directions);
    let mut unvisited_valid_directions:Vec<(&MoveDirection, Position, bool, f32)> = valid_directions.iter()
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

fn calculate_position_heuristic(pos: &Position, goal: &Position, size: &Position, potential_way_length: Option<u32>, size_of_space: u32) -> f32 {
    let playground_diagonal = ((size.x as f32).powi(2) + (size.y as f32).powi(2)).sqrt();
    let offset_from_diag = (pos.x as f32 - pos.y as f32).abs() / (playground_diagonal/2f32);
    let distance_to_goal = helper::calculate_distance(pos, goal) / playground_diagonal;
    let way_length = match potential_way_length {
        Some(len) => (len as f32 / (8.0 * size.x as f32 + 8.0 * size.y as f32)).sqrt(),
        None => 1.0,
    };
    debug!("goal: {}, way: {}, space: {}", distance_to_goal, way_length, size_of_space);

    0.5 * distance_to_goal + 0.5 * way_length / (size_of_space as f32).sqrt()
}


fn explore_space_for_goal(position: &Position, size: &Position, visited: &HashMap<Position, Option<Position>>, goal: &Position) -> (Option<u32>, u32) {
    let mut search_stack = vec![(position.clone(), 0)];
    let mut search_set: HashSet<Position> = HashSet::new();

    let mut way_to_goal: Option<u32> = None;
    while let Some((pos, count)) = search_stack.pop() {
        if pos == *goal {
            if way_to_goal.is_none() || way_to_goal.unwrap() > count + 1 {
                way_to_goal.replace(count + 1);
            }
        }
        search_set.insert(pos.clone());
        let search_positions = [
            MoveDirection::Up, MoveDirection::Right, MoveDirection::Down, MoveDirection::Left
        ].iter()
            .filter(|d| !helper::is_move_over_playground_border(&pos, d, size))
            .map(|d| helper::move_by_direction(&pos, &d))
            .filter(|p| !search_set.contains(p))
            .filter(|p| !visited.contains_key(p));
        for p in search_positions {
            search_stack.push((p, count + 1));
        }
    }
    let size_of_space = search_set.len() as u32;
    return (way_to_goal, size_of_space);
}
