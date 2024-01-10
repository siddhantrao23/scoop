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
    writeln!(msg_html, "<p><i>{}</i></p>", m.content()).unwrap();
  }
  Ok(HttpResponse::Ok()
    .content_type(ContentType::html())
    .body(format!(
      r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta http-equiv="content-type" content="text/html; charset=utf-8">
  <title>Submit Newsletter</title>
</head>
<body>
  {msg_html}
  <form action="/admin/newsletters" method="post">
    <label>Title
    <input
      type="text"
      placeholder="Enter newsletter title"
      name="title"
    >
    </label>
    <br>
    <label>Plain Text Content
    <textarea
      placeholder="Enter the content in plain text"
      name="text_content"
      rows="20"
      cols="50"
    ></textarea>
    </label>
    <br>
    <label>HTML Content
    <textarea
      placeholder="Enter the content in HTML format"
      name="html_content"
      rows="20"
      cols="50"
    ></textarea>
    </label>
    <br>
    <button type="submit">Publish Newsletter</button>
  </form>
  <p><a href="/admin/dashboard">&lt;- Back</a></p>
</body>
</html>"#,
  )))
}