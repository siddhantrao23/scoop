use std::time::Duration;

use fake::{faker::{name::en::Name, internet::en::SafeEmail}, Fake};
use serde_json::json;
use wiremock::{Mock, matchers::{any, method, path}, ResponseTemplate, MockBuilder};

use crate::helpers::{spawn_app, TestApp, ConfirmationLinks, assert_is_redirect_to};

async fn create_uncomfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
  let name: String = Name().fake();
  let email: String = SafeEmail().fake();
  let body = serde_urlencoded::to_string(&json!({
    "name": name,
    "email": email
  }))
  .unwrap();

  let _mock_guard = Mock::given(path("/email"))
    .and(method("POST"))
    .respond_with(ResponseTemplate::new(200))
    .named("Create unconfirmed subscriber")
    .expect(1)
    .mount_as_scoped(&app.email_server)
    .await;
  app.post_subscriptions(body.into())
    .await
    .error_for_status()
    .unwrap();

  let email_request = &app.email_server
    .received_requests()
    .await
    .unwrap()
    .pop()
    .unwrap();

  app.get_confirmation_links(email_request)
}

async fn create_comfirmed_subscriber(app: &TestApp) {
  let confirmation_links = create_uncomfirmed_subscriber(app).await;
  reqwest::get(confirmation_links.html)
    .await
    .unwrap()
    .error_for_status()
    .unwrap();
}

fn when_sending_an_email() -> MockBuilder {
  Mock::given(path("/email")).and(method("POST"))
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
  let app = spawn_app().await;
  create_uncomfirmed_subscriber(&app).await;
  app.test_user.login(&app).await;

  Mock::given(any())
    .respond_with(ResponseTemplate::new(200))
    .expect(0)
    .mount(&app.email_server)
    .await;

  let newsletter_req_body = serde_json::json!({
    "title": "Newsletter title",
    "text_content": "Newsletter body as plaintext",
    "html_content": "<p>Newsletter body as HTML</p>",
    "idempotency_key": uuid::Uuid::new_v4().to_string(),
  });

  let response = app.post_submit_newsletter(newsletter_req_body).await;
  assert_is_redirect_to(&response, "/admin/newsletters");

  let html_page = app.get_publish_newsletter_html().await;
  assert!(html_page.contains("The newsletter issue has been accepted -\
    emails will go out shortly!"
  ));
  app.displatch_all_pending_emails().await;
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
  let app = spawn_app().await;
  create_comfirmed_subscriber(&app).await;
  app.test_user.login(&app).await;

  Mock::given(any())
    .respond_with(ResponseTemplate::new(200))
    .expect(1)
    .mount(&app.email_server)
    .await;

  let newsletter_req_body = serde_json::json!({
    "title": "Newsletter title",
    "text_content": "Newsletter body as plaintext",
    "html_content": "<p>Newsletter body as HTML</p>",
    "idempotency_key": uuid::Uuid::new_v4().to_string(),
  });
  
  let response = app.post_submit_newsletter(newsletter_req_body).await;
  assert_is_redirect_to(&response, "/admin/newsletters");

  let html_page = app.get_publish_newsletter_html().await;
  assert!(html_page.contains(
    "The newsletter issue has been accepted -\
    emails will go out shortly!"
  ));
  app.displatch_all_pending_emails().await;
}

#[tokio::test]
async fn must_be_logged_in_to_view_newsletters_submit_form() {
  let app = spawn_app().await;

  let response = app.get_publish_newsletter().await;
  assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn must_be_logged_in_to_post_newsletters() {
  let app = spawn_app().await;

  let response = app.post_submit_newsletter(json!({
    "title": "Newsletter title",
    "text_content": "Newsletter body as plaintext",
    "html_content": "<p>Newsletter body as HTML</p>",
    "idempotency_key": uuid::Uuid::new_v4().to_string(),
  })).await;

  assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn newsletter_creation_is_idempotent() {
  let app = spawn_app().await;
  create_comfirmed_subscriber(&app).await;
  app.test_user.login(&app).await;

    when_sending_an_email()
    .respond_with(ResponseTemplate::new(200))
    .expect(1)
    .mount(&app.email_server)
    .await;

  let newsletter_request_body = json!({
    "title": "Newsletter title",
    "text_content": "Newsletter body as plaintext",
    "html_content": "<p>Newsletter body as HTML</p>",
    "idempotency_key": uuid::Uuid::new_v4().to_string(),
  });
  let response = app.post_submit_newsletter(&newsletter_request_body).await;
  assert_is_redirect_to(&response, "/admin/newsletters");

  let html_page = app.get_publish_newsletter_html().await;
  assert!(html_page.contains(
    "The newsletter issue has been accepted -\
    emails will go out shortly!"
  ));

  let response = app.post_submit_newsletter(&newsletter_request_body).await;
  assert_is_redirect_to(&response, "/admin/newsletters");

  let html_page = app.get_publish_newsletter_html().await;
  assert!(html_page.contains(
    "The newsletter issue has been accepted -\
    emails will go out shortly!"
  ));
  app.displatch_all_pending_emails().await;
}

#[tokio::test]
async fn concurrent_form_submission_is_handled_gracefully() {
  let app = spawn_app().await;
  create_comfirmed_subscriber(&app).await;
  app.test_user.login(&app).await;

  when_sending_an_email()
    .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
    .expect(1)
    .mount(&app.email_server)
    .await;

  let newsletter_request_body = json!({
    "title": "Newsletter title",
    "text_content": "Newsletter body as plaintext",
    "html_content": "<p>Newsletter body as HTML</p>",
    "idempotency_key": uuid::Uuid::new_v4().to_string(),
  });
  let response1 = app.post_submit_newsletter(&newsletter_request_body);
  let response2 = app.post_submit_newsletter(&newsletter_request_body);
  let (response1, response2) = tokio::join!(response1, response2);

  assert_eq!(response1.status(), response2.status());
  assert_eq!(response1.text().await.unwrap(), response2.text().await.unwrap());
  app.displatch_all_pending_emails().await;
}