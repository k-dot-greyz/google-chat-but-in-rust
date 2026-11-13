//! HTTP/1.1 messages. One request per connection. No chunked bodies.

use crate::json::{JsonError, Value};

pub const MAX_HEADER: usize = 8 * 1024;
pub const MAX_BODY: usize = 64 * 1024;

#[derive(Clone, Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub segments: Vec<String>,
}

impl Request {
    pub fn from_parts(
        method: &str,
        path: &str,
        query: &str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> Result<Self, String> {
        if body.len() > MAX_BODY {
            return Err("body exceeds 64 KiB".into());
        }
        if query.len() > 2048 {
            return Err("query is too long".into());
        }
        let method = method.to_ascii_uppercase();
        if !matches!(
            method.as_str(),
            "GET" | "HEAD" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS" | "QUERY"
        ) {
            return Err("method not accepted".into());
        }
        if path.len() > 256 || !path.starts_with('/') || path.contains('\\') || path.contains('\0')
        {
            return Err("path rejected".into());
        }
        let decoded = percent_decode(path).map_err(|detail| detail.to_string())?;
        if decoded.contains('\0') || decoded.contains('\\') {
            return Err("path rejected".into());
        }
        let mut segments = Vec::new();
        if decoded != "/" {
            for segment in decoded.trim_start_matches('/').split('/') {
                if segment.is_empty() || segment == "." || segment == ".." {
                    return Err("path rejected".into());
                }
                segments.push(segment.to_string());
            }
        }
        if segments.len() > 8 {
            return Err("path rejected".into());
        }
        let query_pairs = parse_query(query)?;
        let headers = headers
            .into_iter()
            .map(|(name, value)| (name.to_ascii_lowercase(), value))
            .collect();
        Ok(Self {
            method,
            path: decoded,
            query: query_pairs,
            headers,
            body,
            segments,
        })
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        let name = name.to_ascii_lowercase();
        self.headers
            .iter()
            .find(|(key, _)| key == &name)
            .map(|(_, value)| value.as_str())
    }

    pub fn query_one(&self, name: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    pub fn content_type(&self) -> &str {
        self.header("content-type")
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
    }
}

#[derive(Clone, Debug)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub code: String,
}

impl Response {
    pub fn json(status: u16, value: &Value) -> Self {
        Self::with(
            status,
            "application/json; charset=utf-8",
            value.to_json().into_bytes(),
            "no-store",
        )
    }

    pub fn html(status: u16, body: String) -> Self {
        Self::with(
            status,
            "text/html; charset=utf-8",
            body.into_bytes(),
            "no-store",
        )
    }

    pub fn asset(content_type: &str, body: &str) -> Self {
        Self::with(
            status_ok(),
            content_type,
            body.as_bytes().to_vec(),
            "no-cache",
        )
    }

    pub fn redirect() -> Self {
        let mut response = Self::with(303, "text/plain; charset=utf-8", Vec::new(), "no-store");
        response.headers.push(("location".into(), "/".into()));
        response
    }

    pub fn empty(status: u16) -> Self {
        Self::with(status, "text/plain; charset=utf-8", Vec::new(), "no-store")
    }

    pub fn fail(status: u16, code: &str, detail: &str) -> Self {
        let value = Value::Object(vec![
            ("ok".into(), Value::Bool(false)),
            (
                "error".into(),
                Value::Object(vec![
                    ("code".into(), Value::String(code.into())),
                    ("detail".into(), Value::String(detail.into())),
                ]),
            ),
        ]);
        Self::json(status, &value).code(code)
    }

