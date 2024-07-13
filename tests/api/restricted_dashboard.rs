//! tests/api/restricted_dashboard.rs

use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn you_must_be_logged_in_to_access_the_restricted_dashboard() {
    // Arrange
    let test_app = spawn_app().await;

    // Act
    let response = test_app.get_restricted_dashboard().await;

    // Assert
    assert_is_redirect_to(&response, "/login")
}
