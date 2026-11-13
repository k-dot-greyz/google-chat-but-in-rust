//! The contract. Card kinds are a closed set, objects are closed, and the
//! document served at `/api/schema` is built from the same tables.

use crate::json::{Number, Value};

pub const SCHEMA_REV: i64 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Hero,
    Stat,
    List,
    Form,
    Note,
    Gauge,
    Media,
}

impl Kind {
    pub const ALL: [Kind; 7] = [
        Kind::Hero,
        Kind::Stat,
        Kind::List,
        Kind::Form,
        Kind::Note,
        Kind::Gauge,
        Kind::Media,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Hero => "hero",
            Kind::Stat => "stat",
            Kind::List => "list",
            Kind::Form => "form",
            Kind::Note => "note",
            Kind::Gauge => "gauge",
            Kind::Media => "media",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "hero" => Some(Kind::Hero),
            "stat" => Some(Kind::Stat),
            "list" => Some(Kind::List),
            "form" => Some(Kind::Form),
            "note" => Some(Kind::Note),
            "gauge" => Some(Kind::Gauge),
            "media" => Some(Kind::Media),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    pub path: String,
    pub detail: String,
}

impl Issue {
    fn new(path: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            detail: detail.into(),
        }
    }
}

pub struct RouteDef {
    pub method: &'static str,
    pub path: &'static str,
    pub summary: &'static str,
}

pub fn routes() -> &'static [RouteDef] {
    &[
        RouteDef {
            method: "GET",
            path: "/",
            summary: "The shell.",
        },
        RouteDef {
            method: "GET",
            path: "/api/health",
            summary: "Epoch, cursor, seal.",
        },
        RouteDef {
            method: "GET",
            path: "/api/schema",
            summary: "This contract.",
        },
        RouteDef {
            method: "GET",
            path: "/api/view",
            summary: "The island view model.",
        },
        RouteDef {
            method: "GET",
            path: "/api/cards",
            summary: "List cards.",
        },
        RouteDef {
            method: "POST",
            path: "/api/cards",
            summary: "Create a card in memory.",
        },
        RouteDef {
            method: "GET",
            path: "/api/cards/{id}",
            summary: "One card.",
        },
        RouteDef {
            method: "PATCH",
            path: "/api/cards/{id}",
            summary: "Sparse title or body fields.",
        },
        RouteDef {
            method: "DELETE",
            path: "/api/cards/{id}",
            summary: "Remove a card from memory.",
        },
        RouteDef {
            method: "GET",
            path: "/api/pipes",
            summary: "Pipes and directions.",
        },
        RouteDef {
            method: "POST",
            path: "/api/pipes/{id}/push",
            summary: "Run an ingress or egress pipe.",
        },
        RouteDef {
            method: "GET",
            path: "/api/airlock",
            summary: "Policy summary. Sockets stay shut.",
        },
        RouteDef {
            method: "GET",
            path: "/api/pipeline",
            summary: "Module graph.",
        },
        RouteDef {
            method: "GET",
            path: "/api/sync/handshake",
            summary: "sysex-delta/1 handshake.",
        },
        RouteDef {
            method: "GET",
            path: "/api/sync/delta",
            summary: "Patches since a cursor.",
        },
        RouteDef {
            method: "POST",
            path: "/api/sessions",
            summary: "Mint a local session for one channel. No remote provider.",
        },
        RouteDef {
            method: "DELETE",
            path: "/api/sessions",
            summary: "Revoke the session named by x-session.",
        },
        RouteDef {
            method: "QUERY",
            path: "/api/channels/{channel}",
            summary: "Read card fields for the session's channel. The filter is the body.",
        },
        RouteDef {
            method: "GET",
            path: "/api/providers",
            summary: "Auth providers. local does not send.",
        },
    ]
}

pub fn is_id(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && value.len() <= 49
        && chars.all(|ch| {
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '_' | '-')
        })
}

pub fn is_field(value: &str) -> bool {
    is_id(value) && !value.contains('.') && !value.contains('-')
}

