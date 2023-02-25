use axum::{
    extract::{Json, State},
    response::Json as ResponseJson,
};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use sqlx::{Any, Pool};

use crate::{
    execution::Execution,
    types::{Command, Position},
};

/*  A Request is a representation of the JSON spec delivered with the challenge.
    If the request has any other structure the server will respond unkind.
    The deserializes assures us that whatever data is being submitted to the API
    has the appropriate format indicated by the types used, e.g. start is a `Position`,
    a struct with two fields, `x` and `y`, which are both `i32` integers, and the
    commands fields has to be an array (Rust calls them `Vector`, or `Vec`) of
    commands, with their structure being the direction the robot should move in and
    number of steps it'll take.

    I mostly use low-yield integer types here for convenience and since the limits
    expressed by the challenge all fit into a regular integer type.

    The `derive` keyword applies a few Rust macros (essentially generating code on-the-fly),
    which allows for automatically generating certain classes and methods for operations
    relevant to the app (e.g. comparing on instance of a struct to another, for serializing
    and deserializing them into different formats etc.).
*/
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Request {
    pub start: Position,
    pub commands: Vec<Command>,
}

/*  I've chosen to use the Option type here, which can either be a value ("Some")
    or "None" (e.g. empty) because the initial instance of a Response shouldn't
    contain any data it cannot know about itself just by existing. The relevant
    fields are then "filled" by combining its "default" data with the data of another
    class. In this case I'm creating any Response instance from an instance of Execution
    (the function for this is `impl` you see beneath this initializer).
*/
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Response {
    id: Option<i32>,
    timestamp: Option<DateTime<Local>>,
    commands: i32,
    result: i32,
    duration: Option<String>,
}

/*  `From` is a `Trait` in Rust, basically an interface for other classes you can choose
    to implement on your own class (or not). It allows for building bridges and comparable
    data types (e.g. for passing generics to functions; Rust is strongly typed otherwise).
    Essentially, you're giving whoever uses your class certain guarantees it'll behave in a
    certain way.

    By implementing the `From` Trait for `Execution` on `Response` I guarantee that any
    `Execution` can _always_ be converted into a `Response`.

    The reason for this here is simply formatting of the `Response` itself. The data
    associated with `Execution` is not changed. It's a "poor man's" View in MVC.
*/
impl From<Execution> for Response {
    fn from(execution: Execution) -> Self {
        // We want the local timezone attached to the returned JSON result
        let timestamp: Option<DateTime<Local>> = execution.timestamp.map(|dt| DateTime::from(dt));

        // We want to properly format the millisecond duration in seconds
        let duration = execution.duration.map(|d| format!("{:.6}", d));

        Response {
            id: execution.id,
            timestamp,
            commands: execution.commands,
            result: execution.result,
            duration,
        }
    }
}

/*  The main handler/controller for the API path `/path`.
    On top of the request object itself and also receives state information
    from the main router (in this case the database connection pool).

    Its sole job is to receive the request, trigger the calculation for `Execution`
    required for `Response` and then build the `Response` object from the resulting
    `Execution`.

    It has as little ambiguity as possible, it's essentially a conduit (just like
    controllers should be). The heavy lifting should be done by the model itself.
*/
pub async fn handle_enter_path(
    State(pool): State<Pool<Any>>,
    Json(request): Json<Request>,
) -> ResponseJson<Response> {
    let execution = Execution::default();
    let execution = execution.calculate(request).await;
    let execution = execution
        .save(pool)
        .await
        .expect("Unable to save execution to database");

    let response = Response::from(execution);

    ResponseJson(response)
}

#[cfg(test)]
mod test {
    use chrono::{NaiveDate, Utc};
    use std::fs;

    use super::*;
    use crate::types::Direction;

    /*  This test assures that we always carry a proper local timezone in our
        response output, despite working with UTC otherwise
    */
    #[test]
    fn converts_from_execution_to_response() {
        let execution_dt = NaiveDate::from_ymd_opt(2014, 11, 28)
            .unwrap()
            .and_hms_nano_opt(12, 0, 9, 1)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();
        let execution = Execution {
            id: Some(1),
            timestamp: Some(execution_dt),
            commands: 3,
            result: 10,
            duration: Some(0.000023),
        };

        let response = Response::from(execution);

        assert_eq!(
            Some("2014-11-28T13:00:09.000000001+01:00".to_string()),
            response.timestamp.map(|dt| format!("{:?}", dt))
        );

        assert_eq!(Some("0.000023".to_string()), response.duration);
        assert_eq!(3, response.commands);
        assert_eq!(10, response.result);
        assert_eq!(Some(1), response.id);
    }

    /*  These tests are mainly parsing fixtures, taken from the challenge document,
        to ensure compatibility with the supposed "spec" for the requests.
    */
    #[tokio::test]
    async fn parses_fixtures() {
        let file = fs::read_to_string("test/fixtures/example_request_positive.json")
            .expect("Unable to read file");
        let request: Request = serde_json::from_str(&file).unwrap();

        assert_eq!(request.start, Position { x: 10, y: 22 });
        assert_eq!(
            request.commands[0],
            Command {
                direction: Direction::East,
                steps: 2
            }
        );

        let execution = Execution::default();
        let execution = execution.calculate(request).await;
        assert_eq!((execution.commands, execution.result), (2, 3));

        let file = fs::read_to_string("test/fixtures/example_request_negative.json")
            .expect("Unable to read file");
        let request: Request = serde_json::from_str(&file).unwrap();
        assert_eq!(request.start, Position { x: -10, y: -22 });
        assert_eq!(
            request.commands[0],
            Command {
                direction: Direction::West,
                steps: 2
            }
        );

        let execution = Execution::default();
        let execution = execution.calculate(request).await;
        assert_eq!((execution.commands, execution.result), (2, 2));

        let file = fs::read_to_string("test/fixtures/example_request_10_commands.json")
            .expect("Unable to read file");
        let request: Request = serde_json::from_str(&file).unwrap();
        assert_eq!(request.start, Position { x: 0, y: 0 });
        assert_eq!(
            request.commands[0],
            Command {
                direction: Direction::West,
                steps: 120
            }
        );

        let execution = Execution::default();
        let execution = execution.calculate(request).await;
        assert_eq!((execution.commands, execution.result), (10, 15688));
    }
}
