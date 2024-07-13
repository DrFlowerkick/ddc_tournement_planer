//! src/routes/home/mod.rs

use actix_web::Responder;
use askama_actix::Template;

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {}

pub async fn home() -> impl Responder {
    HomeTemplate {}
}
