use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{error::Error as SqlError, Any, FromRow, Pool};
use std::collections::HashSet;

use crate::{api::Request, types::Position};

// The amount we have to use to divide seconds in order to get microseconds
pub const MICROSECONDS: i32 = 1000000;

/*  The `Execution` model does both, the heavy lifting for the computation as well as
    handling the database interactions. I've explained the `derive` syntax for `Request`
    in `api.rs` already, but this one carries a special `derive` macro called `FromRow`
    which tells the database interface that you can expect this struct to be deserializable
    from a database row coming from the database (including all of its types). If the
    database types/format changes it will no longer cleanly serialize into this type and
    throw an error, avoiding common data issues when dealing with persistence layers.

    Otherwise, we are relying on the database to fill in `id` and `timestamp` (that's why
    they are "Option" type, e.g. can be "None" (null) initially), since both of these values
    are data-storage relevant, not object-relevant (e.g. `id` can only ever be truly
    consistent if the database decides which `id` is associated with which row). `duration`
    is also an "Option" since it's a type were "guessing" the initial value is tricky
    because it's a `float` (where `commands` and `result` can easily be set to `0`).

    `timestamp` is kept as UTC here and only ever localized when necessary.
*/
#[derive(FromRow, Serialize, PartialEq, Debug, Clone, Default)]
pub struct Execution {
    pub id: Option<i32>,
    pub timestamp: Option<DateTime<Utc>>,
    pub commands: i32,
    pub result: i32,
    pub duration: Option<f64>,
}

impl Execution {
    /*  This is the main function responsible for coordinating the robot's movements
       and storing the results. In the beginning it simply stores the number of commands
       it is going to execute, initializes its original position and then builds a HashSet
       which contain the unique representations of all the fields it has visited.

       Since the HashSet is fairly efficient at storing hashes Positions we can just
       keep storing positions (it'll essentially be a no-op) regardless of whether they
       are a part of the set already or not. The members of the set are then the vertices
       the robot has cleaned, piped into the `result` attribute.
    */
    pub async fn calculate(mut self, request: Request) -> Self {
        self.commands = request.commands.len() as i32;

        let mut position = request.start;
        let mut cleaned: HashSet<Position> = HashSet::new();
        let commands = request.commands;

        /*  This is our starting timestamp for measuring the duration
            of the computation.
        */
        let start_time = Utc::now();

        for command in commands {
            /*  This creates an _inclusive_ Range type in Rust, in this case
                1 to "number of steps".
            */
            for _ in 1..=command.steps {
                position = position.shift(&command.direction);
                cleaned.insert(position);
            }
        }

        self = self.set_duration(start_time);
        self.result = cleaned.len() as i32;
        self
    }

    /*  This function contains the interaction logic for the persistence layer/database.
        It receives just the connection pool for the database and then essentially
        saves "itself" into a database row, while also fetching the result with the same
        query.

        This yields a "fully formed" `Execution` struct, now also containing `id` and
        `timestamp`, as created by the database engine itself, which is then returned to the
        caller as a `Result`. We are using `Result` for error handling purposes here exclusively
        because it allows for the code be more readable. Also, this is probably one of the few
        places you would want to have proper error handling at in the future.
    */
    pub async fn save(&self, state: Pool<Any>) -> Result<Execution, SqlError> {
        let result: Execution = sqlx::query_as(
        r#"insert into executions (commands, result, duration) values ($1, $2, $3) returning *"#,
    )
    .bind(self.commands)
    .bind(self.result)
    .bind(self.duration.clone()).fetch_one(&state)
    .await?;

        Ok(result)
    }

    /*  This function take the initial timestamp we saved before triggering the
        calculation of the robot movements and compares it against a current timestamp.
        It then takes the microseconds elapsed between then and now and converts them
        into a seconds-precision float representation, as required by the data model from
        the challenge.
    */
    fn set_duration(mut self, start_time: DateTime<Utc>) -> Self {
        let now = Utc::now();
        let duration = (now - start_time)
            .num_microseconds()
            .map(|d| d as f64 / MICROSECONDS as f64);
        self.duration = duration;
        self
    }
}

/*  All of these tests are mainly focusing on `Request` > `Execution` > `Response` conversion
    to ensure data consistency and to catch any issues from changing the data model
    in any of these structs.
*/
#[cfg(test)]
mod test {
    use crate::{
        api::Request,
        types::{Command, Direction, Position},
    };

    use super::Execution;

    #[tokio::test]
    async fn calculates_row_item_10_east() {
        let request = Request {
            start: Position { x: 0, y: 0 },
            commands: vec![Command {
                direction: Direction::East,
                steps: 10,
            }],
        };

        let execution = Execution::default();
        let execution = execution.calculate(request).await;
        assert_eq!(execution.result, 10)
    }

    #[tokio::test]
    async fn calculates_row_item_0_east() {
        let request = Request {
            start: Position { x: 0, y: 0 },
            commands: vec![Command {
                direction: Direction::East,
                steps: 0,
            }],
        };

        let execution = Execution::default();
        let execution = execution.calculate(request).await;
        assert_eq!(execution.result, 0)
    }

    #[tokio::test]
    async fn calculates_row_item_10_west() {
        let request = Request {
            start: Position { x: -10, y: 0 },
            commands: vec![Command {
                direction: Direction::West,
                steps: 10,
            }],
        };

        let execution = Execution::default();
        let execution = execution.calculate(request).await;
        assert_eq!(execution.result, 10)
    }

    #[tokio::test]
    async fn calculates_row_items_10_west_10_east() {
        let request = Request {
            start: Position { x: -10, y: 0 },
            commands: vec![
                Command {
                    direction: Direction::West,
                    steps: 10,
                },
                Command {
                    direction: Direction::East,
                    steps: 10,
                },
            ],
        };

        let execution = Execution::default();
        let execution = execution.calculate(request).await;
        assert_eq!(execution.result, 11)
    }

    #[tokio::test]
    async fn calculates_row_items_122_west_70_east_22_north() {
        let request = Request {
            start: Position { x: -10, y: 0 },
            commands: vec![
                Command {
                    direction: Direction::West,
                    steps: 122,
                },
                Command {
                    direction: Direction::East,
                    steps: 70,
                },
                Command {
                    direction: Direction::North,
                    steps: 22,
                },
            ],
        };

        let execution = Execution::default();
        let execution = execution.calculate(request).await;
        assert_eq!(execution.result, 144)
    }

    #[tokio::test]
    async fn calculates_row_items_22_east_70_west_120000_north() {
        let request = Request {
            start: Position { x: 100000, y: 222 },
            commands: vec![
                Command {
                    direction: Direction::East,
                    steps: 22,
                },
                Command {
                    direction: Direction::West,
                    steps: 70,
                },
                Command {
                    direction: Direction::North,
                    steps: 120000,
                },
            ],
        };

        let execution = Execution::default();
        let execution = execution.calculate(request).await;
        assert_eq!(execution.result, 99849)
    }
}
