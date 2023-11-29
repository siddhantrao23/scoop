use std::net::TcpListener;
use sqlx::PgPool;

use zero2prod::telemetry::{init_subscriber, get_subscriber};
use zero2prod::{startup::run, configuration::get_configuration};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read user configuration.");
    let connection_pool = PgPool::connect_lazy(
        &configuration.database.connection_string())
        .expect("Failed to connect to postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to run migrations.");
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool)?.await
}