use crate::{MoveDirection, Position, Walls};

pub fn move_by_direction(pos: &Position, dir: &MoveDirection) -> Position {
    match dir {
        MoveDirection::Up => Position { x: pos.x, y: pos.y-1 },
        MoveDirection::Right => Position { x: pos.x+1, y: pos.y },
        MoveDirection::Down => Position { x: pos.x, y: pos.y+1 },
        MoveDirection::Left => Position { x: pos.x-1, y: pos.y },
    }
}

pub fn is_move_over_playground_border(pos: &Position, dir: &MoveDirection, size: &Position) -> bool {
    match dir {
        MoveDirection::Up => pos.y == 0,
        MoveDirection::Right => pos.x >= size.x,
        MoveDirection::Down => pos.y >= size.y,
        MoveDirection::Left => pos.x == 0,
    }
}

pub fn direction_from_move(pos1: &Position, pos2: &Position) -> MoveDirection {
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

pub fn has_wall(walls: &Walls, dir: &MoveDirection) -> bool {
    match dir {
        MoveDirection::Up => walls.top,
        MoveDirection::Right => walls.right,
        MoveDirection::Down => walls.bottom,
        MoveDirection::Left => walls.left,
    }
}

pub fn calculate_distance(pos1: &Position, pos2: &Position) -> f32 {
    ((pos1.x as f32 - pos2.x as f32).powi(2) + (pos1.y as f32 - pos2.y as f32).powi(2)).sqrt()
}

pub fn distance_from_line(pos: &Position, p1: &Position, p2: &Position) -> f32 {
    // see https://en.wikipedia.org/wiki/Distance_from_a_point_to_a_line#Line_defined_by_two_points
    let x0 = pos.x as f32;
    let y0 = pos.y as f32;
    let x1 = p1.x as f32;
    let y1 = p1.y as f32;
    let x2 = p2.x as f32;
    let y2 = p2.y as f32;

    ((x2-x1)*(y1-y0) - (x1-x0)*(y2-y1)).abs() / ((x2-x1).powi(2)+(y2-y1).powi(2)).sqrt()
}
