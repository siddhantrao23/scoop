use actix_web::{HttpResponse, http::header::ContentType};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

pub async fn subscribe_form(
  flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
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
  <form action="/subscriptions" method="post">
    <label>Name
    <input
      type="text"
      placeholder="Enter Your Name"
      name="name"
    >
    </label>
    <br>
    <label>Email
    <input
      type="text"
      placeholder="Enter Your Email Address"
      name="email"
    >
    </label>
    <br>
    <button type="submit">Submit!</button>
  </form>
  <p><a href="/">&lt;- Back</a></p>
</body>
</html>"#,
  )))
}