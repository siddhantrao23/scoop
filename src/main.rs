use scoop::{
  telemetry::{init_subscriber, get_subscriber}, 
  configuration::get_configuration, 
  startup::Application, issue_delivery_workers::run_worker_till_stopped
};
use std::fmt::{Debug, Display};
use tokio::task::JoinError;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let subscriber = get_subscriber(
      "scoop".into(), "info".into(), std::io::stdout
  );
  init_subscriber(subscriber);

  let configuration = get_configuration().expect("Failed to read user configuration.");
  let application = Application::build(configuration.clone()).await?;
  let application_task = tokio::spawn(application.run_until_stopped());
  let worker_task= tokio::spawn(run_worker_till_stopped(configuration));

  tokio::select! {
    o = application_task => report_exit("API", o),
    o = worker_task => report_exit("Background Worker", o),
  };

  Ok(())
}

fn report_exit(
  task: &str,
  outcome: Result<Result<(), impl Debug + Display>, JoinError>
) {
  match outcome {
    Ok(Ok(())) => {
      tracing::info!("{} has exited", task)
    }
    Ok(Err(e)) => {
      tracing::error!(
        error.cause_chain = ?e,
        error.message = %e,
        "{} failed",
        task
      )
    }
    Err(e) => {
      tracing::error!(
        error.cause_chain = ?e,
        error.message = %e,
        "{} has failed to complete",
        task
      )
    }
  }
}