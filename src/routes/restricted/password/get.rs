//! src/routes/restricted/password/get.rs

use actix_web::Responder;
use actix_web_flash_messages::IncomingFlashMessages;
use askama_actix::Template;

#[derive(Template)]
#[template(path = "password.html")]
struct LoginTemplate {
    flash_messages: Vec<String>,
}

pub async fn change_password_form(flash_messages: IncomingFlashMessages) -> impl Responder {
    let flash_messages: Vec<String> = flash_messages
        .iter()
        .map(|m| m.content().to_string())
        .collect();
    LoginTemplate { flash_messages }
}
