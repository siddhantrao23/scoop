use wiremock::{Mock, matchers::{path, method}, ResponseTemplate};

use crate::helpers::{spawn_app, assert_is_redirect_to};

#[tokio::test]
async fn subscribe_redirects_for_valid_form_data() {
  let app = spawn_app().await;
  let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
  
  Mock::given(path("/email"))
      .and(method("POST"))
      .respond_with(ResponseTemplate::new(200))
      .expect(1)
      .mount(&app.email_server)
      .await;
  
  let response = app.post_subscriptions(body.into()).await;
  assert_is_redirect_to(&response, "/subscriptions");
  let html_body = app.get_subscription_html().await;
  assert!(html_body.contains("Check your email for a verification link!"));
}

#[tokio::test]
async fn subscribe_persists_new_subscriber() {
  let app = spawn_app().await;
  let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
  
  Mock::given(path("/email"))
      .and(method("POST"))
      .respond_with(ResponseTemplate::new(200))
      .expect(1)
      .mount(&app.email_server)
      .await;
  
  let response = app.post_subscriptions(body.into()).await;
  assert_is_redirect_to(&response, "/subscriptions");
  let html_body = app.get_subscription_html().await;
  assert!(html_body.contains("Check your email for a verification link!"));
  
  let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
          .fetch_one(&app.db_pool)
          .await
          .expect("Failed to fetch saved subsciption.");
  assert_eq!(saved.email, "ursula_le_guin@gmail.com");
  assert_eq!(saved.name, "le guin");
  assert_eq!(saved.status, "pending_confirmation");
}

#[tokio::test]
async fn subscribe_returns_400_for_missing_fields() {
  let app = spawn_app().await;

  let invalid_data = vec![
    ("name=le%20guin", "missing the email"),
    ("email=ursula_le_guin%40gmail.com", "missing the name"),
    ("", "missing both name and email")
  ];
  for (body, error_msg) in invalid_data {
    let response = app.post_subscriptions(body.into()).await;
    assert_eq!(
      400,
      response.status().as_u16(),
      "The API did not fail with 400 Bad Request when the payload was {}.",
      error_msg
    );
  }
}

#[tokio::test]
async fn subscribe_returns_400_for_invalid_form_data() {
  let app = spawn_app().await;

  let invalid_data = vec![
    ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
    ("name=Ursula&email=", "empty email"),
    ("name=Ursula&email=definitely-not-an-email", "invalid email"),
  ];
  for (body, error_msg) in invalid_data {
    let response = app.post_subscriptions(body.into()).await;
    assert_eq!(
      400,
      response.status().as_u16(),
      "The API did not return a 400 Bad Request when the payload was {}.",
      error_msg
    );
  }
}

#[tokio::test]
async fn subscribe_sends_confirmation_email_for_valid_data() {
  let app = spawn_app().await;
  let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

  Mock::given(path("/email"))
      .and(method("POST"))
      .respond_with(ResponseTemplate::new(200))
      .expect(1)
      .mount(&app.email_server)
      .await;
  
  app.post_subscriptions(body.into()).await;
}

#[tokio::test]
async fn subscribe_sends_confirmation_email_with_link() {
  let app = spawn_app().await;
  let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

  Mock::given(path("/email"))
      .and(method("POST"))
      .respond_with(ResponseTemplate::new(200))
      .expect(1)
      .mount(&app.email_server)
      .await;
  
  app.post_subscriptions(body.into()).await;

  let email_request = &app.email_server.received_requests().await.unwrap()[0];
  let confirmation_links = app.get_confirmation_links(&email_request);

  assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}