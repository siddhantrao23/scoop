use crate::helpers::spawn_app;
use wiremock::{ResponseTemplate, Mock};
use wiremock::matchers::{path, method};

#[tokio::test]
async fn unsubscriptions_requests_without_token_are_rejected_with_a_400() {
  let app = spawn_app().await;
  let response = reqwest::get(&format!("{}/unsubscribe", app.address))
    .await
    .unwrap();
  assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn the_unsub_link_returned_by_subscribe_returns_a_200_if_called() {
  let app = spawn_app().await;
  let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
  
  Mock::given(path("/email"))
    .and(method("POST"))
    .respond_with(ResponseTemplate::new(200))
    .mount(&app.email_server)
    .await;

  app.post_subscriptions(body.into()).await;

  let email_request = &app.email_server.received_requests().await.unwrap()[0];
  let unsubscription_link = app.get_unsubscription_link(&email_request);
  
  let response = reqwest::get(unsubscription_link)
    .await
    .unwrap();
  
  assert_eq!(response.status().as_u16(), 200);
  println!("{}", app.get_subscription_html().await);
}

#[tokio::test]
async fn unsub_link_removes_subscriber() {
  let app = spawn_app().await;
  let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
  
  Mock::given(path("/email"))
    .and(method("POST"))
    .respond_with(ResponseTemplate::new(200))
    .mount(&app.email_server)
    .await;

  app.post_subscriptions(body.into()).await;

  let email_request = &app.email_server.received_requests().await.unwrap()[0];
  let unsubscription_link = app.get_unsubscription_link(&email_request);
  
  reqwest::get(unsubscription_link)
    .await
    .unwrap()
    .error_for_status()
    .unwrap();
  
  let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
          .fetch_optional(&app.db_pool)
          .await
          .expect("Failed to fetch saved subsciption.");
  assert!(saved.is_none());
}