    pub fn code(mut self, code: &str) -> Self {
        self.code = code.to_string();
        self
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    pub fn without_body(mut self) -> Self {
        self.body.clear();
        self
    }

    fn with(status: u16, content_type: &str, body: Vec<u8>, cache: &str) -> Self {
        Self {
            status,
            code: "ok".into(),
            headers: vec![
                ("content-type".into(), content_type.into()),
                ("cache-control".into(), cache.into()),
                ("x-content-type-options".into(), "nosniff".into()),
                ("referrer-policy".into(), "no-referrer".into()),
                ("x-frame-options".into(), "DENY".into()),
                (
                    "content-security-policy".into(),
                    "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'; object-src 'none'".into(),
                ),
            ],
            body,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = format!(
            "HTTP/1.1 {} {}\r\ncontent-length: {}\r\nconnection: close\r\n",
            self.status,
            reason(self.status),
            self.body.len()
        );
        for (name, value) in &self.headers {
            if name.eq_ignore_ascii_case("content-length")
                || name.eq_ignore_ascii_case("connection")
            {
                continue;
            }
            let _ = std::fmt::Write::write_fmt(&mut out, format_args!("{name}: {value}\r\n"));
        }
        out.push_str("\r\n");
        let mut bytes = out.into_bytes();
        bytes.extend_from_slice(&self.body);
        bytes
    }
}

fn status_ok() -> u16 {
    200
}

pub fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        303 => "See Other",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        411 => "Length Required",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        422 => "Unprocessable Content",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        _ => "Status",
    }
}

#[derive(Debug)]
pub enum HttpError {
    Bad(String),
    TooLarge,
    LengthRequired,
    Unsupported,
    Incomplete,
}

pub enum Parse {
    Need,
    Done(Request),
    Err(HttpError),
}

pub fn try_parse(buf: &[u8]) -> Parse {
    let Some(split) = find_header_end(buf) else {
        return if buf.len() > MAX_HEADER {
            Parse::Err(HttpError::TooLarge)
        } else {
            Parse::Need
        };
    };
    if split > MAX_HEADER {
        return Parse::Err(HttpError::TooLarge);
    }
    let header_text = match std::str::from_utf8(&buf[..split.saturating_sub(4)]) {
        Ok(text) => text,
        Err(_) => return Parse::Err(HttpError::Bad("headers are not utf-8".into())),
    };
    let mut lines = header_text.split("\r\n");
    let Some(start) = lines.next() else {
        return Parse::Err(HttpError::Bad("missing request line".into()));
    };
    let mut parts = start.split(' ');
    let (Some(method), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Parse::Err(HttpError::Bad("bad request line".into()));
    };
    if version != "HTTP/1.1" && version != "HTTP/1.0" {
        return Parse::Err(HttpError::Bad("bad version".into()));
    }
    if !target.starts_with('/') || target.starts_with("//") {
        return Parse::Err(HttpError::Bad(
            "only origin-form targets are accepted".into(),
        ));
    }
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let mut headers = Vec::new();
    let mut content_length = None;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            return Parse::Err(HttpError::Bad("line folding is not accepted".into()));
        }
        let Some((name, value)) = line.split_once(':') else {
            return Parse::Err(HttpError::Bad("bad header".into()));
        };
        if name.is_empty() || name.len() > 128 || value.len() > 4096 || !header_name(name) {
            return Parse::Err(HttpError::Bad("bad header".into()));
        }
        let name = name.to_ascii_lowercase();
        let value = value.trim().to_string();
        if name == "transfer-encoding" {
            return Parse::Err(HttpError::Unsupported);
        }
        if name == "content-length" {
            if content_length.is_some() {
                return Parse::Err(HttpError::Bad("repeated content-length".into()));
            }
            match value.parse::<usize>() {
                Ok(len) if len <= MAX_BODY => content_length = Some(len),
                Ok(_) => return Parse::Err(HttpError::TooLarge),
                Err(_) => return Parse::Err(HttpError::Bad("bad content-length".into())),
            }
        }
        if headers.len() >= 64 {
            return Parse::Err(HttpError::Bad("too many headers".into()));
        }
        headers.push((name, value));
    }
    let body_len = match content_length {
        Some(len) => len,
        None => {
            if matches!(method, "POST" | "PUT" | "PATCH" | "QUERY") {
                return Parse::Err(HttpError::LengthRequired);
            }
            0
        }
    };
    let total = split + body_len;
    if buf.len() < total {
        return Parse::Need;
    }
    let body = buf[split..total].to_vec();
    match Request::from_parts(method, path, query, headers, body) {
        Ok(request) => Parse::Done(request),
        Err(detail) => Parse::Err(HttpError::Bad(detail)),
    }
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
}

fn header_name(name: &str) -> bool {
    name.bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

pub fn parse_form(body: &str) -> Result<Vec<(String, String)>, String> {
    if body.len() > MAX_BODY {
        return Err("body exceeds 64 KiB".into());
    }
    let mut pairs = Vec::new();
    if body.is_empty() {
        return Ok(pairs);
    }
    for part in body.split('&') {
        let (key, value) = part.split_once('=').unwrap_or((part, ""));
        let key = percent_decode(&key.replace('+', " ")).map_err(|detail| detail.to_string())?;
        let value =
            percent_decode(&value.replace('+', " ")).map_err(|detail| detail.to_string())?;
        if pairs.iter().any(|(existing, _)| existing == &key) {
            return Err(format!("duplicate field `{key}`"));
        }
        pairs.push((key, value));
    }
    Ok(pairs)
}

fn parse_query(query: &str) -> Result<Vec<(String, String)>, String> {
    parse_form(query)
}

pub fn percent_decode(input: &str) -> Result<String, &'static str> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err("bad percent-encoding");
            }
            let high = hex(bytes[index + 1])?;
            let low = hex(bytes[index + 2])?;
            out.push((high << 4) | low);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).map_err(|_| "bad percent-encoding")
}

fn hex(byte: u8) -> Result<u8, &'static str> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err("bad percent-encoding"),
    }
}

pub fn json_error_message(error: &JsonError) -> String {
    format!("{} at {}", error.message, error.position)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_post_and_rejects_traversal() {
        let raw = b"POST /api/pipes/ingress.pass/push?cursor=1 HTTP/1.1\r\nHost: hull\r\nContent-Type: application/json\r\nContent-Length: 11\r\n\r\n{\"dish\":1}\n";
        let Parse::Done(request) = try_parse(raw) else {
            panic!("parse");
        };
        assert_eq!(request.method, "POST");
        assert_eq!(
            request.segments,
            vec!["api", "pipes", "ingress.pass", "push"]
        );
        assert_eq!(request.query_one("cursor"), Some("1"));
        assert_eq!(request.body.len(), 11);
        assert!(Request::from_parts("GET", "/assets/%2e%2e/secret", "", vec![], vec![]).is_err());
        assert!(matches!(
            try_parse(b"POST / HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\n\r\n"),
            Parse::Err(HttpError::Unsupported)
        ));
    }
}
