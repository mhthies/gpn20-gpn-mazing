use core::cmp::Ordering::Equal;
use core::option::Option;
use core::option::Option::{None, Some};
use crate::{AlgorithmConfig, helper, MoveDirection, Position, Walls};
use crate::client::{Answer, Command};
use rand::rngs::ThreadRng;
use std::collections::{HashMap, VecDeque};
use std::collections::HashSet;
use log::{debug, info, warn};
use crate::helper::{direction_from_move, distance_from_line, has_wall, move_by_direction};

#[derive(Default)]
pub struct State {
    last_pos: Option<Position>,
    start_pos: Option<Position>,
    current_pos: Option<Position>,
    current_goal: Option<Position>,
    current_walls: Option<Walls>,
    visited_positions: HashMap<Position, Option<Position>>,
    heuristics_stack: Vec<f32>,
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
        self.heuristics_stack.clear();
    }
}

pub fn decide_action(state: &mut State, rng: &mut ThreadRng, config: &AlgorithmConfig) -> Option<Command<'static>> {
    if let None = state.current_pos {
        return None;
    }

    let walls = state.current_walls.as_ref().unwrap();
    let pos = state.current_pos.as_ref().unwrap();
    let goal = state.current_goal.as_ref().unwrap();
    let start = state.start_pos.as_ref().unwrap();
    let size = Position{ x: start.y, y: start.y };

    let recent_minimal_heuristic = state.heuristics_stack.iter()
        .rev()
        .take(config.heuristic_decline_length as usize)
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(Equal))
        .unwrap_or(&f32::INFINITY);

    let valid_directions: Vec<(&MoveDirection, Position, bool, f32)> =
        [MoveDirection::Up, MoveDirection::Right, MoveDirection::Down, MoveDirection::Left]
        .iter()
        .filter(|d| {
            !has_wall(walls, d)
        })
        .map(|d| {
            let p = move_by_direction(pos, d);
            let (way, size_of_space) = explore_space_to_goal(&p, &size, &state.visited_positions, goal);
            debug!("Way length: {:?}, Size of Space: {}", way, size_of_space);
            let heuristic = calculate_position_heuristic(&p, goal, &size, way, size_of_space);
            (d, p, way.is_some(), heuristic)
        })
        .filter(|(_d, _pos, way, _heuristic)| { *way })
        .filter(|(_d, _pos, _way, heuristic)| *heuristic <= config.heuristic_cut)
        .filter(|(_d, _pos, _way, heuristic)| {
            let pass = *heuristic <= config.heuristic_decline_cut * recent_minimal_heuristic;
            if !pass { warn!("Cutting here, old heuristic is better: {} < {}.", recent_minimal_heuristic, heuristic) }
            pass
        })
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
            state.heuristics_stack.pop();
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
    debug!("All heuristics: [{}]",
        unvisited_valid_directions.iter()
        .map(|(_d, _pos, _way, heuristic)| {
            heuristic.to_string()
        })
        .collect::<Vec<String>>()
        .connect(","));
    state.heuristics_stack.push(unvisited_valid_directions[0].3);
    return Some(Command::Move(unvisited_valid_directions[0].0.clone()));
}

fn calculate_position_heuristic(pos: &Position, goal: &Position, size: &Position, potential_way_length: Option<u32>, size_of_space: u32) -> f32 {
    let playground_diagonal = ((size.x as f32).powi(2) + (size.y as f32).powi(2)).sqrt();
    let playground_center = Position{ x: size.x / 2, y: size.y / 2 };
    let offset_from_center_goal_line = distance_from_line(pos, &playground_center, goal) / playground_diagonal;
    let distance_to_goal = helper::calculate_distance(pos, goal) / playground_diagonal;
    let way_length = match potential_way_length {
        Some(len) => (len as f32 / (8.0 * size.x as f32 + 8.0 * size.y as f32)).sqrt(),
        None => 1.0,
    };
    debug!("goal: {}, way: {}, space: {}", distance_to_goal, way_length, size_of_space);

    0.5 * distance_to_goal
    + 0.5 * way_length / (size_of_space as f32).sqrt()
    + 0.1 * offset_from_center_goal_line
}


fn explore_space_to_goal(position: &Position, size: &Position, visited: &HashMap<Position, Option<Position>>, goal: &Position) -> (Option<u32>, u32) {
    let mut search_stack = VecDeque::from(vec![(position.clone(), 0)]);
    let mut search_set: HashSet<Position> = HashSet::new();

    while let Some((pos, count)) = search_stack.pop_front() {
        search_set.insert(pos.clone());
        if pos == *goal {
            let size_of_space = search_set.len() as u32;
            return (Some(count), size_of_space);
        }
        let search_positions = [
            MoveDirection::Up, MoveDirection::Right, MoveDirection::Down, MoveDirection::Left
        ].iter()
            .filter(|d| !helper::is_move_over_playground_border(&pos, d, size))
            .map(|d| helper::move_by_direction(&pos, &d))
            .filter(|p| !search_set.contains(p))
            .filter(|p| !visited.contains_key(p));
        for p in search_positions {
            search_stack.push_back((p, count + 1));
        }
    }
    let size_of_space = search_set.len() as u32;
    return (None, size_of_space);
}
