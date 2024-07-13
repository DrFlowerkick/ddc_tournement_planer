//! src/authentication/password.rs

// ToDo: password should never be send in clear text in PasswordFormData
// ToDo: learn how to check and hash password in browser, before sending it to server

use crate::error::error_chain_fmt;
use crate::telemetry::spawn_blocking_with_tracing;
use anyhow::Context;
use argon2::{
    password_hash::SaltString, Algorithm, Argon2, Params, PasswordHash, PasswordHasher,
    PasswordVerifier, Version,
};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

type CredsResult<T> = Result<T, CredentialsError>;

#[derive(serde::Deserialize)]
pub struct PasswordFormData {
    pub current_password: Secret<String>,
    pub new_password: Secret<String>,
    pub new_password_check: Secret<String>,
}

#[derive(thiserror::Error)]
pub enum CredentialsError {
    #[error("Username could not be found.")]
    UnknownUsername,
    #[error("Failed to verify password.")]
    PasswordVerifikationFailed(#[from] argon2::password_hash::Error),
    #[error("You entered two different new passwords - the field values must match.")]
    DifferentNewPasswords,
    #[error("The new password is invalid.")]
    InvalidNewPassword,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for CredentialsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> CredsResult<uuid::Uuid> {
    let mut user_id: Option<uuid::Uuid> = None;
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );
    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_credentials(&credentials.username, pool).await?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task.")??;
    // user_id is only set to Some, if we found credentials in database
    user_id.ok_or(CredentialsError::UnknownUsername)
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> CredsResult<()> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format.")?;
    Argon2::default().verify_password(
        password_candidate.expose_secret().as_bytes(),
        &expected_password_hash,
    )?;
    Ok(())
}

#[tracing::instrument(name = "Get stored credentials", skip(username, pool))]
async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> CredsResult<Option<(uuid::Uuid, Secret<String>)>> {
    let row = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
        username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to retrieve stored credentials.")?
    .map(|row| (row.user_id, Secret::new(row.password_hash)));
    Ok(row)
}

#[tracing::instrument(name = "Change password", skip(password, pool))]
pub async fn change_password_in_db(
    user_id: uuid::Uuid,
    password: Secret<String>,
    pool: &PgPool,
) -> CredsResult<()> {
    let password_hash = spawn_blocking_with_tracing(move || compute_password_hash(password))
        .await
        .context("Failed to spawn computation of password hash")??;
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

fn compute_password_hash(password: Secret<String>) -> CredsResult<Secret<String>> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15_000, 2, 1, None).unwrap(),
    )
    .hash_password(password.expose_secret().as_bytes(), &salt)
    .context("Failed to hash password.")?
    .to_string();
    Ok(Secret::new(password_hash))
}

pub async fn check_new_password(
    username: String,
    form: &PasswordFormData,
    pool: &PgPool,
) -> CredsResult<()> {
    // check for equal new passwords
    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        return Err(CredentialsError::DifferentNewPasswords);
    }
    let credentials = Credentials {
        username,
        password: form.current_password.to_owned(),
    };
    // validate current password
    validate_credentials(credentials, pool).await?;
    // check new password properties
    if form.new_password.expose_secret().chars().count() < 13
        || form.new_password.expose_secret().chars().count() > 128
        || form
            .new_password
            .expose_secret()
            .chars()
            .any(|c| c.is_ascii_whitespace())
    {
        return Err(CredentialsError::InvalidNewPassword);
    }
    Ok(())
}
