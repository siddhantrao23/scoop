#[derive(serde::Deserialize)]
pub struct Settings {
  pub database: DatabaseSettings,
  pub application: ApplicationSettings,
}

#[derive(serde::Deserialize)]
pub struct ApplicationSettings {
  pub host: String,
  pub port: u16
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
  pub username: String,
  pub password: String,
  pub port: u16,
  pub host: String,
  pub name: String
}

impl DatabaseSettings {
  pub fn connection_string(&self) -> String {
    format!(
      "postgres://{}:{}@{}:{}/{}",
      self.username, self.password, self.host, self.port, self.name
    )
  }

  pub fn connection_string_without_db(&self) -> String {
    format!(
      "postgres://{}:{}@{}:{}",
      self.username, self.password, self.host, self.port
    )
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