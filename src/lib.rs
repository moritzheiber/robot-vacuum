mod api;
mod execution;
mod types;

use axum::{routing::post, Router};
use shuttle_service::ShuttleAxum;
use sqlx::PgPool;
use sync_wrapper::SyncWrapper;

/*  The entire `main` function is `async` meaning it's safe to spawn as many of the processes
    inside of it as required, i.e. to scale the API appropriately.
*/
#[shuttle_service::main]
async fn main(#[shuttle_shared_db::Postgres] pool: PgPool) -> ShuttleAxum {
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

    let wrapper = SyncWrapper::new(app);

    Ok(wrapper)
}
