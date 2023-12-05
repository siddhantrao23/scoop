use actix_web::{HttpResponse, web::{self}};
use sqlx::PgPool;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberName};

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
  let new_sub = NewSubscriber {
    email: form.0.email,
    name: SubscriberName::parse(form.0.name),
  };

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
    new_sub.email,
    new_sub.name.inner_ref(),
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