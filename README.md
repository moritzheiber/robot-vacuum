# Robot Vacuum

## Introduction

A robot vacuum moving in a 2D plain. It was a developer test for a job application originally, but I removed all the references to the original challenge and salvaged the code as an example for deploying a web app on Shuttle.

## General assumptions

- The robot does **not** clean the field it starts from, i.e. when its told to move 10 fields into one direction and then 10 fields back into the same direction it will have cleaned **11** fields because it cleans the field it starts from as well (and will not "clean" the fields it has passed over already again).
- *If* the robot is at the edge of the grid (=< 100000 in all directions) any steps towards the edge it has met are discarded and the robot remains in its place. The command itself still counts as fully executed.
- The output for the seconds of the `duration` are a JSON `string`. As per the spec, numbers of any kind can be represented as a `string` because they're easily converted into their respective types.
- The timezone indicated in the document is `GMT+2` (`+02:00`). I have taken the liberty to use my own timezone as an output format. You can change the timezone inside the Docker container (see below) to fit your expectations.
- Since the challenge says that "[...]All should be considered well formed and syntactically correct[...]" there is little to no error handling or type checking, however, Rust's strict type system brings fairly strong guarantees as to validity of the application's logic when given the right input data (but will fail/panic when given malformed input data without remedies).

## Application logic

Generally, its setup has one controller method (`handle_enter_path` in `api.rs`), two HTTP structs (`Request` and `Response`) and a model (`Execution` in `execution.rs`).

You can set the variable `DATABASE_URL` either to connect to a `sqlite3` or `postgres` database:

