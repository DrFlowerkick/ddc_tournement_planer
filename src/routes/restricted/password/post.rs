//! src/routes/restricted/password/post.rs

use crate::authentication::{change_password_in_db, check_new_password, PasswordFormData, UserId};
use crate::error::AppResult;
use crate::utils::see_other;
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use sqlx::PgPool;

pub async fn change_password(
    form: web::Form<PasswordFormData>,
    user_id: web::ReqData<UserId>,
    pool: web::Data<PgPool>,
) -> AppResult<HttpResponse> {
    let username = user_id.get_username(&pool).await?;
    let user_id = user_id.into_inner();
    // first check new password
    check_new_password(username, &form, &pool).await?;
    // than change password in db
    change_password_in_db(*user_id, form.0.new_password, &pool).await?;
    FlashMessage::info("Your password has been changed.").send();
    Ok(see_other("/restricted/password"))
}