pub fn check_plain(value: &str, min: usize, max: usize) -> Result<(), String> {
    let count = value.chars().count();
    if count < min || count > max {
        return Err(format!("length {count} is outside {min}..={max}"));
    }
    if value.chars().any(is_forbidden_char) {
        return Err("control or bidi character".into());
    }
    Ok(())
}

fn is_forbidden_char(ch: char) -> bool {
    (ch as u32) < 0x20
        || ch == '\u{007f}'
        || ('\u{202a}'..='\u{202e}').contains(&ch)
        || ('\u{2066}'..='\u{2069}').contains(&ch)
}

pub fn validate_card(value: &Value) -> Result<(), Vec<Issue>> {
    let pairs = object(value, "", &["id", "kind", "rev", "title", "body"])?;
    let mut issues = Vec::new();
    let id = match req_str(pairs, "", "id") {
        Ok(id) if is_id(id) => Some(id),
        Ok(_) => {
            issues.push(Issue::new("/id", "id must be a short lowercase token"));
            None
        }
        Err(issue) => {
            issues.push(issue);
            None
        }
    };
    let kind = match req_str(pairs, "", "kind") {
        Ok(kind) => match Kind::parse(kind) {
            Some(kind) => Some(kind),
            None => {
                issues.push(Issue::new("/kind", "unknown card kind"));
                None
            }
        },
        Err(issue) => {
            issues.push(issue);
            None
        }
    };
    match req_i64(pairs, "", "rev") {
        Ok(rev) if (0..=1_000_000_000).contains(&rev) => {}
        Ok(_) => issues.push(Issue::new("/rev", "rev is out of range")),
        Err(issue) => issues.push(issue),
    }
    match req_str(pairs, "", "title") {
        Ok(title) => {
            if let Err(detail) = check_plain(title, 1, 80) {
                issues.push(Issue::new("/title", detail));
            }
        }
        Err(issue) => issues.push(issue),
    }
    match (kind, find(pairs, "body")) {
        (Some(kind), Some(body)) => {
            if let Err(mut body_issues) = validate_body(kind, body) {
                issues.append(&mut body_issues);
            }
        }
        (_, None) => issues.push(Issue::new("/body", "missing body")),
        _ => {}
    }
    let _ = id;
    finish(issues)
}

fn validate_body(kind: Kind, body: &Value) -> Result<(), Vec<Issue>> {
    match kind {
        Kind::Hero => {
            let pairs = object(body, "/body", &["kicker", "lede"])?;
            let mut issues = Vec::new();
            match opt_str(pairs, "/body", "kicker") {
                Ok(Some(text)) => {
                    if let Err(detail) = check_plain(text, 0, 40) {
                        issues.push(Issue::new("/body/kicker", detail));
                    }
                }
                Ok(None) => {}
                Err(issue) => issues.push(issue),
            }
            push_text(&mut issues, pairs, "/body", "lede", 1, 280, true);
            finish(issues)
        }
        Kind::Stat => {
            let pairs = object(body, "/body", &["label", "value", "unit", "hint"])?;
            let mut issues = Vec::new();
            push_text(&mut issues, pairs, "/body", "label", 1, 40, true);
            push_i64(&mut issues, pairs, "/body", "value", -1_000_000, 1_000_000);
            push_text(&mut issues, pairs, "/body", "unit", 0, 20, true);
            push_text(&mut issues, pairs, "/body", "hint", 0, 140, true);
            finish(issues)
        }
        Kind::List => {
            let pairs = object(body, "/body", &["items"])?;
            let mut issues = Vec::new();
            match req_array(pairs, "/body", "items") {
                Ok(items) if items.len() > 24 => {
                    issues.push(Issue::new("/body/items", "a list holds at most 24"))
                }
                Ok(items) => {
                    for (index, item) in items.iter().enumerate() {
                        if let Err(mut item_issues) = validate_item(item, index) {
                            issues.append(&mut item_issues);
                        }
                    }
                }
                Err(issue) => issues.push(issue),
            }
            finish(issues)
        }
        Kind::Form => validate_form(body),
        Kind::Note => {
            let pairs = object(body, "/body", &["text", "tone"])?;
            let mut issues = Vec::new();
            push_text(&mut issues, pairs, "/body", "text", 1, 500, true);
            push_tone(&mut issues, pairs, "/body/tone");
            finish(issues)
        }
        Kind::Gauge => {
            let pairs = object(body, "/body", &["label", "value", "caption"])?;
            let mut issues = Vec::new();
            push_text(&mut issues, pairs, "/body", "label", 1, 40, true);
            push_i64(&mut issues, pairs, "/body", "value", 0, 100);
            push_text(&mut issues, pairs, "/body", "caption", 0, 180, true);
            finish(issues)
        }
        Kind::Media => validate_media(body),
    }
}

