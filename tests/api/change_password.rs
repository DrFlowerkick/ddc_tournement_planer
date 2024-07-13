//! tests/api/change_password.rs

use crate::helpers::{assert_is_redirect_to, spawn_app};
use uuid::Uuid;

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_change_password_form() {
    // Arrange
    let test_app = spawn_app().await;

    // Act
    let response = test_app.get_change_password().await;

    // Assert
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_change_password() {
    // Arrange
    let test_app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    // Act
    let response = test_app
        .post_change_password(&serde_json::json!({
            "current_password": Uuid::new_v4().to_string(),
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;

    // Assert
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn new_password_fields_must_match() {
    // Arrange
    let test_app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let another_new_password = Uuid::new_v4().to_string();

    // Act - Part 1 - Login
    test_app.test_user.login(&test_app).await;

    // Act - Part 2 - Try to change password
    let response = test_app
        .post_change_password(&serde_json::json!({
            "current_password": Uuid::new_v4().to_string(),
            "new_password": &new_password,
            "new_password_check": &another_new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/restricted/password");

    // Act - Part 3 - Follow the redirect
    let html_page = test_app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>You entered two different new passwords - the field values must match.</i></p>"
    ));
}

#[tokio::test]
async fn current_password_must_be_valid() {
    // Arrange
    let test_app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let wrong_password = Uuid::new_v4().to_string();

    // Act - Part 1 - Login
    test_app.test_user.login(&test_app).await;

    // Act - Part 2 - Try to change password
    let response = test_app
        .post_change_password(&serde_json::json!({
            "current_password": &wrong_password,
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/restricted/password");

    // Act - Part 3 - Follow the redirect
    let html_page = test_app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>Failed to verify password.</i></p>"));
}

#[tokio::test]
async fn new_password_must_be_valid() {
    // Arrange
    let test_app = spawn_app().await;
    let bad_new_passwords = [
        "",
        "0123456789a",
        "01234 6789abc",
        // 129 chars
        "0123456789_0123456789_0123456789_0123456789_0123456789_0123456789_0123456789_0123456789_0123456789_0123456789_0123456789_01234567"    
    ];

    // Act - Part 1 - Login
    test_app.test_user.login(&test_app).await;

    // Act - Part 2 - Try to change password
    for bad_password in bad_new_passwords {
        // Act - Part 2 - Try to change password
        let response = test_app
            .post_change_password(&serde_json::json!({
                "current_password": &test_app.test_user.password,
                "new_password": &bad_password.to_string(),
                "new_password_check": &bad_password.to_string(),
            }))
            .await;
        assert_is_redirect_to(&response, "/restricted/password");

        // Act - Part 3 - Follow the redirect
        let html_page = test_app.get_change_password_html().await;
        assert!(
            html_page.contains("<p><i>The new password is invalid.</i></p>"),
            "Bad password: {}",
            bad_password
        );
    }
}

#[tokio::test]
async fn logout_clears_session_state() {
    // Arrange
    let test_app = spawn_app().await;

    // Act - Part 1 - Login
    let response = test_app.test_user.login(&test_app).await;
    assert_is_redirect_to(&response, "/restricted/dashboard");

    // Act - Part 2 - Follow the redirect
    let html_page = test_app.get_restricted_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", test_app.test_user.username)));

    // Act - Part 3 - Logout
    let response = test_app.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    // Act - Part 4 - Follow the redirect
    let html_page = test_app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>You have successfully logged out.</i></p>"#));

    // Act - Part 5 - Attempt to load restricted panel
    let response = test_app.get_restricted_dashboard().await;
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn changing_password_works() {
    // Arrange
    let test_app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    // Act - Part 1 - Login
    let response = test_app.test_user.login(&test_app).await;
    assert_is_redirect_to(&response, "/restricted/dashboard");

    // Act - Part 2 - Change password
    let response = test_app
        .post_change_password(&serde_json::json!({
            "current_password": &test_app.test_user.password,
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/restricted/password");

    // Act - Part 3 - Follow the redirect
    let html_page = test_app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>Your password has been changed.</i></p>"));

    // Act - Part 4 - Logout
    let response = test_app.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    // Act - Part 5 - Follow the redirect
    let html_page = test_app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>You have successfully logged out.</i></p>"#));

    // Act - Part 6 - Login using the new password
    let login_body = serde_json::json!({
        "username": &test_app.test_user.username,
        "password": &new_password
    });
    let response = test_app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/restricted/dashboard");
}
