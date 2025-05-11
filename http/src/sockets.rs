use std::collections::HashMap;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    select,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};
use uuid::Uuid;

use crate::chat::ChatMessage;
use crate::{http_ok, HTTPRequest, HTTPResponses, Response};
use base64::prelude::*;
use sha1::{self, Digest, Sha1};

const WEB_SOCK_KEY: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

const BUF_SIZE: u64 = 1024;

#[derive(Debug)]
pub enum WebSocketCommands {
    WebSocketAddConn(Uuid, UnboundedSender<Vec<u8>>),
    WebSocketRemoveConn(Uuid),
    WebSocketBroadcast(Vec<u8>),
    WebSocketBroadcastExcept(Uuid, Vec<u8>),
}

/// This structure manages the socket sessions.
pub struct WebSockets {
    web_sockets: HashMap<Uuid, UnboundedSender<Vec<u8>>>,
}

impl WebSockets {
    pub fn new() -> Self {
        Self {
            web_sockets: HashMap::default(),
        }
    }

    /// Add a socket to then session
    pub async fn add_session(&mut self, ident: Uuid, session: UnboundedSender<Vec<u8>>) {
        println!("Adding a session given by {}", ident);
        let _ = self.web_sockets.insert(ident, session);
    }

    /// Remove a socket from session
    pub async fn remove_session(&mut self, ident: Uuid) {
        println!("Removing a session given by {}", ident);
        let _ = self.web_sockets.remove(&ident);
    }

    /// Broadcast payload to all registered sockets within a session
    pub async fn broadcast(&mut self, payload: Vec<u8>) {
        for (_, session) in self.web_sockets.iter_mut() {
            let _ = session.send(payload.clone());
        }
    }

    /// Broadcast payload to all registered sockets except the one identified by ident.
    pub async fn broadcast_except(&mut self, ident: Uuid, payload: Vec<u8>) {
        for (_, session) in self
            .web_sockets
            .iter_mut()
            .filter(|&(ident_inner, _)| ident != *ident_inner)
        {
            let _ = session.send(payload.clone());
        }
    }
}

impl Default for WebSockets {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default, Debug)]
pub struct WebSocketFrame {
    fin: bool, // This is a flag that determines whether or not this packet has all the data.
    opcode: u8,
    payload_len: u64,
    masking_key: [u8; 4],
    payload_complete: bool,
}
pub struct WebSocketFrameParser;

impl WebSocketFrameParser {
    /// Parses the following frame:
    /// ```
    /// +----------------------------------------------------+
    /// | FIN | RSV1 | RSV2 | RSV3 | Opcode                  |
    /// |-----|------|------|------|-------------------------|
    /// |  1  |  1   |  1   |  1   |  4                      |
    /// +----------------------------------------------------+
    /// | Mask | Payload Length (7 bits)                     |
    /// |------|---------------------------------------------|
    /// |  1   |  7 (or 7+16/64 bits if needed)              |
    /// +----------------------------------------------------+
    /// | Masking Key (if Mask = 1)                          |
    /// | (32 bits)                                          |
    /// +----------------------------------------------------+
    /// | Payload Data (0 to 2^64-1 bytes)                   |
    /// | (Variable length)                                  |
    /// +----------------------------------------------------+
    /// ```
    pub fn parse_frame(frame_bytes: &[u8]) -> (WebSocketFrame, &[u8]) {
        let mut frame: WebSocketFrame = WebSocketFrame::default();

        frame.fin = frame_bytes[0] >> 7 & 0x1 == 1;

        frame.opcode = u8::from_be_bytes([frame_bytes[0] & 0xf]);
        frame.payload_len = u64::from_be_bytes([0, 0, 0, 0, 0, 0, 0, frame_bytes[1] & 0x7f]);

        let mut mask_begninning: usize = 2;

        if frame.payload_len == 126 {
            // 126, so we need to read the next two bytes
            frame.payload_len =
                u64::from_be_bytes([0, 0, 0, 0, 0, 0, frame_bytes[2], frame_bytes[3]]);
            mask_begninning = 4;
        } else if frame.payload_len == 127 {
            frame.payload_len = u64::from_be_bytes([
                frame_bytes[2],
                frame_bytes[3],
                frame_bytes[4],
                frame_bytes[5],
                frame_bytes[6],
                frame_bytes[7],
                frame_bytes[8],
                frame_bytes[9],
            ]);
            mask_begninning = 10;
        }

        frame.payload_complete =
            frame_bytes.len() >= 14 && frame.payload_len <= frame_bytes.len() as u64 - 14; // 14 is 4 bytes for masking key + 10 bytes for other stuff. Make sure the buffer is more than 14 bytes

        frame.masking_key = [
            frame_bytes[mask_begninning],
            frame_bytes[mask_begninning + 1],
            frame_bytes[mask_begninning + 2],
            frame_bytes[mask_begninning + 3],
        ];

        (frame, &frame_bytes[mask_begninning + 4..])
    }

