use actix_web::{HttpResponse, web::{self}};
use sqlx::PgPool;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberName, SubscriberEmail};

#[derive(serde::Deserialize)]
pub struct FormData {
  email: String,
  name: String
}

#[tracing::instrument(
  name = "Adding a new subscriber",
  skip(form, pool),
  fields(
    subscriber_name = %form.name,
    subscriber_email = %form.email 
  )
)]
pub async fn subscribe(
        form: web::Form<FormData>,
        pool: web::Data<PgPool>
) -> HttpResponse {
  let name = match SubscriberName::parse(form.0.name) {
    Ok(name) => name,
    Err(_) => return HttpResponse::BadRequest().finish(),
  };
  
  let email = match SubscriberEmail::parse(form.0.email) {
    Ok(email) => email,
    Err(_) => return HttpResponse::BadRequest().finish(),
  };

  let new_sub = NewSubscriber { email, name };

  match insert_subscriber(&pool, &new_sub).await {
    Ok(_) => HttpResponse::Ok().finish(),
    Err(_) => HttpResponse::InternalServerError().finish(),
  }
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
    INSERT INTO subscriptions (id, email, name, subscribed_at)
    VALUES ($1, $2, $3, $4)
    "#,
    Uuid::new_v4(),
    new_sub.email.as_ref()  ,
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