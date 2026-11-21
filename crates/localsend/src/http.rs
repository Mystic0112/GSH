use serde::Serialize;
use std::collections::HashMap;
use std::io::{self, Read, Write};

const MAX_HEADER: usize = 16 * 1024;

pub(super) struct Response {
    status: u16,
    body: Vec<u8>,
    content_type: &'static str,
}
pub(super) fn empty_response(status: u16) -> Response {
    Response {
        status,
        body: Vec::new(),
        content_type: "application/octet-stream",
    }
}
pub(super) fn json_response<T: Serialize>(status: u16, body: &T) -> Response {
    Response {
        status,
        body: serde_json::to_vec(body).unwrap_or_default(),
        content_type: "application/json",
    }
}
pub(super) fn write_response(stream: &mut impl Write, response: Response) -> io::Result<()> {
    let reason = match response.status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        413 => "Payload Too Large",
        422 => "Unprocessable Content",
        500 => "Internal Server Error",
        _ => "Error",
    };
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nContent-Type: {}\r\nConnection: close\r\n\r\n",
        response.status,
        reason,
        response.body.len(),
        response.content_type
    )?;
    stream.write_all(&response.body)
}

type ParsedHeaders = (String, String, HashMap<String, String>);

pub(super) fn read_headers(stream: &mut impl Read) -> io::Result<ParsedHeaders> {
    let mut bytes = Vec::new();
    let mut buf = [0u8; 1];
    while bytes.len() < MAX_HEADER {
        stream.read_exact(&mut buf)?;
        bytes.push(buf[0]);
        if bytes.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    if !bytes.ends_with(b"\r\n\r\n") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "headers too long",
        ));
    }
    let split = bytes.len() - 4;
    let text = std::str::from_utf8(&bytes[..split])
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut lines = text.split("\r\n");
    let mut request = lines.next().unwrap_or("").split_whitespace();
    let method = request.next().unwrap_or("").to_owned();
    let target = request.next().unwrap_or("").to_owned();
    let mut headers = HashMap::new();
    for line in lines {
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_ascii_lowercase(), value.trim().to_owned());
        }
    }
    Ok((method, target, headers))
}
pub(super) fn read_body(stream: &mut impl Read, length: u64, limit: u64) -> Result<Vec<u8>, u16> {
    if length > limit {
        return Err(413);
    }
    let mut body = vec![0; length as usize];
    stream.read_exact(&mut body).map_err(|_| 400u16)?;
    Ok(body)
}
pub(super) fn parse_query(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter_map(|part| part.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}
