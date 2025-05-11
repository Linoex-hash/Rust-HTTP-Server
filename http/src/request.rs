use regex::{bytes::Regex as BRegex, Regex};
use std::{
    fmt,
    panic::catch_unwind,
    str::{from_utf8, FromStr},
};

#[derive(Debug, PartialEq, Eq)]
pub struct HTTPRequestHeader {
    pub method: String,
    pub path: String,
    pub http_version: String,
    pub content_length: Option<usize>,
    pub content_type: Option<String>,
    pub web_socket_key: Option<String>,
}

// Wrapper for HTTPRequestHeader and a Vec<u8> representing the body
#[derive(Debug, PartialEq, Eq)]
pub struct HTTPRequest(pub HTTPRequestHeader, pub Vec<u8>);

// Holds an HTTP Request and the index that request ends in the original byte buffer
pub struct DeconstructedHTTPRequest(pub HTTPRequestHeader, pub usize);

impl<'a> TryFrom<&'a [u8]> for DeconstructedHTTPRequest {
    type Error = String;
    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        let boundary = BRegex::new("\r\n\r\n").map_err(|err| {
            format!("Could not construct regex for double carriage new line! => {err}")
        })?;

        let value = boundary
            .splitn(value, 2)
            .next()
            .ok_or("Could not find HTTP Headers from Byte Slice")?;

        // Convert from UTF 8, map the error, then use and_then to try to convert from a string and return a result with findings.
        // Cannot use map as that will wrap the from str result within a result resulting in nested results.
        // Return a Deconstructed HTTP request containing the request and index marking the end of the headers and body beginning

        from_utf8(value)
            .map_err(|err| format!("Could not convert byte sequence to UTF-8 => {err}"))
            .and_then(HTTPRequestHeader::from_str)
            .map(|headers| DeconstructedHTTPRequest(headers, value.len() + boundary.as_str().len()))
    }
}

impl FromStr for HTTPRequestHeader {
    type Err = String;
    // Input: s as a request line, up to the first \r\n\r\n
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (first_line, rest) = s
            .split_once("\r\n")
            .ok_or("Could not find first line of HTTP Request".to_owned())?;

        let re = Regex::new(r"([A-Z]+)\s+(/[^\s]*)\s+HTTP/(\d\.\d)")
            .map_err(|err| format!("Could not get regex to parse first line => {err}"))?;

        let (_, [method, path, http_version]) = re
            .captures(first_line)
            .ok_or(format!(
                "Could not capture first line of HTTP Request: {first_line}"
            ))
            .and_then(|s| {
                catch_unwind(|| s.extract()).map_err(|err| {
                    format!("Error processing captured groups for first line: {err:?})")
                })
            })?;

        // Get Content Length
        let re = Regex::new(r"content-length: (\d+)")
            .map_err(|err| format!("Could not get regex to parse content-length => {err}"))?;

        let content_length: Option<usize> = re
            .captures(rest)
            .and_then(|s| s.get(1))
            .map(|length| length.as_str().parse().unwrap());

        //Get Content Type
        let re = Regex::new(r"Content-Type: (.+)\r\n")
            .map_err(|err| format!("Could not get regex to parse content-type => {err}"))?;

        let content_type: Option<String> = re
            .captures(rest)
            .and_then(|s| s.get(1))
            .map(|length| length.as_str().to_owned());

        //Get WebSocket Key
        let re = Regex::new(r"Sec-WebSocket-Key: (.+)\r\n")
            .map_err(|err| format!("Could not get regex to parse sec web socket key => {err}"))?;

        let web_socket_key: Option<String> = re
            .captures(rest)
            .and_then(|s| s.get(1))
            .map(|length| length.as_str().to_owned());

        Ok(HTTPRequestHeader {
            method: method.to_owned(),
            path: path.to_owned(),
            http_version: http_version.to_owned(),
            content_length,
            content_type,
            web_socket_key,
        })
    }
}

impl fmt::Display for HTTPRequestHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "HTTP Method  : {}", self.method)?;
        writeln!(f, "Path         : {}", self.path)?;
        write!(f, "HTTP Version : {}", self.http_version)
    }
}
#[cfg(test)]
mod tests {

