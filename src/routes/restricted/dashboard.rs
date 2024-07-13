//! src/routes/restricted/dashboard.rs

use actix_web::{web, Responder};
use askama_actix::Template;
use sqlx::PgPool;

use crate::authentication::UserId;
use crate::error::AppResult;

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    username: String,
}

pub async fn restricted_dashboard(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> AppResult<impl Responder> {
    let username = user_id.get_username(&pool).await?;
    Ok(DashboardTemplate { username })
}