fn validate_media(body: &Value) -> Result<(), Vec<Issue>> {
    let pairs = object(body, "/body", &["medium", "src", "caption", "fallback"])?;
    let mut issues = Vec::new();
    match req_str(pairs, "/body", "medium") {
        Ok("video" | "audio" | "frame") => {}
        Ok(_) => issues.push(Issue::new(
            "/body/medium",
            "medium must be video, audio, or frame",
        )),
        Err(issue) => issues.push(issue),
    }
    match opt_str(pairs, "/body", "src") {
        Ok(Some(src)) if safe_media_src(src) => {}
        Ok(Some(_)) => issues.push(Issue::new(
            "/body/src",
            "src must be empty or a local /assets file",
        )),
        Ok(None) => {}
        Err(issue) => issues.push(issue),
    }
    push_text(&mut issues, pairs, "/body", "caption", 0, 160, true);
    push_text(&mut issues, pairs, "/body", "fallback", 1, 180, true);
    finish(issues)
}

pub fn safe_media_src(src: &str) -> bool {
    if src.is_empty() {
        return true;
    }
    let Some(name) = src.strip_prefix("/assets/") else {
        return false;
    };
    !name.is_empty()
        && name.len() <= 80
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains(':')
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
}

fn validate_item(item: &Value, index: usize) -> Result<(), Vec<Issue>> {
    let path = format!("/body/items/{index}");
    let pairs = object(item, &path, &["title", "detail", "tone"])?;
    let mut issues = Vec::new();
    push_text(&mut issues, pairs, &path, "title", 1, 48, true);
    push_text(&mut issues, pairs, &path, "detail", 0, 180, true);
    match req_str(pairs, &path, "tone") {
        Ok(tone) if is_tone(tone) => {}
        Ok(_) => issues.push(Issue::new(format!("{path}/tone"), "unknown tone")),
        Err(issue) => issues.push(issue),
    }
    finish(issues)
}

fn validate_form(body: &Value) -> Result<(), Vec<Issue>> {
    let pairs = object(body, "/body", &["pipe", "submit", "fields"])?;
    let mut issues = Vec::new();
    match req_str(pairs, "/body", "pipe") {
        Ok(pipe) if is_id(pipe) => {}
        Ok(_) => issues.push(Issue::new("/body/pipe", "pipe id is not valid")),
        Err(issue) => issues.push(issue),
    }
    push_text(&mut issues, pairs, "/body", "submit", 1, 40, true);
    match req_array(pairs, "/body", "fields") {
        Ok(fields) if fields.is_empty() || fields.len() > 8 => {
            issues.push(Issue::new("/body/fields", "a form has 1 to 8 fields"));
        }
        Ok(fields) => {
            for (index, field) in fields.iter().enumerate() {
                if let Err(mut field_issues) = validate_field(field, index) {
                    issues.append(&mut field_issues);
                }
            }
        }
        Err(issue) => issues.push(issue),
    }
    finish(issues)
}

