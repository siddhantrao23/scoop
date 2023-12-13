use actix_web::{HttpResponse, web};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
#[allow(dead_code)]
pub struct Parameters {
  subscription_token: String
}

#[tracing::instrument(
  name = "Confirm a pending subscriber",
  skip(parameters, pool)
)]
pub async fn confirm(
  parameters: web::Query<Parameters>,
  pool: web::Data<PgPool>
) -> HttpResponse {
  let id = match get_subscriber_id_from_token(
    &pool,
    &parameters.subscription_token
  ).await {
    Ok(id) => id,
    Err(_) => return HttpResponse::InternalServerError().finish(),
  };

  match id {
    None => HttpResponse::Unauthorized().finish(),
    Some(id) => {
      if confirm_subscriber(&pool, id).await.is_err() {
        return HttpResponse::InternalServerError().finish()
      }
      HttpResponse::Ok().finish()
    }
  }
}

#[tracing::instrument(
  name = "Get subscriber_id from token",
  skip(pool, subcription_token)
)]
async fn get_subscriber_id_from_token(
  pool: &PgPool,
  subcription_token: &str
) -> Result<Option<Uuid>, sqlx::Error> {
  let result = sqlx::query!(r#"
    SELECT subscriber_id FROM subscription_tokens
    WHERE subscription_token = $1
    "#,
    subcription_token
  )
  .fetch_optional(pool)
  .await
  .map_err(|e| {
    tracing::error!("Failed to execute query: {:?}", e);
    e
  })?;
  Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(
  name = "Mark subscriber as confirmed.",
  skip(pool, subcriber_id)
)]
async fn confirm_subscriber(
  pool: &PgPool,
  subcriber_id: Uuid,
) -> Result<(), sqlx::Error> {
  sqlx::query!(
    r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
    subcriber_id
  )
  .fetch_optional(pool)
  .await
  .map_err(|e| {
    tracing::error!("Failed to execute query: {:?}", e);
    e
  })?;
  Ok(())
}