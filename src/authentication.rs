use anyhow::Context;
use argon2::{PasswordHash, Argon2, PasswordVerifier, password_hash::SaltString, Algorithm, Version, Params, PasswordHasher};
use secrecy::{Secret, ExposeSecret};
use sqlx::PgPool;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
  #[error("Invalid Credentials")]
  InvalidCredentials(#[source] anyhow::Error),
  #[error(transparent)]
  UnexpectedError(#[from] anyhow::Error),
}

pub struct Credentials {
  pub username: String,
  pub password: Secret<String>,
}

#[tracing::instrument(name="Verify password_hash", skip(credentials, pool))]
pub async fn validate_credentials(
  credentials: Credentials,
  pool: &PgPool
) -> Result<uuid::Uuid, AuthError> {
  let row: Option<_> = sqlx::query!(
    r#"
    SELECT user_id, password_hash
    FROM users
    WHERE username = $1
    "#,
    credentials.username,
  )
  .fetch_optional(pool)
  .await
  .context("Failed to perform a query to retrieve stored user.")
  .map_err(AuthError::UnexpectedError)?;

  let (user_id, expected_password_hash) = match row {
    Some(row) => (row.user_id, row.password_hash),
    None => {
      return Err(AuthError::InvalidCredentials(anyhow::anyhow!("Unknown username")));
    }
  };

  let expected_password_hash = PasswordHash::new(&expected_password_hash)
    .context("Failed to parse hash in PHC string format.")
    .map_err(AuthError::UnexpectedError)?;

  tracing::info_span!("Verify password hash")
    .in_scope( || {
      Argon2::default()
      .verify_password(
        credentials.password.expose_secret().as_bytes(), 
        &expected_password_hash
      )
    })
    .context("Invalid password.")
    .map_err(AuthError::InvalidCredentials)?;

  Ok(user_id)
}

pub async fn change_password(
  user_id: uuid::Uuid,
  password: Secret<String>,
  pool: &PgPool
) -> Result<(), anyhow::Error> {
  let password_hash = tracing::info_span!("Change password")
    .in_scope( || {
      compute_password_hash(password)
  })
  .context("Failed to hash password")
  .map_err(AuthError::UnexpectedError)?;

  sqlx::query!(
    r#"
    UPDATE users
    SET password_hash = $1
    WHERE user_id = $2
    "#,
    password_hash.expose_secret(),
    user_id
  )
  .execute(pool)
  .await
  .context("Failed to change user's password in the database.")?;

  Ok(())
}

fn compute_password_hash(password: Secret<String>) -> Result<Secret<String>, anyhow::Error> {
  let salt = SaltString::generate(&mut rand::thread_rng());
  let password_hash = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(password.expose_secret().as_bytes(), &salt)?
    .to_string();
    Ok(Secret::new(password_hash))
}