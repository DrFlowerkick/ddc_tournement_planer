//! src/sessionn_state.rs

use crate::error::{error_chain_fmt, AppResult, Error};
use actix_session::{Session, SessionExt};
use actix_web::{dev::Payload, FromRequest, HttpRequest};
use std::future::{ready, Ready};
use uuid::Uuid;

#[derive(thiserror::Error)]
pub enum SessionError {
    #[error("The user has not logged in.")]
    UserNotLoggedIn,
    #[error("User not found")]
    UserNotFound,
    #[error(transparent)]
    SessionInsertError(#[from] actix_session::SessionInsertError),
    #[error(transparent)]
    SessionGetError(#[from] actix_session::SessionGetError),
}

impl std::fmt::Debug for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

pub struct TypedSession(Session);

impl TypedSession {
    const USER_ID_KEY: &'static str = "user_id";

    pub fn renew(&self) {
        self.0.renew();
    }

    pub fn insert_user_id(&self, user_id: Uuid) -> AppResult<()> {
        self.0
            .insert(Self::USER_ID_KEY, user_id)
            .map_err(SessionError::from)
            .map_err(Error::from)
    }

    pub fn get_user_id(&self) -> AppResult<Option<Uuid>> {
        self.0
            .get(Self::USER_ID_KEY)
            .map_err(SessionError::from)
            .map_err(Error::from)
    }

    pub fn log_out(self) {
        self.0.purge();
    }
}

impl FromRequest for TypedSession {
    // This is a complicated way of saying
    // "We return the same error returned by the
    // implementation of 'FromRequest' for 'Session'".
    type Error = <Session as FromRequest>::Error;
    // Rust does not yet support the `async` syntax in traits.
    // [WELL, that chanegd, but I'm a dummy and don't know how to use it her!]
    // From request expects a `Future` as return type to allow for extractors
    // that need to perform asynchronous operations (e.g. a HTTP call)
    // We do not have a `Future`, because we don't perform any I/O,
    // so we wrap `TypedSession` into `Ready` to convert it into a `Future` that
    // resolves to the wrapped value the first time it's polled by the executor.
    type Future = Ready<Result<TypedSession, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(Ok(TypedSession(req.get_session())))
    }
}
