use config;

#[derive(serde::Deserialize)]
pub struct Settings {
  pub database: DatabaseSettings,
  pub application_port: u16
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
  username: String,
  password: String,
  port: u16,
  host: String,
  name: String
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
  let settings = config::Config::builder()
    .add_source(
      config::File::new("configuration.yaml", config::FileFormat::Yaml)
    )
    .build()?;

  settings.try_deserialize::<Settings>()
}