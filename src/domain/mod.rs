//! src/domain/mod.rs

mod user_email;
mod user_name;
mod user_token;

pub use user_email::UserEmail;
pub use user_name::UserName;
pub use user_token::UserToken;

/// Validation error for domain data
#[derive(thiserror::Error, Debug)]
pub enum ValidationError {
    #[error("`{0}` is not a valid user email.")]
    InvalidEmail(String),
    #[error("`{0}` is not a valid user name.")]
    InvalidName(String),
    #[error("`{0}` is not a valid user token.")]
    InvalidToken(String),
}
