use zero2prod::{
  telemetry::{init_subscriber, get_subscriber}, 
  configuration::get_configuration, 
  startup::Application
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let subscriber = get_subscriber(
      "zero2prod".into(), "info".into(), std::io::stdout
  );
  init_subscriber(subscriber);

  let configuration = get_configuration().expect("Failed to read user configuration.");
  let application = Application::build(configuration).await?;
  application.run_until_stopped().await?;
  Ok(())
}