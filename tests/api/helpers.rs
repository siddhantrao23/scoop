use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use secrecy::ExposeSecret;
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
  pub test_user: TestUser,
  pub api_client: reqwest::Client,
}

pub struct ConfirmationLinks {
  pub html: reqwest::Url,
  pub plain_text: reqwest::Url,
}

impl TestApp {
  pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
    self.api_client
      .post(&format!("{}/subscriptions", &self.address))
      .header("Content-Type", "application/x-www-form-urlencoded")
      .body(body)
      .send()
      .await
      .expect("Failed to send request.")
  }

  pub fn get_confirmation_links(
    &self,
    email_request: &wiremock::Request
  ) -> ConfirmationLinks {
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

    let get_link = |s: &str| {
      let links: Vec<_> = linkify::LinkFinder::new()
        .links(s)
        .filter(|l| *l.kind() == linkify::LinkKind::Url)
        .collect();
      
      assert_eq!(links.len(), 1);
      let raw_link = links[0].as_str().to_owned();
      let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
      assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
      confirmation_link.set_port(Some(self.port)).unwrap();
      confirmation_link
    };
    
    let html = get_link(&body["html_body"].as_str().unwrap());
    let plain_text = get_link(&body["text_body"].as_str().unwrap());

    ConfirmationLinks {
      html,
      plain_text
    }
  }

  pub async fn post_newsletter(
    &self,
    body: serde_json::Value
  ) -> reqwest::Response {
    self.api_client
      .post(format!("{}/newsletters", &self.address))
      .basic_auth(&self.test_user.username, Some(&self.test_user.password))
      .json(&body)
      .send()
      .await
      .expect("Failed to execute request.")
  }

  pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
  where 
    Body: serde::Serialize,
    {
      self.api_client
        .post(&format!("{}/login", self.address))
        .form(body)
        .send()
        .await
        .expect("Failed to execute request")
    }

  pub async fn get_login_html(&self) -> String {
    self.api_client
      .get(&format!("{}/login", &self.address))
      .send()
      .await
      .expect("Failed to send request.")
      .text()
      .await
      .unwrap()
  }
 
  pub async fn get_admin_dashboard(&self) -> reqwest::Response {
    self.api_client
      .get(&format!("{}/admin/dashboard", &self.address))
      .send()
      .await
      .expect("Failed to send request.")
  }

  pub async fn get_admin_dashboard_html(&self) -> String {
    self.get_admin_dashboard()
      .await
      .text()
      .await
      .unwrap()
  }

  pub async fn get_change_password(&self) -> reqwest::Response {
    self.api_client
      .get(format!("{}/admin/password", self.address))
      .send()
      .await
      .expect("Failed to execute request.")
  }

  pub async fn post_change_password<Body>(&self, body: &Body) -> reqwest::Response
    where
      Body: serde::Serialize, 
  {
    self.api_client
      .post(format!("{}/admin/password", self.address))
      .form(body)
      .send()
      .await
      .expect("Failed to execute request.")
  }

  pub async fn get_change_password_html(&self) -> String {
    self.get_change_password()
      .await
      .text()
      .await
      .unwrap()
  }
}

pub struct TestUser {
  pub user_id: Uuid,
  pub username: String,
  pub password: String,
}

impl TestUser {
  pub fn generate() -> Self {
    Self {
      user_id: Uuid::new_v4(),
      username: Uuid::new_v4().to_string(),
      password: Uuid::new_v4().to_string(),
    }
  }

  async fn store(&self, pool: &PgPool) {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::default()
      .hash_password(self.password.as_bytes(), &salt)
      .unwrap()
      .to_string();

    sqlx::query!(
      "INSERT INTO users (user_id, username, password_hash)
      VALUES ($1, $2, $3)",
      self.user_id,
      self.username,
      password_hash,
    )
    .execute(pool)
    .await
    .expect("Failed to store test user.");
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
  let application_port = application.port();
  let _ = tokio::spawn(application.run_until_stopped());
  
  let api_client = reqwest::Client::builder()
    .redirect(reqwest::redirect::Policy::none())
    .cookie_store(true)
    .build()
    .unwrap();

  let test_app = TestApp {
    address,
    port: application_port, 
    db_pool: get_connection_pool(&configuration.database),
    email_server,
    test_user: TestUser::generate(),
    api_client,
  };
  test_app.test_user.store(&test_app.db_pool).await;
  test_app
}

async fn configure_database(configuration: &DatabaseSettings) -> PgPool {
  let mut connection = PgConnection::connect(
      &configuration.connection_string_without_db().expose_secret()
  )
  .await
  .expect("Failed to connect to postgres.");

  connection.execute(format!(r#"CREATE DATABASE "{}";"#, configuration.name).as_str())
    .await
    .expect("Failed to create test database.");

  let connection_pool = PgPool::connect(&configuration.connection_string().expose_secret())
    .await
    .expect("Failed to connect to postgres.");

  sqlx::migrate!("./migrations")
    .run(&connection_pool)
    .await
    .expect("Failed to run migrations.");

  connection_pool
}

pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
  assert_eq!(response.status().as_u16(), 303);
  assert_eq!(response.headers().get("Location").unwrap(), location);
}