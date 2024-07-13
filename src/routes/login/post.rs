//! src/routes/login/post.rs

use crate::authentication::{validate_credentials, Credentials};
use crate::error::{AppResult, Error};
use crate::session_state::TypedSession;
use crate::utils::see_other;
use actix_web::{web, HttpResponse};
use secrecy::Secret;
use sqlx::PgPool;

// ToDo: password should never be send in clear text in FormData
// ToDo: learn how to check and hash password in browser, before sending it to server

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(
    skip(form, pool, session),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    session: TypedSession,
) -> AppResult<HttpResponse> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    // mask CredentialsError with anonymous LoginError to prevent leakage of
    // information about a failed user login.
    let user_id = validate_credentials(credentials, &pool)
        .await
        .map_err(|_| Error::LoginError)?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
    session.renew();
    session.insert_user_id(user_id)?;
    Ok(see_other("/restricted/dashboard"))
}
