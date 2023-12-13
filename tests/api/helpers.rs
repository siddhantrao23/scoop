use sqlx::{PgPool, PgConnection, Connection, Executor};
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::{
  configuration::{get_configuration, DatabaseSettings},
  telemetry::{get_subscriber, init_subscriber},
  startup::{get_connection_pool, Application},
};
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
  pub port: u16,
  pub db_pool: PgPool,
  pub email_server: MockServer,
}

impl TestApp {
  pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
    reqwest::Client::new()
      .post(&format!("{}/subscriptions", &self.address))
      .header("Content-Type", "application/x-www-form-urlencoded")
      .body(body)
      .send()
      .await
      .expect("Failed to send request.")
  }
}

pub async fn spawn_app() -> TestApp {
  Lazy::force(&TRACING);

  let email_server = MockServer::start().await;

  let configuration = {
    let mut c = get_configuration().expect("Failed to fetch configuration.");
    c.database.name = Uuid::new_v4().to_string();
    c.email_client.base_url = email_server.uri();
    c.application.port = 0;
    c
  };
  
  configure_database(&configuration.database).await;
  
  let application = Application::build(configuration.clone()).await.expect("Failed to build Application.");
  let address = format!("http://127.0.0.1:{}", application.port());
  let port = application.port();
  let _ = tokio::spawn(application.run_until_stopped());
  
  TestApp {
    address,
    port, 
    db_pool: get_connection_pool(&configuration.database),
    email_server,
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