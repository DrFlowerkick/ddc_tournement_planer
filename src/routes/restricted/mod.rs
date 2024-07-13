//! src/routes/restricted/mod.rs

mod dashboard;
mod logout;
mod password;

pub use dashboard::restricted_dashboard;
pub use logout::log_out;
pub use password::*;