    use super::*;
    fn new_request(
        method: &str,
        path: &str,
        http_version: &str,
        content_length: Option<usize>,
        content_type: Option<&str>,
        web_socket_key: Option<&str>,
    ) -> HTTPRequestHeader {
        HTTPRequestHeader {
            method: method.to_owned(),
            path: path.to_owned(),
            http_version: http_version.to_owned(),
            content_length,
            content_type: content_type.map(|s| s.to_owned()),
            web_socket_key: web_socket_key.map(|s| s.to_owned()),
        }
    }
    #[test]
    fn valid_get_request() {
        let test_bytes: &[u8] = &[
            71, 69, 84, 32, 47, 32, 72, 84, 84, 80, 47, 49, 46, 49, 13, 10, 97, 99, 99, 101, 112,
            116, 45, 101, 110, 99, 111, 100, 105, 110, 103, 58, 32, 103, 122, 105, 112, 44, 32,
            100, 101, 102, 108, 97, 116, 101, 44, 32, 98, 114, 13, 10, 65, 99, 99, 101, 112, 116,
            58, 32, 42, 47, 42, 13, 10, 85, 115, 101, 114, 45, 65, 103, 101, 110, 116, 58, 32, 84,
            104, 117, 110, 100, 101, 114, 32, 67, 108, 105, 101, 110, 116, 32, 40, 104, 116, 116,
            112, 115, 58, 47, 47, 119, 119, 119, 46, 116, 104, 117, 110, 100, 101, 114, 99, 108,
            105, 101, 110, 116, 46, 99, 111, 109, 41, 13, 10, 72, 111, 115, 116, 58, 32, 108, 111,
            99, 97, 108, 104, 111, 115, 116, 58, 56, 48, 56, 48, 13, 10, 67, 111, 110, 110, 101,
            99, 116, 105, 111, 110, 58, 32, 99, 108, 111, 115, 101, 13, 10, 13, 10, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let expected_answer: HTTPRequestHeader = new_request("GET", "/", "1.1", None, None, None);

        let DeconstructedHTTPRequest(actual_answer, _) = test_bytes
            .try_into()
            .expect("Could not convert byte slice into HTTP Request");

        assert_eq!(expected_answer, actual_answer);
    }
    #[test]
    fn test_valid_get_request_2() {
        let test_bytes: &[u8] = &[
            71, 69, 84, 32, 47, 104, 101, 108, 108, 111, 32, 72, 84, 84, 80, 47, 49, 46, 49, 13,
            10, 72, 111, 115, 116, 58, 32, 108, 111, 99, 97, 108, 104, 111, 115, 116, 58, 56, 48,
            56, 48, 13, 10, 85, 115, 101, 114, 45, 65, 103, 101, 110, 116, 58, 32, 77, 111, 122,
            105, 108, 108, 97, 47, 53, 46, 48, 32, 40, 87, 105, 110, 100, 111, 119, 115, 32, 78,
            84, 32, 49, 48, 46, 48, 59, 32, 87, 105, 110, 54, 52, 59, 32, 120, 54, 52, 59, 32, 114,
            118, 58, 49, 48, 57, 46, 48, 41, 32, 71, 101, 99, 107, 111, 47, 50, 48, 49, 48, 48, 49,
            48, 49, 32, 70, 105, 114, 101, 102, 111, 120, 47, 49, 49, 53, 46, 48, 13, 10, 65, 99,
            99, 101, 112, 116, 58, 32, 116, 101, 120, 116, 47, 104, 116, 109, 108, 44, 97, 112,
            112, 108, 105, 99, 97, 116, 105, 111, 110, 47, 120, 104, 116, 109, 108, 43, 120, 109,
            108, 44, 97, 112, 112, 108, 105, 99, 97, 116, 105, 111, 110, 47, 120, 109, 108, 59,
            113, 61, 48, 46, 57, 44, 105, 109, 97, 103, 101, 47, 97, 118, 105, 102, 44, 105, 109,
            97, 103, 101, 47, 119, 101, 98, 112, 44, 42, 47, 42, 59, 113, 61, 48, 46, 56, 13, 10,
            65, 99, 99, 101, 112, 116, 45, 76, 97, 110, 103, 117, 97, 103, 101, 58, 32, 101, 110,
            45, 85, 83, 44, 101, 110, 59, 113, 61, 48, 46, 53, 13, 10, 65, 99, 99, 101, 112, 116,
            45, 69, 110, 99, 111, 100, 105, 110, 103, 58, 32, 103, 122, 105, 112, 44, 32, 100, 101,
            102, 108, 97, 116, 101, 44, 32, 98, 114, 13, 10, 67, 111, 110, 110, 101, 99, 116, 105,
            111, 110, 58, 32, 107, 101, 101, 112, 45, 97, 108, 105, 118, 101, 13, 10, 85, 112, 103,
            114, 97, 100, 101, 45, 73, 110, 115, 101, 99, 117, 114, 101, 45, 82, 101, 113, 117,
            101, 115, 116, 115, 58, 32, 49, 13, 10, 83, 101, 99, 45, 70, 101, 116, 99, 104, 45, 68,
            101, 115, 116, 58, 32, 100, 111, 99, 117, 109, 101, 110, 116, 13, 10, 83, 101, 99, 45,
            70, 101, 116, 99, 104, 45, 77, 111, 100, 101, 58, 32, 110, 97, 118, 105, 103, 97, 116,
            101, 13, 10, 83, 101, 99, 45, 70, 101, 116, 99, 104, 45, 83, 105, 116, 101, 58, 32,
            110, 111, 110, 101, 13, 10, 83, 101, 99, 45, 70, 101, 116, 99, 104, 45, 85, 115, 101,
            114, 58, 32, 63, 49, 13, 10, 13, 10, 0, 0,
        ];
        let expected_answer: HTTPRequestHeader =
            new_request("GET", "/hello", "1.1", None, None, None);

        let DeconstructedHTTPRequest(actual_answer, _) = test_bytes
            .try_into()
            .expect("Could not convert byte slice into HTTP Request");

        assert_eq!(expected_answer, actual_answer);
    }
}
