use http::{
    http_err, http_ok, HTTPRequest,
    HTTPResponses::{self, *},
    HTTPResult, Router,
};
pub fn http_routes() -> Router {
    Router::new()
        .route("GET|POST", "/$", "1.1", hello_world)
        .and_then(|route| route.route("POST", "/image$", "1.1", get_image))
        .and_then(|route| route.route("POST", "/user_json$", "1.1", print_json))
        .and_then(|route| route.route("GET|POST", "/custom$", "1.1", custom_route))
        .unwrap()
}

fn hello_world(_: HTTPRequest) -> HTTPResult {
    http_ok("Hello, world!".into())
}

// Function parameter destructure. A reference can be found at (https://doc.rust-lang.org/book/ch18-01-all-the-places-for-patterns.html#function-parameters). Basically, instead of setting a variable e.g x = HTTPRequest, we can use destructuring and extract only the information we want using HTTPRequest(x, y) instead. That way, the function can use x and y in the body without having to clutter itself manually extracting the fields.
// Function takes HTTP request as a parameter, but then destructures it into the variable header since we only care about the header.
// The body is unneeded and marked with a wildcard. This means that ownership won't transfer over, for what help that may be.
fn custom_route(HTTPRequest(headers, _): HTTPRequest) -> HTTPResult {
    println!("Headers => {headers:?}");
    http_ok(Custom {
        code: 201,
        message: "Created".to_owned(),
        ctype: "text/plain".to_owned(),
        headers: None,
        body: Vec::from("Something has occured!".as_bytes()),
    })
}

fn get_image(HTTPRequest(_, body): HTTPRequest) -> HTTPResult {
    println!("Body Length: {}", body.len());
    http_ok(Redirect("/".to_owned()))
}

fn print_json(HTTPRequest(_, body): HTTPRequest) -> HTTPResult {
    println!(
        "Json Receieved: {}",
        String::from_utf8(body).map_err(|err| {
            eprintln!("Could not format the body as JSON => {err}");
            HTTPResponses::internal_server_error()
        })?
    );
    http_err(HTTPError {
        status_code: 500,
        message: "Some Server Error".to_owned(),
        body: "Congrats, you've broken our site!".to_owned(),
    })
}