    pub async fn get_payload(tcp_handler: &mut BufReader<TcpStream>) -> Option<Vec<u8>> {
        let mut buf: [u8; BUF_SIZE as usize] = [0; BUF_SIZE as usize];
        let mut bytes_read;

        bytes_read = tcp_handler.read(&mut buf).await.unwrap();

        let (frame, payload): (WebSocketFrame, &[u8]) =
            WebSocketFrameParser::parse_frame(&buf[..bytes_read]);

        if frame.opcode == 8 {
            return None;
        }

        let mut full_payload: Vec<u8> = Vec::with_capacity(frame.payload_len as usize);
        full_payload.extend_from_slice(payload);

        if frame.fin {
            if !frame.payload_complete {
                while full_payload.len() < frame.payload_len as usize {
                    bytes_read = tcp_handler.read(&mut buf).await.unwrap();
                    full_payload.extend_from_slice(&buf[..bytes_read]);
                }
            }
            WebSocketFrameParser::decode_payload(&mut full_payload, frame.masking_key);
            Some(full_payload)
        } else {
            WebSocketFrameParser::decode_payload(&mut full_payload, frame.masking_key);
            let mut frame: WebSocketFrame = Default::default(); // Have a frame that we can reuse to test if it's finished
            while !frame.fin {
                bytes_read = tcp_handler.read(&mut buf).await.unwrap();

                let res = WebSocketFrameParser::parse_frame(&buf[..bytes_read]);

                frame = res.0;
                let payload = res.1;

                let mut next_full_payload: Vec<u8> = Vec::with_capacity(frame.payload_len as usize);
                next_full_payload.extend_from_slice(payload);

                if !frame.payload_complete {
                    while next_full_payload.len() < frame.payload_len as usize {
                        bytes_read = tcp_handler.read(&mut buf).await.unwrap();
                        next_full_payload.extend_from_slice(&buf[..bytes_read]);
                    }
                }
                // decode payload and hand it to the full_payload
                Self::decode_payload(&mut next_full_payload, frame.masking_key);
            }
            Some(full_payload)
        }
    }

    /// Pack everything into a web socket frame and send a response
    pub fn send_response(payload: &[u8]) -> Vec<u8> {
        if payload.is_empty() {
            (136u8).to_be_bytes().into_iter().chain([0]).collect()
        } else if payload.len() >= (1 << 16) {
            // craft the OP code + 127 payload len + the actual payload
            (129u8)
                .to_be_bytes()
                .into_iter()
                .chain((127u8).to_be_bytes())
                .chain((payload.len() as u64).to_be_bytes())
                .chain(payload.iter().cloned())
                .collect()
        } else if payload.len() >= ((1 << 7) - 2) {
            (129u8)
                .to_be_bytes()
                .into_iter()
                .chain((126u8).to_be_bytes())
                .chain((payload.len() as u16).to_be_bytes())
                .chain(payload.iter().cloned())
                .collect()
        } else {
            (129u8)
                .to_be_bytes()
                .into_iter()
                .chain((payload.len() as u8).to_be_bytes())
                .chain(payload.iter().cloned())
                .collect()
        }
    }

    pub fn decode_payload(payload: &mut [u8], masking_key: [u8; 4]) {
        for chunk in payload.chunks_mut(4) {
            Self::__xor_payload(chunk, &masking_key)
        }
    }

    fn __xor_payload(payload_chunk: &mut [u8], masking_key: &[u8]) {
        payload_chunk
            .iter_mut()
            .zip(masking_key)
            .for_each(|(encrypted, key_frag)| *encrypted ^= *key_frag);
    }
}

struct WebSocketHandShake;

impl WebSocketHandShake {
    pub fn handshake(sec_socket_key: &str) -> Box<HTTPResponses> {
        // Hash the key with the constant web socket key
        let mut hasher = Sha1::new();
        hasher.update(format!("{sec_socket_key}{WEB_SOCK_KEY}"));

        // Encode the hash as base64
        let sec_socket_accept: String = BASE64_STANDARD.encode(hasher.finalize());
        http_ok(HTTPResponses::Custom {
            code: 101,
            message: "Switching Protocols".into(),
            ctype: "text".into(),
            headers: Some(HashMap::from([
                ("Connection".into(), "Upgrade".into()),
                ("Upgrade".into(), "websocket".into()),
                ("Sec-WebSocket-Accept".into(), sec_socket_accept),
            ])),
            body: Vec::new(),
        })
        .unwrap()
    }
}

pub async fn handle_web_sockets(
    request: HTTPRequest,
    mut tcp_handler: BufReader<TcpStream>,
    sender: UnboundedSender<WebSocketCommands>,
) {
    // Get the key and fail if it does not exist
    let key = match request.0.web_socket_key {
        Some(x) => x,
        None => {
            tcp_handler
                .write_all(&HTTPResponses::not_found().to_response())
                .await
                .unwrap();
            return;
        }
    };

    let response = WebSocketHandShake::handshake(&key);

    // Send a response to the client saying we got the web socket
    tcp_handler
        .write_all(&response.to_response())
        .await
        .unwrap();

    let uuid = uuid::Uuid::new_v4();

    // Web socket stuff
    let (tx, mut rx): (UnboundedSender<Vec<u8>>, UnboundedReceiver<Vec<u8>>) = unbounded_channel();

    sender
        .send(WebSocketCommands::WebSocketAddConn(uuid, tx))
        .unwrap();

    loop {
        select! { // we use select here because we want to listen for broadcasts
            broad_cast_payload = rx.recv() => match broad_cast_payload {
                Some(value) => tcp_handler.write_all(&value).await.unwrap(),
                None => break,
            },
            user_payload = WebSocketFrameParser::get_payload(&mut tcp_handler) => match user_payload {
                Some(value) => handle_payload(value, uuid, sender.clone()),
                None => break
            }
        };
    }

    sender
        .send(WebSocketCommands::WebSocketRemoveConn(uuid))
        .unwrap();
}

pub fn handle_payload(payload: Vec<u8>, uuid: Uuid, sender: UnboundedSender<WebSocketCommands>) {
    let decoded = String::from_utf8_lossy(&payload);
    let mut message: ChatMessage = serde_json::from_str::<ChatMessage>(&decoded).unwrap();
    message.username = uuid;
    let output = serde_json::to_string(&message)
        .map(String::into_bytes)
        .map(|res| WebSocketFrameParser::send_response(&res))
        .unwrap();
    sender
        .send(WebSocketCommands::WebSocketBroadcast(output))
        .unwrap();
}
