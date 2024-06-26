use crate::helpers::{spawn_app, assert_is_redirect_to};

#[tokio::test]
async fn an_error_flash_message_is_set_on_error() {
  let app = spawn_app().await;

  let login_body = serde_json::json!({
    "username": "random-username",
    "password": "random-password",
  });
  let response = app.post_login(&login_body).await;
  assert_is_redirect_to(&response, "/login");
  
  let html_page = app.get_login_html().await;
  assert!(html_page.contains("Authentication failed"));
}

#[tokio::test]
async fn successfull_login_redirects_to_admin_dashboard() {
  let app = spawn_app().await;

  let response = app.test_user.login(&app).await;
  assert_is_redirect_to(&response, "/admin/dashboard");
  
  let html_page = app.get_admin_dashboard_html().await;
  assert!(html_page.contains(&format!("Welcome {}", app.test_user.username)));
}