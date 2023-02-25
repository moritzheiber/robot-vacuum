mod api;
mod execution;
mod types;

use axum::{routing::post, Router};
use sqlx::any::AnyPoolOptions;
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use std::net::Ipv4Addr;

/*  For developer convenience these should be made configurable at runtime
    in the future.
*/
const SERVER_ADDRESS: &str = "0.0.0.0";
const SERVER_PORT: u16 = 5000;
const CONNECTION_POOL_SIZE: u32 = 5;

/*  The entire `main` function is `async` meaning it's safe to spawn as many of the processes
    inside of it as required, i.e. to scale the API appropriately.
*/
#[tokio::main]
async fn main() {
    /*  We are reading the DATABASE_URL environment variable as requested to configure
        the database dynamically. It follows the common syntax for database URLs, as
        documented in the README
    */
    let db_url = std::env::var("DATABASE_URL").expect("set DATABASE_URL env variable");

    // We are not making much use of the tracer in this challenge but it's still good to have it around
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "axum_api=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    /*  I'm creating a fairly small pool of connections for the database here which you
        will want to increase when scaling the app vertically.

        We are using Pool<Any> here to allow for changing the database backend seamlessly.
    */
    let pool = AnyPoolOptions::new()
        .max_connections(CONNECTION_POOL_SIZE)
        .connect(&db_url)
        .await
        .expect("Unable to connect to database");

    /*  All migrations are run at the start of the app, every time.
        Obviously, previously run migrations aren't run again. `sqlx` keeps track
        of them for us.
    */
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Unable to run migrations");

    /*  This is the main router object where we're mounting the routes into. The challenge
        only stipulates a single route, for which we are passing a single "handler" or controller.

        We are also passing along the database connection pool as "state" to ensure we can use
        it to store our execution results later.
    */
    let app = Router::new()
        .route("/path", post(api::handle_enter_path))
        .with_state(pool);

    /*  This creates the initial server, listening on the port and address defined
        at the top of this file. Once the initialization is complete the server listens
        for requests.
    */
    let addr = std::net::SocketAddr::from((
        SERVER_ADDRESS
            .parse::<Ipv4Addr>()
            .expect("Unable to qualify address to bind to"),
        SERVER_PORT,
    ));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("failed to start server");
}

/*  This function is bound to the server object to deal with POSIX signals sent to it
    by the operating system. Typical signals are `terminate` and exit (abbreviated as `ctrl_c`)
    in this case.

    This makes sure we are gracefully exiting the server process whenever somebody presses
    `CTRL+C` on the command line or when the container itself is shut down.
*/
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("Exiting...");
}
