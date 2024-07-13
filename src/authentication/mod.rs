//! src/authentication/mod.rs

mod middleware;
mod password;

pub use middleware::{reject_anonymous_users, UserId};
pub use password::{
    change_password_in_db, check_new_password, validate_credentials, Credentials, CredentialsError,
    PasswordFormData,
};
