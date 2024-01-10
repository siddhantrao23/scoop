use serde_json::json;
use wiremock::{Mock, matchers::{any, method, path}, ResponseTemplate};

use crate::helpers::{spawn_app, TestApp, ConfirmationLinks, assert_is_redirect_to};

async fn create_uncomfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
  let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
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
  });

  let response = app.post_submit_newsletter(newsletter_req_body).await;
  assert_is_redirect_to(&response, "/admin/newsletters");

  let html_page = app.get_publish_newsletter_html().await;
  assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"))
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
  });
  
  let response = app.post_submit_newsletter(newsletter_req_body).await;
  assert_is_redirect_to(&response, "/admin/newsletters");

  let html_page = app.get_publish_newsletter_html().await;
  assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"))
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
  })).await;

  assert_is_redirect_to(&response, "/login");
}