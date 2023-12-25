use anyhow::Context;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use actix_web::{HttpResponse, web, ResponseError};
use reqwest::StatusCode;
use sqlx::{PgPool, Postgres, Transaction};
use chrono::Utc;
use uuid::Uuid;

use crate::{
  domain::{NewSubscriber, SubscriberName, SubscriberEmail}, 
  email_client::EmailClient, startup::ApplicationBaseUrl
};

#[derive(serde::Deserialize)]
pub struct FormData {
  email: String,
  name: String
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { email, name })
    }
}

fn generate_subscription_token() -> String {
  let mut rng = thread_rng();
  std::iter::repeat_with(|| rng.sample(Alphanumeric))
    .map(char::from)
    .take(25)
    .collect()
}

#[tracing::instrument(
  name = "Adding a new subscriber",
  skip(form, pool, email_client, base_url),
  fields(
    subscriber_name = %form.name,
    subscriber_email = %form.email 
  )
)]
pub async fn subscribe(
  form: web::Form<FormData>,
  pool: web::Data<PgPool>,
  email_client: web::Data<EmailClient>,
  base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
  let new_sub = form.0.try_into().map_err(SubscribeError::ValidationError)?;
  let mut transaction = pool.begin()
    .await
    .context("Failed to acquire a Postgres connection from pool")?;
  let subscriber_id = insert_subscriber(&mut transaction, &new_sub)
    .await
    .context("Failed to insert new subscriber in database.")?;
  let subscriber_token = generate_subscription_token();
  store_token(&mut transaction, subscriber_id, &subscriber_token)
    .await
    .context("Failed to store confirmation token for a new subscriber.")?;
  transaction.commit()
    .await
    .context("Failed to  commit the transaction to database.")?;
  send_confirmation_email(&email_client, new_sub, &base_url.0, &subscriber_token)
    .await
    .context("Failed to send the confirmation email to subscriber.")?;
  Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
  name = "Send a confirmation email to new subscribers.",
  skip(email_client, new_sub, base_url, subscription_token),
)]
pub async fn send_confirmation_email(
  email_client: &EmailClient,
  new_sub: NewSubscriber,
  base_url: &str,
  subscription_token: &str,
) -> Result<(), reqwest::Error> {
  let confirmation_link = format!(
    "{}/subscriptions/confirm?subscription_token={}",
    base_url,
    subscription_token,
  );
  email_client.send_email(
    new_sub.email,
    "Welcome!", 
    &format!(
      "Welcome to our newsletter!<br/> Click <a href=\"{}\">here</a> to confirm your subscription.",
      confirmation_link
    ),
    &format!(
      "Welcome to our newsletter! \n Visit {} to confirm your subscription.",
      confirmation_link
    ),    
  ).await
}

#[tracing::instrument(
  name = "Saving new subscriber details to the database",
  skip(transaction, new_sub)
)]
pub async fn insert_subscriber(
  transaction: &mut Transaction<'_, Postgres>, 
  new_sub: &NewSubscriber
) -> Result<Uuid, sqlx::Error> {
  let subscriber_id = Uuid::new_v4();
  sqlx::query!(r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at, status)
    VALUES ($1, $2, $3, $4, 'pending_confirmation')
    "#,
    subscriber_id,
    new_sub.email.as_ref(),
    new_sub.name.as_ref(),
    Utc::now(),
  )
  .execute(transaction)
  .await
  .map_err(|e| {
    tracing::error!("Failed to execute query: {:?}", e);
    e
  })?;
  Ok(subscriber_id)
}

#[tracing::instrument(
  name = "Storing the subscription token to database."
  skip(transaction, subscriber_id, subscriber_token)
)]
async fn store_token(
  transaction: &mut Transaction<'_, Postgres>, 
  subscriber_id: Uuid,
  subscriber_token: &str,
) -> Result<(), StoreTokenError> {
  sqlx::query!(r#"
    INSERT INTO subscription_tokens (subscription_token, subscriber_id)
    VALUES ($1, $2)
    "#,
    subscriber_token,
    subscriber_id,
  )
  .execute(transaction)
  .await
  .map_err(|e| {
    tracing::error!("Failed to execute query: {:?}", e);
    StoreTokenError(e)
  })?;
  Ok(())
}

fn error_chain_fmt(
  e: &impl std::error::Error,
  f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
  writeln!(f, "{}\n", e)?;
  let mut current = e.source();
  while let Some(cause) = current {
    writeln!(f, "Caused by:\n\t{}", cause)?;
    current = cause.source();
  }
  Ok(())
}

pub struct StoreTokenError(sqlx::Error);

impl std::fmt::Display for StoreTokenError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(
        f,
        "A database error was encountered while trying to store the subscription token."
      )
  }
}

impl std::fmt::Debug for StoreTokenError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    error_chain_fmt(self, f)
  }
}

impl std::error::Error for StoreTokenError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    Some(&self.0)
  }
}

impl ResponseError for StoreTokenError {}

#[derive(thiserror::Error)]
pub enum SubscribeError {
  #[error("{0}")]
  ValidationError(String),
  #[error(transparent)]
  UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for SubscribeError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
  error_chain_fmt(self, f)
  }
}

impl ResponseError for SubscribeError {
  fn status_code(&self) -> StatusCode {
    match self {
      SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
      SubscribeError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
  }
}