//! src/utils.rs

use actix_web::{http::header::LOCATION, HttpResponse};

/// forward to other location
pub fn see_other(location: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, location))
        .finish()
}
