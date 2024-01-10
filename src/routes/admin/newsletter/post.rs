use crate::utils::see_other;
use crate::{utils::e500, domain::SubscriberEmail};
use crate::email_client::EmailClient;
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use serde::Deserialize;
use sqlx::PgPool;

struct ConfirmedSubscriber {
  email: SubscriberEmail
}

#[derive(Deserialize)]
pub struct FormData {
  title: String,
  text_content: String,
  html_content: String,
}

#[tracing::instrument(
  name = "Publishing a newsletter issue",
  skip(form, pool, email_client),
  fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
  form: web::Form<FormData>, 
  pool: web::Data<PgPool>,
  email_client: web::Data<EmailClient>,
)
-> Result<HttpResponse, actix_web::Error> {
  let subscribers = get_confirmed_subscribers(&pool).await.map_err(e500)?;
  for subscriber in subscribers {
    match subscriber {
      Ok(subscriber) => {
        email_client.send_email(
          &subscriber.email, 
          &form.title, 
          &form.text_content,
          &form.html_content,
        )
        .await
        .with_context(|| {
          format!("Failed to send newsletter to {:?}", subscriber.email)
        })
        .map_err(e500)?;
      },
      Err(error) => {
        tracing::warn!(
          error.cause_chain = ?error,
            "Skipping a confirmed subscriber. \
            Their stored contact details are invalid",
        )
      }
    }
  }
  FlashMessage::info("The newsletter issue has been published!").send();
  Ok(see_other("/admin/newsletters"))
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
  pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
  let confirmed_subscribers = sqlx::query!(
    r#"
    SELECT email
    FROM subscriptions
    WHERE status = 'confirmed'
    "#,
  )
  .fetch_all(pool)
  .await?
  .into_iter()
  .map(|r| match  SubscriberEmail::parse(r.email) {
    Ok(email) => Ok(ConfirmedSubscriber { email }),
    Err(error) => Err(anyhow::anyhow!(error)),
  })
  .collect();

  Ok(confirmed_subscribers)
}
