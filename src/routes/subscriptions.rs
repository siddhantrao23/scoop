use actix_web::{HttpResponse, web};
use sqlx::PgPool;
use chrono::Utc;
use uuid::Uuid;

use crate::{
  domain::{NewSubscriber, SubscriberName, SubscriberEmail}, 
  email_client::EmailClient
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

#[tracing::instrument(
  name = "Adding a new subscriber",
  skip(form, pool, email_client),
  fields(
    subscriber_name = %form.name,
    subscriber_email = %form.email 
  )
)]
pub async fn subscribe(
  form: web::Form<FormData>,
  pool: web::Data<PgPool>,
  email_client: web::Data<EmailClient>,
) -> HttpResponse {
  let new_sub = match NewSubscriber::try_from(form.0) {
    Ok(new_sub) => new_sub,
    Err(_) => return HttpResponse::BadRequest().finish(),
  };

  if insert_subscriber(&pool, &new_sub).await.is_err() {
    println!("failed to insert.");
    return HttpResponse::InternalServerError().finish()
  }

  if send_confirmation_email(&email_client, new_sub).await.is_err() {
    println!("failed to send email.");
    return HttpResponse::InternalServerError().finish()
  } 
  HttpResponse::Ok().finish()
}

#[tracing::instrument(
  name = "Send a confirmation email to new subscribers.",
  skip(email_client, new_sub),
)]
pub async fn send_confirmation_email(
  email_client: &EmailClient,
  new_sub: NewSubscriber,
) -> Result<(), reqwest::Error> {
  let confirmation_link = "http://random-domain.com/subscriptions/confirm";
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
  skip(new_sub, pool)
)]
pub async fn insert_subscriber(
  pool: &PgPool, 
  new_sub: &NewSubscriber
) -> Result<(), sqlx::Error> {
  sqlx::query!(r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at, status)
    VALUES ($1, $2, $3, $4, 'pending_confirmation')
    "#,
    Uuid::new_v4(),
    new_sub.email.as_ref(),
    new_sub.name.as_ref(),
    Utc::now(),
  )
  .execute(pool)
  .await
  .map_err(|e| {
    tracing::error!("Failed to execute query: {:?}", e);
    e
  })?;
  Ok(())
} 