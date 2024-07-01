//! src/domain/user_token.rs

use crate::domain::ValidationError;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

#[derive(serde::Deserialize, Debug, Clone)]
pub struct UserToken {
    user_token: String,
}

impl AsRef<str> for UserToken {
    fn as_ref(&self) -> &str {
        &self.user_token
    }
}

impl UserToken {
    /// Generate a random 25-characters-long case-sensitive user token.
    pub fn generate_user_token() -> Self {
        let mut rng = thread_rng();
        Self {
            user_token: std::iter::repeat_with(|| rng.sample(Alphanumeric))
                .map(char::from)
                .take(25)
                .collect(),
        }
    }
    /// check if any char of user_token is not alphanumeric
    pub fn is_valid(&self) -> Result<&str, ValidationError> {
        if self
            .user_token
            .chars()
            .any(|c| !c.is_alphanumeric())
            || self.user_token.chars().count() != 25
        {
            Err(ValidationError::InvalidToken(
                self.user_token.to_owned(),
            ))
        } else {
            Ok(&self.user_token)
        }
    }
    /// parse string as token
    pub fn parse(s: String) -> Result<UserToken, ValidationError> {
        let user_token = Self {
            user_token: s,
        };
        user_token.is_valid()?;
        Ok(user_token)
    }
}
