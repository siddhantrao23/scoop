use std::net::TcpListener;

use actix_session::storage::RedisSessionStore;
use actix_web::cookie::Key;
use actix_web::web::Data;
use actix_web::{HttpServer, web, App};
use actix_web::dev::Server;
use actix_web_flash_messages::FlashMessagesFramework;
use actix_web_flash_messages::storage::CookieMessageStore;
use actix_session::SessionMiddleware;
use actix_web_lab::middleware::from_fn;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tracing_actix_web::TracingLogger;

use crate::authentication::reject_anonymous_users;
use crate::configuration::{Settings, DatabaseSettings};
use crate::routes::{home, login_form, login, admin_dashboard, change_password_form, change_password, log_out, publish_newsletter, newsletter_form};
use crate::email_client::EmailClient;
use crate::routes::{health_check, subscribe, confirm};

pub struct Application {
  port: u16,
  server: Server,
}

impl Application {
  pub async fn build(configuration: Settings) -> Result<Application, anyhow::Error> {
    let sender_email = configuration.email_client.sender()
        .expect("Invalid sender email address.");
    let timeout = configuration.email_client.timeout();
    let email_client = EmailClient::new(
        sender_email,
        configuration.email_client.base_url,
        configuration.email_client.auth_token,
        timeout,
    );

    let connection_pool = get_connection_pool(&configuration.database);
    let address = format!("{}:{}", 
        configuration.application.host, configuration.application.port);
    let listener = TcpListener::bind(address)?;
    let port = listener.local_addr().unwrap().port();
    
    let server = run(
      listener,
      connection_pool,
      email_client,
      configuration.application.base_url,
      configuration.application.hmac_secret,
      configuration.redis_uri,
    ).await?;

    Ok(Self { port, server })
  }

  pub fn port(&self) -> u16 {
    self.port
  }

  pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
    self.server.await
  }
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
  PgPoolOptions::new()
    .acquire_timeout(std::time::Duration::from_secs(2))
    .connect_lazy(
      &configuration.connection_string().expose_secret()
    )
    .expect("Failed to connect to postgres.")
}

pub struct ApplicationBaseUrl(pub String);
pub struct HmacSecret(pub Secret<String>);

async fn run(
  listener: TcpListener,
  db_pool: PgPool,
  email_client: EmailClient,
  base_url: String,
  hmac_secret: Secret<String>,
  redis_uri: Secret<String>,
) -> Result<Server, anyhow::Error> {
  let db_pool = web::Data::new(db_pool);
  let email_client = web::Data::new(email_client);
  let base_url = web::Data::new(ApplicationBaseUrl(base_url));
  
  let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());
  let message_store = CookieMessageStore::builder(
    Key::from(hmac_secret.expose_secret().as_bytes())
  ).build();
  let message_framework = FlashMessagesFramework::builder(message_store).build();
  let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;
  let server = HttpServer::new(move || {
      App::new()
          .wrap(TracingLogger::default())
          .wrap(message_framework.clone())
          .wrap(SessionMiddleware::new(redis_store.clone(), secret_key.clone()))
          .route("/", web::get().to(home))
          .route("/health_check", web::get().to(health_check))
          .route("/login", web::get().to(login_form))
          .route("/login", web::post().to(login))
          .route("/subscriptions", web::post().to(subscribe))
          .route("/subscriptions/confirm", web::get().to(confirm))
          .service(
            web::scope("/admin")
              .wrap(from_fn(reject_anonymous_users))
              .route("/dashboard", web::get().to(admin_dashboard))
              .route("/password", web::get().to(change_password_form))
              .route("/password", web::post().to(change_password))
              .route("/logout", web::post().to(log_out))
              .route("/newsletters", web::get().to(newsletter_form))
              .route("/newsletters", web::post().to(publish_newsletter))
          )
          .app_data(db_pool.clone())
          .app_data(email_client.clone())
          .app_data(base_url.clone())
          .app_data(Data::new(HmacSecret(hmac_secret.clone())))
    })
  .listen(listener)?
  .run();

  Ok(server)
}