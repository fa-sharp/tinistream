use redis_sse_streamer::build_rocket;
use rocket::launch;

#[launch]
fn rocket() -> _ {
    // Initialize JSON logging in production
    if cfg!(not(debug_assertions)) {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .json()
            .init();
    }

    build_rocket()
}
