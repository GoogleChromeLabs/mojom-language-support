extern crate lsp_types;
extern crate serde;
extern crate serde_json;

mod definition;
mod import;
mod mojomast;
mod protocol;
mod server;

pub use server::start;