- `sqlite3`: `sqlite://<path-to-file>` (e.g. `sqlite:///my-database.db)
- `postgres`: `postgres://<username>:<password>@<hostname>/<database>` (see the `docker-compose.yml` file for an example)

*Note: If you wish to use `sqlite3` you'll have to following the instructions under "Running the app" later in the README.*

The database schema is set up by a migration (in the `migrations/` folder) and migrations are run on every start of the app to ensure consistency and extensibility.

The app serializes the JSON input from the `POST` request in a `Request` struct, which it then uses to create an `Execution` model, `calculate()` the output variables required to store the result(s) and then `save()` the result to the database.

It uses a single query to save the result and retrieve the `Execution` again (using the `RETURNING *` SQL syntax) since the `id` and `timestamp` fields are assigned by the underlying database (and therefore empty (Rust calls this [`None` for an `Option` value](https://doc.rust-lang.org/std/option/)) before the `Execution` is saved to the database). This allows for greater consistency when storing the executions, as the database itself knows exactly when an item has been stored and also has excellent support for creating identifiers (primary keys).

Afterwards the `Execution` is converted into a `Response` object to achieve the desired output format (e.g. `timestamp` including a local timezone, `duration` in seconds with a precision of 6 after the point) and passed back to the browser as a JSON-encoded string with the appropriate `content-type` (`application/json`).

## Thoughts on structure, readability, maintainability, performance, re-usability and test-ability

- The app in designed to be fully threadable/concurrent wherever needed (request/response processing, calculations, working with the database), which should allow for fairly trivial horizontal and vertical scaling.
- The app follows the general MVC pattern, although the "View" part of it is simply a JSON serialization for now. It should be trivial to extend its functionality with other controllers, models and, eventually, views for displaying data. The underlying web framework, [axum](https://github.com/tokio-rs/axum), has plenty of available templating functionality, is well-maintained and has first-class [`async`](https://tokio.rs/tokio/tutorial/async) support, meaning it's structure it scalable enough to respond to a large number of requests, given the appropriate resources.
- I tried to split the code into logical units of operation, following the data model for the content that's submitted to the API, with a few supplemental types in classes outside of the structure (mainly `Position` and `Direction`).
- The database abstraction used, [`sqlx`](https://github.com/launchbadge/sqlx), is a thin, fully `async` layer on top of most of the available drivers and mainly focuses on pooling connections, query-building and consistency. If further logic, and especially type-checking/mangling, would be required I would probably take a look at `sea-orm` instead (which is based upon `sqlx`). It uses a fairly small pool of connections initially (`5`) which should probably be extended if the app sees heavier traffic.
- Postgres itself stores any `TIMESTAMPTZ` type as UTC-time, so we wouldn't need to pass UTC timestamps per-se but I like to represent time in UTC mostly and only convert it into different timezones when necessary (e.g. when displaying the timezone somewhere else). It provides for greater consistency and reduces ambiguity when dealing with `DateTime` objects.
- I've mostly written unit-tests, but also some integration tests (focusing on type interactions, e.g. `Response` with `Execution` or `Position` and `Direction`). Unfortunately, I didn't get to adding a few integration tests for the database handling and I would probably look into extending the code so far as to be able to test different database implementations as well. The code itself already has a state it passes around, which you could easily extend for mocking the essentials for integration tests.
- The "algorithm" I use is trivial and not well-optimized. I would've wished to find a more appropriate solution here but my algorithms classes I took in university were 20 years ago, unfortunately. For the boundaries defined by the challenge (10000 commands at most, 100000 steps at most) it *should be* performant enough, I haven't run any benchmarks though. I'm using a [`HashSet`](https://doc.rust-lang.org/std/collections/struct.HashSet.html) internally, which means the more ground is covered by the robot the easier it'll be on the computation (since the relevant vertices are already stored in the set), but it still only covers the entire grid one step at a time.
- I used the `Any` database type in the code on purpose to make it easier to iterate the code locally using an SQL abstraction (Sqlite3). Obviously, any future integration tests should be performed against an actual Postgres database, however, Sqlite3 has enough compatibility (and `sqlx` a good enough abstraction) to make it a convenient mechanism for local development.

## Working with the app

### Prerequisites for building/running the app

- [Rust](https://rustup.rs/) (>= 1.65.0)
- [Docker](https://docs.docker.com/engine/install/) (>= 20.10.21)
- [Docker Compose](https://docs.docker.com/compose/install/) (>= 2.12.2)

### Building the app

Rust comes with its own package manager, `cargo`, which can be used to build the app:

```console
$ cargo build
```

You can find the resulting binary at the path `target/debug/robot-vacuum`. If you want to build a release artifact (debugging symbols stripped, optimized for size and speed) you can use:

```console
$ cargo build --release
```

It will take a little longer and also won't re-use any of the existing cached artifacts. Its resulting artifact binary is to be found at `target/release/robot-vacuum`.

### Running the app

The app supports `sqlite3` and `postgres` as database backends, but unfortunately, the migrations will have to be adjusted accordingly. The changes are documented in the file in the `migrations/` folder.

If you want to run the app locally against `sqlite3` you can do this with:

```console
$ DATABASE_URL="sqlite://sqlite.db?mode=rwc" cargo run
```

`?mode=rwc` basically tells the database driver to open the database in "read-write" mode, but also "create" (`c` at the end) it should it be missing.

The server will then listen on port `5000` on `0.0.0.0` (all interfaces) by default. You can change the `SERVER_ADDRESS` constant in `main.rs` to something else, just be aware that the app needs to listen on an outgoing address (i.e. _not_ `localhost`) for it to work inside the Docker container.

### Running the tests

The test coverage is pretty good, with all important functions safe for the direct database interaction tested, including the request/response parsing. You can run the tests with `cargo`:

```console
$ cargo test
```

## Building the Docker container

The `Dockerfile` uses [BuildKit](https://docs.docker.com/build/buildkit/) to ensure that subsequent builds are cached properly, massively speeding up the process of re-building the container (cutting the time it takes down from roughly 10 minutes to a few seconds, sometimes). Please set the `BUILDKIT` variable accordingly:

```console
$ export DOCKER_BUILDKIT=1
```

Then you can simply run:

```console
$ docker build -t robot-vacuum .
```

The resulting image is based on `alpine:3.16`, to minimize its footprint. I would've used a `scratch` image, however, dealing with timezones is tricky (and the output is supposed to have a local timezone attached to it).

### Changing the timezone

You can change the timezone the container is using by passing the build argument `TZ` to `docker build`:

```console
$ docker build --build-arg TZ="GMT" -t robot-vacuum .
```

### Clearing the Docker cache

If you want to purge the build cache later you can do this with:

```console
$ docker builder prune --filter type=exec.cachemount
```

## Docker Compose

The `docker-compose.yml` uses a fairly recent syntax for `healthcheck` on the `postgres` service and `depends_on` for `robot-vacuum`, so please make sure your `docker-compose` or `docker-compose-plugin` are up to date.

### Starting the stack

```console
$ docker compose up
```

*Note: This assumes you're using the Docker Compose plugin. If you're using the stand-alone binary distribution of `compose` please use `docker-compose` instead of `docker compose` whenever working with `compose`.*

The `robot-vacuum` service will be available shortly after the database has finished initializing and can be reached via port `5000`. You can use one of the fixtures to test its functionality:

```console
$ curl --json @test/fixtures/example_request_positive.json http://localhost:5000/path
{"id":1,"timestamp":"2022-12-15T14:16:15.189809+01:00","commands":2,"result":3,"duration":"0.000001"}
```

### Stopping the stack

Simply press `CTRL+C` in your terminal. The database and app will shut down shortly after. You can get rid of the existing containers with:

```console
$ docker compose rm -fv
```
