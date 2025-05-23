mod chat;
mod request;
mod response;
mod route;
mod sockets;

pub use request::{DeconstructedHTTPRequest, HTTPRequest, HTTPRequestHeader};
pub use response::{http_err, http_ok, HTTPResponses, HTTPResult, Response};
pub use route::Router;
pub use sockets::*;
pub use HTTPResponses::*;

#[macro_export]
macro_rules! debg {
    ($code:expr) => {
        if cfg!(debug_assertions) {
            dbg!($code)
        } else {
            $code
        }
    };
}
