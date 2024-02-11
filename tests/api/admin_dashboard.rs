use crate::helpers::{spawn_app, assert_is_redirect_to};

#[tokio::test]
async fn only_logged_in_users_can_access_dashboard() {
  let app = spawn_app().await;

  let response = app.get_admin_dashboard().await;

  assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn logout_clears_session_state() {
  let app = spawn_app().await;

  let response = app.test_user.login(&app).await;
  assert_is_redirect_to(&response, "/admin/dashboard");

  let html_body = app.get_admin_dashboard_html().await;
  assert!(html_body.contains(&format!("Welcome {}", app.test_user.username)));

  let response = app.post_logout().await;
  assert_is_redirect_to(&response, "/login");

  let html_body = app.get_login_html().await;
  assert!(html_body.contains("You have successfully logged out."));

  let response = app.get_admin_dashboard().await;
  assert_is_redirect_to(&response, "/login");
}