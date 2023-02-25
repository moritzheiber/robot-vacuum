use serde::{Deserialize, Serialize};
use std::ops::Add;

// The grid limit in any direction as defined by the challenge
pub const FIELD_LIMIT: i32 = 100000;

/*  `Position` is a representation of a single vertex on a 2D grid.
    Its main purpose is to ensure the robot stays within the defined grid
    and to serve as an efficient item for storing the cleaning results
    (that's why it has `Hash` as a `derive` macro).
*/
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Hash, Clone, Copy)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    /*  This function simulates a simple movement in a `Direction` away from a `Position`
        under certain conditions (mainly whether it can actually move into the direction
        with the required amount of steps). Once the conditions have cleared it replaces
        its current position with the destination it's supposed to move towards and returns
        itself back to the caller.
    */
    pub fn shift(mut self, direction: &Direction) -> Self {
        let movement = Position::from(direction);
        let destination = self + movement;
        if !destination.out_of_bounds() {
            self = destination
        }

        self
    }

    /*  The function just checks whether the field limit has been reached for any point
        of the `Position`.
    */
    fn out_of_bounds(&self) -> bool {
        self.x.abs() > FIELD_LIMIT || self.y.abs() > FIELD_LIMIT
    }
}

/*  I'm implementing the `Add` Trait for `Position` because that's how the robot actually
    moves! It adds the position it gets passed to in the `shift` function to the position
    it already has stored in its representation of `self`. Since we're just dealing with
    coordinates in a 2D grid it's a simple matter of adding the relevant coordinates together
    and returning the resulting `Position` as `self`.
*/
impl Add for Position {
    type Output = Self;

    fn add(self, position: Self) -> Self {
        Self {
            x: self.x + position.x,
            y: self.y + position.y,
        }
    }
}

/*  I'm storing any direction the robot can move into in this `Enum` representation,
    making it easier to associated functionality with each direction. I'm telling the
    serialization/deserialization library `serde` here that it should expect each `enum`
    representation to be in `lowercase` instead of the representation chosen by the `enum`.
    It's a common convention in Rust to write `enum`s as `PascalCase`.
*/
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    North,
    East,
    South,
    West,
}

/*  This is where I tell the robot how far it should move, given a direction. I've chosen
    to simply tell it to move exactly *one vertex* with each positional movement, with
    each direction representing a new coordinate on a 2D grid, which gets added to the already
    existing `Position`, e.g. moving a single step `North` means adding `1` to the `y` axis.
*/
impl From<&Direction> for Position {
    fn from(d: &Direction) -> Position {
        match d {
            Direction::North => Position { x: 0, y: 1 },
            Direction::East => Position { x: 1, y: 0 },
            Direction::West => Position { x: -1, y: 0 },
            Direction::South => Position { x: 0, y: -1 },
        }
    }
}

/*  `Command` is the upper "container" for instructions for the robot and it only contains
    the `Direction` the robot is supposed to move into and how many steps it should take.

    `Command` is mainly used for serialization/deserialization and contains little logic otherwise.
*/
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Command {
    pub direction: Direction,
    pub steps: i32,
}

/*  The test coverage here concerns itself with movements, in any direction, to ensure
    the robot can move and will move into relevant directions when told so.

    It also covers the use-case of adding a `Position` that's greater than 1/-1.
*/
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn shifts_position_with_boundary() {
        let position = Position { x: 1, y: 1 };
        let position = position.shift(&Direction::North);

        assert_eq!(position, Position { x: 1, y: 2 });

        let position = Position { x: 100000, y: 1 };
        let position = position.shift(&Direction::East);

        assert_eq!(position, Position { x: 100000, y: 1 });

        let position = Position { x: -100000, y: 1 };
        let position = position.shift(&Direction::West);

        assert_eq!(position, Position { x: -100000, y: 1 });

        let position = Position { x: 0, y: 0 };
        let position = position.shift(&Direction::West);

        assert_eq!(position, Position { x: -1, y: 0 })
    }

    #[test]
    fn adds_positions() {
        let a = Position { x: 10, y: 5 };
        let b = Position { x: 15, y: 11 };

        assert_eq!(Position { x: 25, y: 16 }, a + b)
    }

    #[test]
    fn adds_negative_position() {
        let a = Position { x: -10, y: 0 };
        let b = Position { x: 3, y: -210 };

        assert_eq!(Position { x: -7, y: -210 }, a + b)
    }
}
