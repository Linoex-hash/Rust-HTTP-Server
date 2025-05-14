use std::fs;

use http::{http_ok, HTTPRequest, HTTPResponses, HTTPResult, Router};
pub fn http_routes() -> Router {
    Router::new()
        .route("GET", "/$", "1.1", index)
        .and_then(|route| route.route("GET", "/style.css", "1.1", get_css))
        .and_then(|route| route.route("GET", "/functions.js", "1.1", get_js))
        .unwrap()
}

fn index(_: HTTPRequest) -> HTTPResult {
    http_ok(HTTPResponses::Html(
        fs::read_to_string("sample_page/index.html").unwrap(),
    ))
}

// Function parameter destructure. A reference can be found at (https://doc.rust-lang.org/book/ch18-01-all-the-places-for-patterns.html#function-parameters). Basically, instead of setting a variable e.g x = HTTPRequest, we can use destructuring and extract only the information we want using HTTPRequest(x, y) instead. That way, the function can use x and y in the body without having to clutter itself manually extracting the fields.
// Function takes HTTP request as a parameter, but then destructures it into the variable header since we only care about the header.
// The body is unneeded and marked with a wildcard. This means that ownership won't transfer over, for what help that may be.
fn get_css(_: HTTPRequest) -> HTTPResult {
    http_ok(HTTPResponses::Css(
        fs::read_to_string("sample_page/style.css").unwrap(),
    ))
}

fn get_js(_: HTTPRequest) -> HTTPResult {
    http_ok(HTTPResponses::JavaScript(
        fs::read_to_string("sample_page/functions.js").unwrap(),
    ))
}