fn validate_field(field: &Value, index: usize) -> Result<(), Vec<Issue>> {
    let path = format!("/body/fields/{index}");
    let pairs = object(field, &path, &["name", "label", "type", "required", "max"])?;
    let mut issues = Vec::new();
    match req_str(pairs, &path, "name") {
        Ok(name) if is_field(name) => {}
        Ok(_) => issues.push(Issue::new(
            format!("{path}/name"),
            "field name is not valid",
        )),
        Err(issue) => issues.push(issue),
    }
    push_text(&mut issues, pairs, &path, "label", 1, 40, true);
    match req_str(pairs, &path, "type") {
        Ok("text") => {}
        Ok(_) => issues.push(Issue::new(
            format!("{path}/type"),
            "only text fields are accepted",
        )),
        Err(issue) => issues.push(issue),
    }
    match find(pairs, "required") {
        Some(Value::Bool(_)) => {}
        Some(_) => issues.push(Issue::new(
            format!("{path}/required"),
            "required must be a boolean",
        )),
        None => issues.push(Issue::new(format!("{path}/required"), "missing required")),
    }
    push_i64(&mut issues, pairs, &path, "max", 1, 80);
    finish(issues)
}

pub fn is_tone(value: &str) -> bool {
    matches!(value, "ink" | "copper" | "sea" | "deny")
}

pub fn document() -> Value {
    let kinds = Value::Array(
        Kind::ALL
            .iter()
            .map(|kind| Value::String(kind.as_str().into()))
            .collect(),
    );
    let resources = Value::Array(
        routes()
            .iter()
            .map(|route| {
                object_owned(vec![
                    ("method", Value::String(route.method.into())),
                    ("path", Value::String(route.path.into())),
                    ("summary", Value::String(route.summary.into())),
                ])
            })
            .collect(),
    );
    object_owned(vec![
        ("name", Value::String("hull".into())),
        ("revision", Value::Number(Number::Int(SCHEMA_REV))),
        ("protocol", Value::String("sysex-delta/1".into())),
        ("kinds", kinds),
        (
            "probes",
            Value::Array(vec![
                Value::String("unlisted".into()),
                Value::String("metadata".into()),
                Value::String("plain-http".into()),
                Value::String("allowlisted".into()),
            ]),
        ),
        ("resources", resources),
        (
            "rules",
            Value::Array(vec![
                Value::String("Unknown object fields are refused.".into()),
                Value::String(
                    "Pipe push bodies are derived from steps. Callers do not name URLs.".into(),
                ),
                Value::String(
                    "Hull class beats the allowlist. The dry dock does not open a socket.".into(),
                ),
                Value::String("Integers are bare digits. Exponents are not integers.".into()),
            ]),
        ),
    ])
}

pub fn issues_text(issues: &[Issue]) -> String {
    issues
        .iter()
        .map(|issue| format!("{}: {}", issue.path, issue.detail))
        .collect::<Vec<_>>()
        .join("; ")
}

fn object<'a>(
    value: &'a Value,
    path: &str,
    allowed: &[&str],
) -> Result<&'a [(String, Value)], Vec<Issue>> {
    let Value::Object(pairs) = value else {
        return Err(vec![Issue::new(path, "expected an object")]);
    };
    let mut issues = Vec::new();
    for (key, _) in pairs {
        if !allowed.contains(&key.as_str()) {
            let full = join(path, key);
            issues.push(Issue::new(full, "unknown field"));
        }
    }
    if issues.is_empty() {
        Ok(pairs)
    } else {
        Err(issues)
    }
}

fn find<'a>(pairs: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
    pairs
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value)
}

fn req_str<'a>(pairs: &'a [(String, Value)], path: &str, key: &str) -> Result<&'a str, Issue> {
    match find(pairs, key) {
        Some(Value::String(text)) => Ok(text),
        Some(_) => Err(Issue::new(join(path, key), "expected a string")),
        None => Err(Issue::new(join(path, key), "missing")),
    }
}

fn opt_str<'a>(
    pairs: &'a [(String, Value)],
    path: &str,
    key: &str,
) -> Result<Option<&'a str>, Issue> {
    match find(pairs, key) {
        Some(Value::String(text)) => Ok(Some(text)),
        Some(_) => Err(Issue::new(join(path, key), "expected a string")),
        None => Ok(None),
    }
}

