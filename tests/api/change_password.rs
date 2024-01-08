use serde_json::json;

use crate::helpers::{spawn_app, assert_is_redirect_to};

#[tokio::test]
async fn must_be_logged_in_to_see_password_change_form() {
  let app = spawn_app().await;

  let response = app.get_change_password().await;

  assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn must_be_logged_in_to_change_password() {
  let app = spawn_app().await;
  let new_password = uuid::Uuid::new_v4().to_string();

  let response = app.post_change_password(
    &json!({
      "current_password": uuid::Uuid::new_v4().to_string(),
      "new_password": &new_password,
      "new_password_check": &new_password,
  }))
  .await;

  assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn both_passwords_must_match() {
  let app = spawn_app().await;

  let new_password = uuid::Uuid::new_v4().to_string();
  let another_new_password = uuid::Uuid::new_v4().to_string();

  app.post_login(&json!({
    "username": &app.test_user.username,
    "password": &app.test_user.password
  }))
  .await;

  let response = app.post_change_password(
    &json!({
      "current_password": uuid::Uuid::new_v4().to_string(),
      "new_password": &new_password,
      "new_password_check": &another_new_password,
  }))
  .await;

  assert_is_redirect_to(&response, "/admin/password");
  
  let html_page = app.get_change_password_html().await;
  assert!(html_page.contains(
    "<p><i>You entered two different new passwords - the field values must match.</i></p>"
  ));
}

#[tokio::test]
async fn current_passwords_must_be_valid() {
  let app = spawn_app().await;

  let new_password = uuid::Uuid::new_v4().to_string();
  let wrong_password = uuid::Uuid::new_v4().to_string();

  app.post_login(&json!({
    "username": &app.test_user.username,
    "password": &app.test_user.password
  }))
  .await;

  let response = app.post_change_password(
    &json!({
      "current_password": &wrong_password,
      "new_password": &new_password,
      "new_password_check": &new_password,
  }))
  .await;

  assert_is_redirect_to(&response, "/admin/password");
  
  let html_page = app.get_change_password_html().await;
  println!("{}", html_page);
  assert!(html_page.contains(
    "<p><i>The current password is incorrect.</i></p>"
  ));
}