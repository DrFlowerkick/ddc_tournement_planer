//! src/routes/restricted/logout.rs

use crate::error::AppResult;
use crate::session_state::TypedSession;
use crate::utils::see_other;
use actix_web::HttpResponse;
use actix_web_flash_messages::FlashMessage;

pub async fn log_out(session: TypedSession) -> AppResult<HttpResponse> {
    session.log_out();
    FlashMessage::info("You have successfully logged out.").send();
    Ok(see_other("/login"))
}
