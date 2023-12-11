use actix_web::{HttpResponse, web};
use sqlx::PgPool;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::NewSubscriber;

#[derive(serde::Deserialize)]
pub struct FormData {
  pub email: String,
  pub name: String
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
  let new_sub = match NewSubscriber::try_from(form.0) {
    Ok(new_sub) => new_sub,
    Err(_) => return HttpResponse::BadRequest().finish(),
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