use std::net::TcpListener;
use sqlx::postgres::PgPoolOptions;
use zero2prod::email_client::EmailClient;
use zero2prod::telemetry::{init_subscriber, get_subscriber};
use zero2prod::{startup::run, configuration::get_configuration};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read user configuration.");
    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy(
            &configuration.database.connection_string()
        )
        .expect("Failed to connect to postgres.");
    
    let sender_email = configuration.email_client.sender()
        .expect("Invalid sender email address.");
    let timeout = configuration.email_client.timeout();
    let email_client = EmailClient::new(
        sender_email,
        configuration.email_client.base_url,
        configuration.email_client.auth_token,
        timeout,
    );

    let address = format!("{}:{}", 
        configuration.application.host, configuration.application.port);
    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool, email_client)?.await
}