use crate::helpers::{spawn_app, assert_is_redirect_to};

#[tokio::test]
async fn only_logged_in_users_can_access_dashboard() {
  let app = spawn_app().await;

  let response = app.get_admin_dashboard().await;

  assert_is_redirect_to(&response, "/login");
}