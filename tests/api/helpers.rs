use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use serde_json::json;
use sqlx::{PgPool, PgConnection, Connection, Executor};
use uuid::Uuid;
use wiremock::MockServer;
use scoop::{
  configuration::{get_configuration, DatabaseSettings},
  telemetry::{get_subscriber, init_subscriber},
  startup::{get_connection_pool, Application}, email_client::EmailClient, issue_delivery_workers::{ExecutionOutcome, try_execute_task},
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
  pub email_client: EmailClient
}

pub struct ConfirmationLinks {
  pub html: reqwest::Url,
  pub plain_text: reqwest::Url,
}

impl TestApp {
  pub async fn displatch_all_pending_emails(&self) {
    loop {
      if let ExecutionOutcome::EmptyQueue =
        try_execute_task(&self.db_pool, &self.email_client)
          .await
          .unwrap()
          {
            break;
          }
    }
  }

  pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
    self.api_client
      .post(&format!("{}/subscriptions", &self.address))
      .header("Content-Type", "application/x-www-form-urlencoded")
      .body(body)
      .send()
      .await
      .expect("Failed to send request.")
  }

  pub async fn get_subscription_html(&self) -> String {
    self.api_client
      .get(&format!("{}/subscriptions", &self.address))
      .send()
      .await
      .expect("Failed to execute request.")
      .text()
      .await
      .unwrap()
  }

  pub fn get_unsubscription_link(
    &self,
    email_request: &wiremock::Request
  ) -> reqwest::Url {
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

    let html = self.get_link(&body["HtmlBody"].as_str().unwrap(), 1);
    let plain_text = self.get_link(&body["TextBody"].as_str().unwrap(), 1);

    assert_eq!(html, plain_text);
    html
  }

  pub fn get_confirmation_links(
    &self,
    email_request: &wiremock::Request
  ) -> ConfirmationLinks {
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

    let html = self.get_link(&body["HtmlBody"].as_str().unwrap(), 0);
    let plain_text = self.get_link(&body["TextBody"].as_str().unwrap(), 0);

    ConfirmationLinks {
      html,
      plain_text
    }
  }

  fn get_link(&self, s: &str, i: usize) -> reqwest::Url {
    let links: Vec<_> = linkify::LinkFinder::new()
        .links(s)
        .filter(|l| *l.kind() == linkify::LinkKind::Url)
        .collect();
      
    let raw_link = links[i].as_str().to_owned();
    let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
    assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
    confirmation_link.set_port(Some(self.port)).unwrap();
    confirmation_link
  }

  pub async fn post_submit_newsletter<Body>(&self, body: Body) -> reqwest::Response 
  where 
    Body: serde::Serialize,
  {
    self.api_client
      .post(format!("{}/admin/newsletters", &self.address))
      .form(&body)
      .send()
      .await
      .expect("Failed to execute request.")
  }

  pub async fn get_publish_newsletter(&self) -> reqwest::Response {
    self.api_client
      .get(format!("{}/admin/newsletters", &self.address))
      .send()
      .await
      .expect("Failed to send request")
  }

  pub async fn get_publish_newsletter_html(&self) -> String {
    self.get_publish_newsletter()
      .await
      .text()
      .await
      .unwrap()
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
  
  pub async fn post_logout(&self) -> reqwest::Response
  {
    self.api_client
      .post(&format!("{}/admin/logout", self.address))
      .send()
      .await
      .expect("Failed to execute request")
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

  pub async fn login(&self, app: &TestApp) -> reqwest::Response {
    app.post_login(&json!({
      "username": &self.username,
      "password": &self.password,
    }))
    .await
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
    email_client: configuration.email_client.client()
  };
  test_app.test_user.store(&test_app.db_pool).await;
  test_app
}

async fn configure_database(configuration: &DatabaseSettings) -> PgPool {
  let mut connection = PgConnection::connect_with(&configuration.without_db())
  .await
  .expect("Failed to connect to postgres.");

  connection.execute(format!(r#"CREATE DATABASE "{}";"#, configuration.name).as_str())
    .await
    .expect("Failed to create test database.");

  let connection_pool = PgPool::connect_with(configuration.with_db())
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