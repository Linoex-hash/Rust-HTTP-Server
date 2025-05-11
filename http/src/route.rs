use super::{HTTPRequest, HTTPResponses, HTTPResult, Response};

// import the Regex and Regex Error package
use regex::{Error, Regex};
use std::result;

#[derive(Debug)]
struct InternalRoute {
    method: Regex,
    path: Regex,
    http_version: String,
    callback: fn(HTTPRequest) -> HTTPResult,
}

/// In this partial eq implementation, we use function parameter pattern matchin (see https://doc.rust-lang.org/book/ch18-01-all-the-places-for-patterns.html#function-parameters) to extract only the headers, which we then use for comparison.
/// The eq method still takes the whole, request, but we only care about the headers and wildcard the body.
impl PartialEq<HTTPRequest> for InternalRoute {
    fn eq(&self, HTTPRequest(other, _): &HTTPRequest) -> bool {
        self.method.is_match_at(&other.method, 0)
            && self.path.is_match_at(&other.path, 0)
            && self.http_version == other.http_version
    }
}

// This eq has symmetry as long as Internal route implements partial EQ on HTTP Request
impl PartialEq<InternalRoute> for HTTPRequest
where
    InternalRoute: PartialEq<Self>,
{
    fn eq(&self, other: &InternalRoute) -> bool {
        other.eq(self)
    }
}

pub struct Router {
    internal_route_vec: Vec<InternalRoute>,
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    pub fn new() -> Self {
        Self {
            internal_route_vec: Vec::new(),
        }
    }

    /// Consumes self and other router and attaches other router's routes to current router
    pub fn with(mut self, mut other: Router) -> Self {
        self.internal_route_vec
            .append(&mut other.internal_route_vec);
        self
    }
    /// Registers a route in the router object. Consumes self and returns it back in either an Ok variant or an error when parsing the path
    /// # Parameters
    ///  * method      : The method name. This is matched as a string.
    ///  * path        : A regular expression string. This is matched as a regex and regex tokens may be included.
    ///  * http_version: This is matched as a string. The HTTP Version
    ///  * callback    : A function poiner that accepts an HTTP request and a vector of bytes being the body of the request. Return a Result variant comprising of Ok(good response) or Err(Error Response)            
    pub fn route(
        mut self,
        method: &str,
        path: &str,
        http_version: &str,
        callback: fn(HTTPRequest) -> HTTPResult,
    ) -> result::Result<Self, Error> {
        self.internal_route_vec.push(InternalRoute {
            method: Regex::new(method)?,
            path: Regex::new(path)?,
            http_version: http_version.to_owned(),
            callback,
        });

        Ok(self)
    }

    /// Takes a mutable reference to self, consumes an HTTPRequest and body and returns a vector of bytes which is an HTTP Response encoded.
    /// If there aren't any routes that handle the request, then an `HTTP 404` error is returned. Additional errors may be returned from the callback of the route that handles the request.
    /// Is async, so it returns a [`Future`] with a [`Vec<u8>`] output.
    pub async fn handle_request(&self, request: HTTPRequest) -> Vec<u8> {
        self.internal_route_vec
            .iter()
            .find(|route| route == &&request)
            .ok_or(HTTPResponses::not_found())
            .and_then(|route| (route.callback)(request))
            .map_or_else(Response::to_response, Response::to_response)
    }
}
