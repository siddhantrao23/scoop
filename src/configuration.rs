use crate::{domain::SubscriberEmail, email_client::EmailClient};
use secrecy::{Secret, ExposeSecret};
use serde_aux::prelude::*;
use sqlx::{postgres::{PgConnectOptions, PgSslMode}, ConnectOptions};

#[derive(serde::Deserialize)]
#[derive(Clone)]
pub struct Settings {
  pub database: DatabaseSettings,
  pub application: ApplicationSettings,
  pub email_client: EmailClientSettings,
  pub redis_uri: Secret<String>,
}

#[derive(serde::Deserialize, Clone)]
pub struct ApplicationSettings {
  pub host: String,
  #[serde(deserialize_with = "deserialize_number_from_string")]
  pub port: u16,
  pub base_url: String,
  pub hmac_secret: Secret<String>,
}

#[derive(serde::Deserialize)]
#[derive(Clone)]
pub struct DatabaseSettings {
  pub username: String,
  pub password: Secret<String>,
  pub port: u16,
  pub host: String,
  pub name: String,
  pub require_ssl: bool,
}

#[derive(serde::Deserialize)]
#[derive(Clone)]
pub struct EmailClientSettings {
  sender_email: String,
  pub base_url: String,
  pub auth_token: Secret<String>,
  timeout_ms: u64
}

impl EmailClientSettings {
  pub fn client(self) -> EmailClient {
    let sender_email = self.sender().expect("Invalid sender email.");
    let timeout = self.timeout();
    EmailClient::new(sender_email, self.base_url, self.auth_token, timeout)
  }

  pub fn sender(&self) -> Result<SubscriberEmail, String> {
    SubscriberEmail::parse(self.sender_email.clone())
  }

  pub fn timeout(&self) -> std::time::Duration {
    std::time::Duration::from_millis(self.timeout_ms)
  }
}

impl DatabaseSettings {
  pub fn without_db(&self) -> PgConnectOptions {
    PgConnectOptions::new()
      .host(&self.host)
      .username(&self.username)
      .password(&self.password.expose_secret())
      .port(self.port)
      .ssl_mode(PgSslMode::Disable)
  }

  pub fn with_db(&self) -> PgConnectOptions {
    let mut options = self.without_db().database(&self.name);
    options.log_statements(tracing_log::log::LevelFilter::Trace);
    options
  }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
  let base_path = std::env::current_dir()
    .expect("Failed to get current directory.");
  let config_dir = base_path.join("configuration");

  let environment: Environment = std::env::var("APP_ENVIRONMENT")
    .unwrap_or_else(|_| "local".into())
    .try_into()
    .expect("Failed to fetch APP_ENVIRONMENT");
  let environment_filename = format!("{}.yaml", environment.as_str());

  let settings = config::Config::builder()
    .add_source(
      config::File::from(config_dir.join("base.yaml"))
    )
    .add_source(
      config::File::from(config_dir.join(environment_filename))
    )
    .add_source(
      config::Environment::with_prefix("APP")
        .prefix_separator("_")
        .separator("__"),
    )
    .build()?;
  settings.try_deserialize::<Settings>()
}

pub enum Environment {
  Local,
  Production
}

impl Environment {
  pub fn as_str(&self) -> &'static str {
    match self {
        Environment::Local => "local",
        Environment::Production => "production"
    }
  } 
}

impl TryFrom<String> for Environment {
  type Error = String;

  fn try_from(value: String) -> Result<Self, Self::Error> {
      match value.to_lowercase().as_str() {
        "local" => Ok(Environment::Local),
        "production" => Ok(Environment::Production),
        other => Err(format!("{} is not a supported Environment. \
                        Use `local` or `production`.", other
                      )),
      }
  }
}