use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use actix_web::{HttpResponse, web};
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
) -> HttpResponse {
  let new_sub = match NewSubscriber::try_from(form.0) {
    Ok(new_sub) => new_sub,
    Err(_) => return HttpResponse::BadRequest().finish(),
  };
  
  let mut transaction = match pool.begin().await {
    Ok(transaction) => transaction,
    Err(_) => return HttpResponse::InternalServerError().finish()
  };

  let subscriber_id = match insert_subscriber(&mut transaction, &new_sub).await {
    Ok(subscriber_id) => subscriber_id,
    Err(_) => return HttpResponse::InternalServerError().finish(),
  };

  let subscriber_token = generate_subscription_token();
  if store_token(&mut transaction, subscriber_id, &subscriber_token).await.is_err() {
    return HttpResponse::InternalServerError().finish();
  }

  if send_confirmation_email(
    &email_client,
    new_sub,
    &base_url.0,
    &subscriber_token
  ).await.is_err() {
    return HttpResponse::InternalServerError().finish()
  }

  if transaction.commit().await.is_err() {
    return HttpResponse::InternalServerError().finish();
  }
  
  HttpResponse::Ok().finish()
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
) -> Result<(), sqlx::Error> {
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
    e
  })?;
  Ok(())
}