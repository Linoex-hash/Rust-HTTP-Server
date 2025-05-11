use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt::Write;
use std::result;
use std::str::FromStr;

/// The HTTP Result type.
pub type HTTPResult = result::Result<Box<HTTPResponses>, Box<HTTPResponses>>;

pub fn http_ok(value: HTTPResponses) -> HTTPResult {
    Ok(Box::new(value))
}

pub fn http_err(err: HTTPResponses) -> HTTPResult {
    Err(Box::new(err))
}

pub trait Response {
    /// Into Response consumes self and returns a vector of bytes as a TCP stream.
    /// Intended to be flexible with future versions of Responses that may not be of HTTP
    fn to_response(self) -> Vec<u8>;
}

/// Common response types for plain text, hmtl, javascript and so on.
#[derive(Debug, PartialEq, Eq)]
pub enum HTTPResponses {
    PlainText(String),
    Html(String),
    JavaScript(String),
    Css(String),
    Json(String),
    Redirect(String),
    Image {
        ext: String,
        content: Vec<u8>,
    },
    HTTPError {
        status_code: i32,
        message: String,
        body: String,
    },
    Custom {
        code: i32,
        message: String,
        ctype: String,
        headers: Option<HashMap<String, String>>,
        body: Vec<u8>,
    },
}
/// Syntatic sugar for using [`Response::to_response`] on a [`Box<HTTPRequest>`]. Uses the `*` operator of the box pointers to dereference it and calls the `to_response` method  implemented for [`HTTPResponses`]
impl Response for Box<HTTPResponses> {
    fn to_response(self) -> Vec<u8> {
        (*self).to_response()
    }
}
/// When converting to response, if statements handle special cases. For instance, redirect's HTTP status code is different from the rest, so it needs to be handled separatley. This helps to avoid writing duplicate code.
impl Response for HTTPResponses {
    fn to_response(self) -> Vec<u8> {
        // handle the redirect case separate
        if let Self::Redirect(s) = self {
            format!(
                "HTTP/1.1 301 Moved Permanently\r\n\
                X-Content-Type-Options: nosniff\r\n\
                Location: {s}\r\n\r\n"
            )
            .into_bytes()
        } else {
            match self {
                Self::PlainText(s) => Self::craft_string_response(200, "OK", "text/plain", s),
                Self::Html(s) => {
                    Self::craft_string_response(200, "OK", "text/html; charset=utf-8", s)
                }
                Self::JavaScript(s) => Self::craft_string_response(200, "OK", "text/javascript", s),
                Self::Css(s) => Self::craft_string_response(200, "OK", "text/css", s),
                Self::Json(s) => Self::craft_string_response(200, "OK", "application/json", s),
                Self::HTTPError {
                    status_code,
                    message,
                    body,
                } => Self::craft_string_response(status_code, message.as_str(), "text/plain", body),
                Self::Image { ext, content } => Self::craft_byte_response(
                    200,
                    "OK",
                    format!("image/{ext}").as_str(),
                    None,
                    content,
                ),
                Self::Custom {
                    code,
                    message,
                    ctype,
                    headers,
                    body,
                } => {
                    Self::craft_byte_response(code, message.as_str(), ctype.as_str(), headers, body)
                }
                _ => unreachable!(),
            }
        }
    }
}
impl HTTPResponses {
    pub fn not_found() -> Box<Self> {
        Box::new(Self::HTTPError {
            status_code: 404,
            message: "Not found".to_owned(),
            body: "The requested content could not be found.".to_owned(),
        })
    }

    pub fn internal_server_error() -> Box<Self> {
        Box::new(Self::HTTPError {
            status_code: 500,
            message: "Internal Server Error".to_owned(),
            body: "The server has encountered an unexpected error.".to_owned(),
        })
    }
    // Crafts a successful 2XX response on "text" content (HTML, PlainText, Json, etc...)
    fn craft_string_response(code: i32, message: &str, ctype: &str, content: String) -> Vec<u8> {
        Self::craft_byte_response(code, message, ctype, None, content.into_bytes())
    }

    // Crafts a successful 2XX response on "byte" content (Images, etc...)
    fn craft_byte_response(
        code: i32,
        message: &str,
        ctype: &str,
        headers: Option<HashMap<String, String>>,
        mut content: Vec<u8>,
    ) -> Vec<u8> {
        let headers: String = headers
            .map(|h| {
                h.into_iter().fold(String::new(), |mut acc, (key, val)| {
                    let _ = writeln!(&mut acc, "{key}: {val}");
                    acc
                })
            })
            .unwrap_or(String::from(""));
        let mut response = format!(
            "HTTP/1.1 {code} {message}\r\n\
            X-Content-Type-Options: nosniff\r\n\
            Content-Type: {ctype}\r\n\
            {headers}\
            Content-Length: {}\r\n\r\n",
            content.len(),
        )
        .into_bytes();
        response.append(&mut content);
        response
    }
}

/// Adds functionality for `From<String>`.
/// Can use the `into` method for `String` to convert into a plaintext response. For example:
/// ```rust
/// # use http::HTTPResponses;
/// let x: HTTPResponses = String::from("Hello").into(); // Now HTTPResponses::PlainText("Hello")
/// match x {
///     HTTPResponses::PlainText(s) => assert_eq!(s, "Hello"),
///     _ => panic!("Test failed as x was not plain text")
/// }
/// ```
impl From<String> for HTTPResponses {
    fn from(value: String) -> Self {
        Self::PlainText(value)
    }
}
/// Adds functionality for `From<&str>`.
/// Can use the `into` method for `&str` to convert into a plaintext response. Calls the parse method which implicity invokes HTTPResponses' `FromStr` implementation. For example:
/// ```rust
/// # use http::HTTPResponses;
/// // Equivalent to "Hello".parse.unwrap()
/// let x: HTTPResponses = "Hello".into(); // Now HTTPResponses::PlainText("Hello")
/// match x {
///     HTTPResponses::PlainText(s) => assert_eq!(s, "Hello"),
///     _ => panic!("Test failed as x was not plain text")
/// }
/// ```
impl From<&str> for HTTPResponses {
    fn from(value: &str) -> Self {
        // This can never fail, so call unwrap without worry
        value.parse().unwrap()
    }
}

/// Adds functionality for parse. Can use the `parse()` method for `&str` to convert into a plaintext response. For example:
/// ```rust
/// use http::HTTPResponses;
/// let x: HTTPResponses = "Hello".parse().unwrap(); // Result<HTTPResponses::PlainText("Hello"), String> -> HTTPResponses::PlainText("Hello")'
/// match x {
///     HTTPResponses::PlainText(s) => assert_eq!(s, "Hello"),
///     _ => panic!("Test failed as x was not plain text")
/// }
/// ```
/// Note: This returns a result, but the result is always an `Ok` variant. Because of this, it is safe to call `unwrap()` as no error can be returned.
impl FromStr for HTTPResponses {
    type Err = Infallible;
    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        Ok(Self::from(s.to_owned()))
    }
}
