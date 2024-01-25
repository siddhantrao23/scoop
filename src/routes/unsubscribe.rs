use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use sqlx::PgPool;
use uuid::Uuid;

use crate::utils::see_other;

use super::{get_subscriber_id_from_token, Parameters};


#[tracing::instrument(
  name = "Unsubscribe a subscriber",
  skip(parameters, pool)
)]
pub async fn unsubscribe(
  parameters: web::Query<Parameters>,
  pool: web::Data<PgPool>,
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
      if remove_subscriber(&pool, id).await.is_err() {
        return HttpResponse::InternalServerError().finish();
      }
      FlashMessage::info("You have successfully unsubscribed from the newsletter!").send();
      see_other("/subscriptions")
    }
  }
}

async fn remove_subscriber(
  pool: &PgPool,
  subscriber_id: Uuid
) -> Result<(), sqlx::Error> {
  sqlx::query!(
    r#"
    DELETE FROM subscriptions
    WHERE id = $1
    "#,
    subscriber_id
  )
  .execute(pool)
  .await
  .map_err(|e| {
    tracing::error!("Failed to execute query: {:?}", e);
    e
  })?;

  Ok(())
}