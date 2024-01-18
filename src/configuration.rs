use crate::{domain::SubscriberEmail, email_client::EmailClient};
use secrecy::{Secret, ExposeSecret};
use serde_aux::prelude::*;

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
  pub name: String
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
  pub fn connection_string(&self) -> Secret<String> {
    Secret::new(format!(
      "postgres://{}:{}@{}:{}/{}",
      self.username, self.password.expose_secret(), self.host, self.port, self.name
    ))
  }

  pub fn connection_string_without_db(&self) -> Secret<String> {
    Secret::new(format!(
      "postgres://{}:{}@{}:{}",
      self.username, self.password.expose_secret(), self.host, self.port
    ))
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