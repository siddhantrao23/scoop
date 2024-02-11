use actix_web::{HttpResponse, http::header::ContentType};
use actix_web_flash_messages::IncomingFlashMessages;

use crate::{session_state::TypedSession, utils::{see_other, e500}};
use std::fmt::Write;

pub async fn newsletter_form(
  session: TypedSession,
  flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
  if session.get_user_id().map_err(e500)?.is_none() {
    return Ok(see_other("/login"));
  }

  let mut msg_html = String::new();
  for m in flash_messages.iter() {
    writeln!(msg_html,
      r#"
      <div class="alert alert-info">
      <strong>Info!</strong> {}
      </div>
      "#,
      m.content()
    ).unwrap();
  }
  let idempotency_key = uuid::Uuid::new_v4();
  Ok(HttpResponse::Ok()
    .content_type(ContentType::html())
    .body(format!(include_str!("newsletter.html"), msg_html, idempotency_key))
  )
}