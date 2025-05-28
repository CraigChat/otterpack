#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod process;
mod self_extract;

pub use app::TemplateApp;
pub use process::*;
pub use self_extract::*;
