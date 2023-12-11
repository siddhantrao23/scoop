use std::net::TcpListener;
use sqlx::{PgPool, PgConnection, Connection, Executor};
use uuid::Uuid;
use zero2prod::{configuration::{get_configuration, DatabaseSettings}, telemetry::{get_subscriber, init_subscriber}, email_client::EmailClient};
use once_cell::sync::Lazy;

static TRACING: Lazy<()> = Lazy::new(|| {
  let default_filter_level = "info".to_string();
  let subscriber_name = "name".to_string();

  if std::env::var("TEST_LOG").is_ok() {
    let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
    init_subscriber(subscriber);
  } else {
    let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
    init_subscriber(subscriber);
  }
});

pub struct TestApp {
  pub address: String,
  pub db_pool: PgPool,
}

pub async fn spawn_app() -> TestApp {
  Lazy::force(&TRACING);

  let listener = TcpListener::bind("127.0.0.1:0")
        .expect("Failed to bind random port.");
  let port = listener.local_addr().unwrap().port();
  let address = format!("http://127.0.0.1:{}", port);

  let mut configuration = get_configuration().expect("Failed to read user configuration.");
  configuration.database.name = Uuid::new_v4().to_string();
  let connection_pool = configure_database(&configuration.database).await;
  
  let sender_email = configuration.email_client.sender()
    .expect("Invalid sender email address.");
  let email_client = EmailClient::new(
    sender_email,
    configuration.email_client.base_url,
    configuration.email_client.auth_token,
    std::time::Duration::from_millis(200),
  );
  
  let server = zero2prod::startup::run(listener, connection_pool.clone(), email_client).
    expect("Failed to bind address.");
  let _ = tokio::spawn(server);
  TestApp {
    address,
    db_pool: connection_pool
  }
}

async fn configure_database(configuration: &DatabaseSettings) -> PgPool {
  let mut connection = PgConnection::connect(
      &configuration.connection_string_without_db()
  )
  .await
  .expect("Failed to connect to postgres.");

  connection.execute(format!(r#"CREATE DATABASE "{}";"#, configuration.name).as_str())
    .await
    .expect("Failed to create test database.");

  let connection_pool = PgPool::connect(&configuration.connection_string())
    .await
    .expect("Failed to connect to postgres.");

  sqlx::migrate!("./migrations")
    .run(&connection_pool)
    .await
    .expect("Failed to run migrations.");

  connection_pool
}