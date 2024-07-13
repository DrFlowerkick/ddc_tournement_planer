//! error.rs

use crate::{domain::ValidationError, session_state::SessionError, utils::see_other};
use actix_web_flash_messages::FlashMessage;

pub type AppResult<T> = Result<T, Error>;

pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

#[derive(thiserror::Error)]
pub enum Error {
    #[error("Invalid input of user data.")]
    UserValidationError(#[from] ValidationError),
    #[error("Session state error")]
    SessionStateError(#[from] SessionError),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl From<Error> for actix_web::Error {
    fn from(err: Error) -> Self {
        match err {
            // ToDo: replace later with a FlashMessage and see_other redirection
            Error::UserValidationError(_) => actix_web::error::ErrorInternalServerError(err),
            Error::SessionStateError(_) => {
                FlashMessage::error(err.to_string()).send();
                let response = see_other("/login");
                actix_web::error::InternalError::from_response(err, response).into()
            }
            Error::UnexpectedError(_) => actix_web::error::ErrorInternalServerError(err),
        }
    }
}
