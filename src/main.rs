mod parser;
mod static_routes;
use clap::Parser;
use http::{
    handle_web_sockets, DeconstructedHTTPRequest, HTTPRequest, Router, WebSocketCommands,
    WebSockets,
};
use parser::HTTPArgs;
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
};

const BUF_SIZE: usize = 1024;
const RETRIES: u8 = 5;
async fn handle_connection(
    stream: TcpStream,
    router: Arc<Router>,
    web_socket_sender: UnboundedSender<WebSocketCommands>,
) {
    let mut buf: [u8; BUF_SIZE] = [0; BUF_SIZE];
    let mut stream = BufReader::new(stream);
    let request_size = stream
        .read(buf.as_mut_slice())
        .await
        .expect("Could not read from stream!");

    let DeconstructedHTTPRequest(request_line, body_start) = buf
        .as_slice()
        .try_into()
        .expect("Could not convert buffer to HTTP Request");

    println!("Request Line => {request_line:?}");

    if request_line.path == "/websocket" {
        handle_web_sockets(
            HTTPRequest(request_line, Vec::new()),
            stream,
            web_socket_sender,
        )
        .await
    } else {
        let mut body: Vec<u8> = Vec::with_capacity(request_line.content_length.unwrap_or_default());

        // finish the stream if body length < content_length
        // Retry 3 times if failed to get current stream
        if let Some(content_length) = request_line.content_length {
            let mut retries = 1;
            body.extend_from_slice(&buf[body_start..request_size]);
            while body.len() < content_length && retries <= RETRIES {
                let request_size = stream.read(buf.as_mut_slice()).await.unwrap_or_else(|err| {
                    retries += 1;
                    eprintln!("Error finishing body stream. Was able to read {} bytes out of {content_length} bytes.
                    Trying again. This is attempt {retries} out of {RETRIES}.
                    Error message: {err}", body.len());             
                    0
                });
                body.extend_from_slice(&buf[..request_size]);
            }
        }
        println!("Body Length => {}", body.len());

        let response = router.handle_request(HTTPRequest(request_line, body)).await;
        stream
            .write_all(response.as_slice())
            .await
            .expect("Error writing response");
    }
    // allocate a vector of bytes that has a capcity of the content length if it exists or 0
}

#[tokio::main]
async fn main() {
    let HTTPArgs { ip_addr, port } = parser::HTTPArgs::parse();
    let listener = TcpListener::bind({
        let address = format!(
            "{}:{}",
            ip_addr.unwrap_or("0.0.0.0".to_owned()),
            port.unwrap_or(8080)
        );
        println!("Starting server on {address}");
        address
    })
    .await
    .expect("Error binding to tcp socket.");

    // Webs socket stuff
    let (tx, mut rx): (
        UnboundedSender<WebSocketCommands>,
        UnboundedReceiver<WebSocketCommands>,
    ) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        let mut socket_session: WebSockets = WebSockets::default();
        while let Some(command) = rx.recv().await {
            match command {
                WebSocketCommands::WebSocketAddConn(uuid, mutex) => {
                    socket_session.add_session(uuid, mutex).await
                }
                WebSocketCommands::WebSocketRemoveConn(uuid) => {
                    socket_session.remove_session(uuid).await
                }
                WebSocketCommands::WebSocketBroadcast(items) => {
                    socket_session.broadcast(items).await
                }
                WebSocketCommands::WebSocketBroadcastExcept(uuid, items) => {
                    socket_session.broadcast_except(uuid, items).await
                }
            }
        }
    });
    let router: Arc<Router> = Arc::new(Router::new().with(static_routes::http_routes()));

    loop {
        let (socket, _) = listener
            .accept()
            .await
            .expect("Error unwraping the listener");
        let routeref = Arc::clone(&router);
        let tx_cloned = tx.clone();
        tokio::spawn(async move {
            handle_connection(socket, routeref, tx_cloned).await;
        });
    }
}

#[cfg(test)]
mod test {}
