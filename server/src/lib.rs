extern crate lsp_types;
extern crate serde;
extern crate serde_json;

#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    ProtocolError(String),
    // Used to indicate an error occurred while handling a notification.
    NotificationHandleError(String),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::IoError(err)
    }
}

type Result<T> = std::result::Result<T, Error>;

mod protocol;
mod server;

pub use server::start;