fn req_i64(pairs: &[(String, Value)], path: &str, key: &str) -> Result<i64, Issue> {
    match find(pairs, key) {
        Some(Value::Number(Number::Int(number))) => Ok(*number),
        Some(_) => Err(Issue::new(join(path, key), "expected an integer")),
        None => Err(Issue::new(join(path, key), "missing")),
    }
}

fn req_array<'a>(
    pairs: &'a [(String, Value)],
    path: &str,
    key: &str,
) -> Result<&'a [Value], Issue> {
    match find(pairs, key) {
        Some(Value::Array(items)) => Ok(items),
        Some(_) => Err(Issue::new(join(path, key), "expected an array")),
        None => Err(Issue::new(join(path, key), "missing")),
    }
}

fn push_text(
    issues: &mut Vec<Issue>,
    pairs: &[(String, Value)],
    path: &str,
    key: &str,
    min: usize,
    max: usize,
    required: bool,
) {
    match if required {
        req_str(pairs, path, key).map(Some)
    } else {
        opt_str(pairs, path, key)
    } {
        Ok(Some(text)) => {
            if let Err(detail) = check_plain(text, min, max) {
                issues.push(Issue::new(join(path, key), detail));
            }
        }
        Ok(None) => {}
        Err(issue) => issues.push(issue),
    }
}

fn push_i64(
    issues: &mut Vec<Issue>,
    pairs: &[(String, Value)],
    path: &str,
    key: &str,
    min: i64,
    max: i64,
) {
    match req_i64(pairs, path, key) {
        Ok(number) if (min..=max).contains(&number) => {}
        Ok(_) => issues.push(Issue::new(
            join(path, key),
            format!("outside {min}..={max}"),
        )),
        Err(issue) => issues.push(issue),
    }
}

fn push_tone(issues: &mut Vec<Issue>, pairs: &[(String, Value)], path: &str) {
    match req_str(pairs, "/body", "tone") {
        Ok(tone) if is_tone(tone) => {}
        Ok(_) => issues.push(Issue::new(path, "unknown tone")),
        Err(issue) => issues.push(issue),
    }
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        format!("/{key}")
    } else {
        format!("{path}/{key}")
    }
}

fn finish(issues: Vec<Issue>) -> Result<(), Vec<Issue>> {
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

fn object_owned(pairs: Vec<(&str, Value)>) -> Value {
    Value::Object(
        pairs
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse;

    #[test]
    fn unknown_fields_and_kinds_fail() {
        let card = parse(r#"{"id":"card.x","kind":"hero","rev":1,"title":"Hi","body":{"lede":"There.","extra":1}}"#).unwrap();
        let err = validate_card(&card).unwrap_err();
        assert!(err.iter().any(|issue| issue.detail.contains("unknown")));
        let bad =
            parse(r#"{"id":"card.x","kind":"sparkle","rev":1,"title":"Hi","body":{}}"#).unwrap();
        assert!(
            validate_card(&bad)
                .unwrap_err()
                .iter()
                .any(|issue| issue.path == "/kind")
        );
    }

    #[test]
    fn a_remote_media_src_is_refused() {
        let card = parse(r#"{"id":"card.bay","kind":"media","rev":1,"title":"Bay","body":{"medium":"video","src":"https://evil.example/reel.mp4","caption":"no","fallback":"dark"}}"#).unwrap();
        let err = validate_card(&card).unwrap_err();
        assert!(err.iter().any(|issue| issue.path == "/body/src"));
    }

    #[test]
    fn document_lists_the_routes_and_kinds() {
        let doc = document();
        let text = doc.to_json();
        assert!(text.contains("/api/sync/delta"));
        assert!(text.contains("gauge"));
        assert_eq!(
            doc.get("revision").and_then(Value::as_i64),
            Some(SCHEMA_REV)
        );
    }
}
