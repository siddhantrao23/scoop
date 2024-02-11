use actix_web::{HttpResponse, http::header::ContentType};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

pub async fn subscribe_form(
  flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
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
  Ok(HttpResponse::Ok()
    .content_type(ContentType::html())
    .body(format!(
      include_str!("subscribe.html"),
      msg_html
    )
  ))
}