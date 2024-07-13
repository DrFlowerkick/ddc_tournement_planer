//! src/authentication/middleware.rs

use crate::error::{AppResult, Error};
use crate::session_state::{SessionError, TypedSession};
use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    FromRequest, HttpMessage,
};
use actix_web_lab::middleware::Next;
use anyhow::Context;
use sqlx::PgPool;
use std::ops::Deref;
use uuid::Uuid;

pub async fn reject_anonymous_users(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let session = {
        let (http_request, payload) = req.parts_mut();
        TypedSession::from_request(http_request, payload).await
    }?;

    match session.get_user_id()? {
        Some(user_id) => {
            req.extensions_mut().insert(UserId(user_id));
            next.call(req).await
        }
        None => Err(actix_web::Error::from(Error::from(
            SessionError::UserNotLoggedIn,
        ))),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UserId(Uuid);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for UserId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UserId {
    #[tracing::instrument(name = "Get username from UserID", skip(pool))]
    pub async fn get_username(&self, pool: &PgPool) -> AppResult<String> {
        let row = sqlx::query!(
            r#"
            SELECT username
            FROM users
            WHERE user_id = $1
            "#,
            self.0,
        )
        .fetch_optional(pool)
        .await
        .context("Failed to perform query to retrieve a username.")?;
        let username = row.map(|r| r.username).ok_or(SessionError::UserNotFound)?;
        Ok(username)
    }
}
