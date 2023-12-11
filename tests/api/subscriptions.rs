use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
  let app = spawn_app().await;
  let client = reqwest::Client::new();

  let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
  let response = client
    .post(&format!("{}/subscriptions", &app.address))
    .header("Content-Type", "application/x-www-form-urlencoded")
    .body(body)
    .send()
    .await
    .expect("Failed to send request.");

  assert_eq!(200, response.status().as_u16());
  let saved = sqlx::query!("SELECT email, name FROM subscriptions")
          .fetch_one(&app.db_pool)
          .await
          .expect("Failed to fetch saved subsciption.");
  assert_eq!(saved.email, "ursula_le_guin@gmail.com");
  assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn subscribe_returns_400_for_missing_fields() {
  let app = spawn_app().await;
  let client = reqwest::Client::new();

  let invalid_data = vec![
    ("name=le%20guin", "missing the email"),
    ("email=ursula_le_guin%40gmail.com", "missing the name"),
    ("", "missing both name and email")
  ];
  for (body, error_msg) in invalid_data {
    let response = client
      .post(&format!("{}/subscriptions", &app.address))
      .header("Content-Type", "application/x-www-form-urlencoded")
      .body(body)
      .send()
      .await
      .expect("Failed to send request.");

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
  let client = reqwest::Client::new();

  let invalid_data = vec![
    ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
    ("name=Ursula&email=", "empty email"),
    ("name=Ursula&email=definitely-not-an-email", "invalid email"),
  ];
  for (body, error_msg) in invalid_data {
    let response = client
      .post(&format!("{}/subscriptions", &app.address))
      .header("Content-Type", "application/x-www-form-urlencoded")
      .body(body)
      .send()
      .await
      .expect("Failed to send request.");

    assert_eq!(
      400,
      response.status().as_u16(),
      "The API did not return a 400 Bad Request when the payload was {}.",
      error_msg
    );
  }
}

