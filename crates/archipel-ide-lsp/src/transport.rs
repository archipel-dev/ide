//! JSON-RPC base protocol framing (`Content-Length` headers over stdio).

use crate::LspError;
use serde_json::Value;
use std::io::{BufRead, Write};

/// Reads one framed JSON-RPC message.
pub fn read_message(reader: &mut impl BufRead) -> Result<Value, LspError> {
    let mut content_length = None;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 {
            return Err(LspError::Protocol("language server closed output".into()));
        }
        if header == "\r\n" || header == "\n" {
            break;
        }
        let Some((name, value)) = header.split_once(':') else {
            return Err(LspError::Protocol(format!(
                "malformed header line: {}",
                header.trim_end()
            )));
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|e| LspError::Protocol(e.to_string()))?,
            );
        }
    }
    let length =
        content_length.ok_or_else(|| LspError::Protocol("missing Content-Length".into()))?;
    // A corrupt header must not make us allocate unbounded memory.
    if length > 256 * 1024 * 1024 {
        return Err(LspError::Protocol(format!(
            "message of {length} bytes exceeds the 256 MiB limit"
        )));
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    serde_json::from_slice(&body).map_err(|e| LspError::Protocol(e.to_string()))
}

/// Writes one framed JSON-RPC message.
pub fn write_message(writer: &mut impl Write, message: &Value) -> Result<(), LspError> {
    let bytes = serde_json::to_vec(message)?;
    write!(writer, "Content-Length: {}\r\n\r\n", bytes.len())?;
    writer.write_all(&bytes)?;
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn round_trips_framed_json_rpc() {
        let mut buffer = Vec::new();
        write_message(&mut buffer, &serde_json::json!({"jsonrpc":"2.0","id":1})).unwrap();
        let value = read_message(&mut BufReader::new(buffer.as_slice())).unwrap();
        assert_eq!(value["id"], 1);
    }

    #[test]
    fn accepts_extra_headers_and_case_insensitive_names() {
        let body = r#"{"jsonrpc":"2.0","id":7,"result":null}"#;
        let framed = format!(
            "content-length: {}\r\nContent-Type: application/vscode-jsonrpc; charset=utf-8\r\n\r\n{body}",
            body.len()
        );
        let value = read_message(&mut BufReader::new(framed.as_bytes())).unwrap();
        assert_eq!(value["id"], 7);
    }

    #[test]
    fn rejects_missing_or_huge_lengths() {
        assert!(read_message(&mut BufReader::new("X-Other: 1\r\n\r\n{}".as_bytes())).is_err());
        assert!(
            read_message(&mut BufReader::new(
                "Content-Length: 99999999999\r\n\r\n".as_bytes()
            ))
            .is_err()
        );
        assert!(read_message(&mut BufReader::new("".as_bytes())).is_err());
    }
}
