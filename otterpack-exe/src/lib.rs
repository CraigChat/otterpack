#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod self_extract;

pub use app::TemplateApp;
pub use self_extract::*;
