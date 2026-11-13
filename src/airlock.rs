//! Default-deny egress. A `Permit` cannot be built outside this module.
//! `dry_dock` is the only consumer, and it never opens a socket.

use std::net::Ipv4Addr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Probe {
    Unlisted,
    Metadata,
    PlainHttp,
    Allowlisted,
}

impl Probe {
    pub fn as_str(self) -> &'static str {
        match self {
            Probe::Unlisted => "unlisted",
            Probe::Metadata => "metadata",
            Probe::PlainHttp => "plain-http",
            Probe::Allowlisted => "allowlisted",
        }
    }

    pub fn destination(self) -> &'static str {
        match self {
            Probe::Unlisted => "https://example.com:443/hook",
            Probe::Metadata => "http://169.254.169.254/latest/meta-data",
            Probe::PlainHttp => "http://hooks.hull.invalid:80/plate",
            Probe::Allowlisted => "https://hooks.hull.invalid:443/plate",
        }
    }
}

pub fn parse_probe(value: &str) -> Option<Probe> {
    match value {
        "unlisted" => Some(Probe::Unlisted),
        "metadata" => Some(Probe::Metadata),
        "plain-http" => Some(Probe::PlainHttp),
        "allowlisted" => Some(Probe::Allowlisted),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenyCode {
    BadDestination,
    HullClass,
    Scheme,
    BodyTooLarge,
    ContentType,
    NotAllowlisted,
    Method,
    Pipe,
}

impl DenyCode {
    pub fn as_str(self) -> &'static str {
        match self {
            DenyCode::BadDestination => "bad-destination",
            DenyCode::HullClass => "hull",
            DenyCode::Scheme => "scheme",
            DenyCode::BodyTooLarge => "body",
            DenyCode::ContentType => "content-type",
            DenyCode::NotAllowlisted => "allowlist",
            DenyCode::Method => "method",
            DenyCode::Pipe => "pipe",
        }
    }

    pub fn notice(self) -> &'static str {
        match self {
            DenyCode::BadDestination => "Denied. That destination does not parse.",
            DenyCode::HullClass => "Denied. That address is the hull talking to itself.",
            DenyCode::Scheme => "Denied. The airlock takes https only.",
            DenyCode::BodyTooLarge => "Denied. The outbound plate is too big.",
            DenyCode::ContentType => "Denied. The outbound type is not on the list.",
            DenyCode::NotAllowlisted => "Denied. That origin is not on the allowlist.",
            DenyCode::Method => "Denied. That method is not on the allow entry.",
            DenyCode::Pipe => "Denied. This pipe is not on the allow entry.",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin {
    pub scheme: String,
    pub host: String,
    pub port: u16,
}

impl Origin {
    pub fn key(&self) -> String {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        format!("{}://{}:{}", self.scheme, host, self.port)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Allow {
    pub origin: Origin,
    pub methods: Vec<String>,
    pub pipes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    pub https_only: bool,
    pub max_body: usize,
    pub content_types: Vec<String>,
    pub allow: Vec<Allow>,
}

pub struct EgressRequest<'a> {
    pub pipe: &'a str,
    pub method: &'a str,
    pub destination: &'a str,
    pub content_type: &'a str,
    pub body: &'a [u8],
}

#[derive(Debug)]
pub struct Denial {
    pub code: DenyCode,
    pub detail: String,
}

#[derive(Debug)]
pub struct Permit {
    origin: String,
    method: String,
    pipe: String,
}

pub struct Receipt {
    origin: String,
    method: String,
    pipe: String,
    bytes: usize,
    sent: bool,
}

impl Receipt {
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn method(&self) -> &str {
        &self.method
    }
    pub fn pipe(&self) -> &str {
        &self.pipe
    }
    pub fn bytes(&self) -> usize {
        self.bytes
    }
    pub fn sent(&self) -> bool {
        self.sent
    }
}

pub fn parse_origin(url: &str) -> Result<Origin, &'static str> {
    if url.is_empty()
        || url.len() > 300
        || url
            .chars()
            .any(|ch| ch.is_whitespace() || ch == '\\' || ch == '@' || ch == '#')
    {
        return Err("destination rejected");
    }
    let (scheme, rest) = url.split_once("://").ok_or("destination rejected")?;
    if scheme != "https" && scheme != "http" {
        return Err("destination rejected");
    }
    let (authority, _path) = match rest.split_once('/') {
        Some((authority, path)) => {
            if path.contains("://") {
                return Err("destination rejected");
            }
            (authority, path)
        }
        None => (rest, ""),
    };
    if authority.is_empty() || authority.contains('@') {
        return Err("destination rejected");
    }
    let (host, port) = split_host_port(scheme, authority)?;
    if host.is_empty() || host.len() > 253 {
        return Err("destination rejected");
    }
    Ok(Origin {
        scheme: scheme.to_string(),
        host: host.to_ascii_lowercase(),
        port,
    })
}

fn split_host_port(scheme: &str, authority: &str) -> Result<(String, u16), &'static str> {
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, after) = rest.split_once(']').ok_or("destination rejected")?;
        if host.is_empty() || host.contains('%') {
            return Err("destination rejected");
        }
        let port = if let Some(port) = after.strip_prefix(':') {
            parse_port(port)?
        } else if after.is_empty() {
            default_port(scheme)?
        } else {
            return Err("destination rejected");
        };
        return Ok((host.to_string(), port));
    }
    if authority.contains(':') {
        let (host, port) = authority.rsplit_once(':').ok_or("destination rejected")?;
        if host.is_empty() {
            return Err("destination rejected");
        }
        return Ok((host.to_string(), parse_port(port)?));
    }
    Ok((authority.to_string(), default_port(scheme)?))
}

fn default_port(scheme: &str) -> Result<u16, &'static str> {
    match scheme {
        "https" => Ok(443),
        "http" => Ok(80),
        _ => Err("destination rejected"),
    }
}

fn parse_port(text: &str) -> Result<u16, &'static str> {
    if text.is_empty() || text.len() > 5 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err("destination rejected");
    }
    text.parse::<u16>().map_err(|_| "destination rejected")
}

pub fn hull_block_reason(host: &str) -> Option<&'static str> {
    let host = host.trim_matches(['[', ']']);
    let lower = host.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "localhost"
            | "metadata"
            | "metadata.google.internal"
            | "internal"
            | "intranet"
            | "localdomain"
    ) {
        return Some("name");
    }
    for suffix in [".local", ".localhost", ".internal", ".intranet"] {
        if lower.ends_with(suffix) {
            return Some("name");
        }
    }
    if lower.bytes().all(|b| b.is_ascii_digit()) && !lower.is_empty() {
        return Some("numeric-host");
    }
    if lower.contains(':') {
        return match parse_ipv6(&lower) {
            Some(addr) => ipv6_reason(&addr),
            None => Some("bad-ip"),
        };
    }
    if looks_like_v4(&lower) {
        return match parse_v4(&lower) {
            Some(addr) => ipv4_reason(addr),
            None => Some("bad-ip"),
        };
    }
    if !valid_hostname(&lower) {
        return Some("bad-name");
    }
    None
}

fn looks_like_v4(host: &str) -> bool {
    host.split('.').count() == 4
        && host
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
}

fn parse_v4(host: &str) -> Option<Ipv4Addr> {
    let mut octets = [0u8; 4];
    for (index, part) in host.split('.').enumerate() {
        if part.is_empty() || part.len() > 3 {
            return None;
        }
        if part.len() > 1 && part.starts_with('0') {
            return None;
        }
        octets[index] = part.parse().ok()?;
    }
    Some(Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3]))
}

fn ipv4_reason(addr: Ipv4Addr) -> Option<&'static str> {
    let [a, b, _, _] = addr.octets();
    if a == 0
        || a == 10
        || a == 127
        || (a == 169 && b == 254)
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && b == 168)
        || (a == 100 && (64..=127).contains(&b))
        || a >= 224
    {
        Some("range")
    } else {
        None
    }
}

fn parse_ipv6(host: &str) -> Option<[u8; 16]> {
    let (head, tail) = match host.split_once("::") {
        Some((head, tail)) => {
            if tail.contains("::") {
                return None;
            }
            (head, tail)
        }
        None => (host, ""),
    };
    let mut bytes = [0u8; 16];
    let head_groups = parse_groups(head)?;
    let mut tail_groups = parse_groups(tail)?;
    if host.contains("::") {
        let used = head_groups.len() + tail_groups.len();
        if used > 8 {
            return None;
        }
        let missing = 8 - used;
        let mut groups = head_groups;
        groups.extend(std::iter::repeat_n(0u16, missing));
        groups.append(&mut tail_groups);
        write_groups(&groups, &mut bytes)?;
    } else if head_groups.len() == 8 {
        write_groups(&head_groups, &mut bytes)?;
    } else {
        return None;
    }
    Some(bytes)
}

fn parse_groups(text: &str) -> Option<Vec<u16>> {
    if text.is_empty() {
        return Some(Vec::new());
    }
    let mut groups = Vec::new();
    let parts: Vec<&str> = text.split(':').collect();
    for (index, part) in parts.iter().enumerate() {
        if part.contains('.') {
            if index != parts.len() - 1 {
                return None;
            }
            let addr = parse_v4(part)?;
            let octets = addr.octets();
            groups.push(u16::from_be_bytes([octets[0], octets[1]]));
            groups.push(u16::from_be_bytes([octets[2], octets[3]]));
        } else {
            if part.is_empty() || part.len() > 4 {
                return None;
            }
            groups.push(u16::from_str_radix(part, 16).ok()?);
        }
    }
    Some(groups)
}

fn write_groups(groups: &[u16], bytes: &mut [u8; 16]) -> Option<()> {
    if groups.len() != 8 {
        return None;
    }
    for (index, group) in groups.iter().enumerate() {
        let pair = group.to_be_bytes();
        bytes[index * 2] = pair[0];
        bytes[index * 2 + 1] = pair[1];
    }
    Some(())
}

fn ipv6_reason(addr: &[u8; 16]) -> Option<&'static str> {
    if addr.iter().all(|byte| *byte == 0) {
        return Some("unspecified");
    }
    if addr[..10].iter().all(|byte| *byte == 0) && addr[10] == 0xff && addr[11] == 0xff {
        return ipv4_reason(Ipv4Addr::new(addr[12], addr[13], addr[14], addr[15]));
    }
    if *addr == [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1] {
        return Some("loopback");
    }
    if addr[0] & 0xfe == 0xfc {
        return Some("ula");
    }
    if addr[0] == 0xfe && (addr[1] & 0xc0) == 0x80 {
        return Some("link-local");
    }
    if addr[0] == 0xff {
        return Some("multicast");
    }
    None
}

fn valid_hostname(host: &str) -> bool {
    if host.is_empty() || host.len() > 253 || host.starts_with('.') || host.ends_with('.') {
        return false;
    }
    host.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    })
}

pub fn authorize(policy: &Policy, request: EgressRequest<'_>) -> Result<Permit, Denial> {
    let origin = match parse_origin(request.destination) {
        Ok(origin) => origin,
        Err(_) => {
            return Err(deny(DenyCode::BadDestination));
        }
    };
    if hull_block_reason(&origin.host).is_some() {
        return Err(deny(DenyCode::HullClass));
    }
    if policy.https_only && origin.scheme != "https" {
        return Err(deny(DenyCode::Scheme));
    }
    if request.body.len() > policy.max_body {
        return Err(deny(DenyCode::BodyTooLarge));
    }
    let content_type = request
        .content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if !policy
        .content_types
        .iter()
        .any(|allowed| allowed == &content_type)
    {
        return Err(deny(DenyCode::ContentType));
    }
    let method = request.method.to_ascii_uppercase();
    let Some(entry) = policy.allow.iter().find(|entry| entry.origin == origin) else {
        return Err(deny(DenyCode::NotAllowlisted));
    };
    if !entry.methods.iter().any(|item| item == &method) {
        return Err(deny(DenyCode::Method));
    }
    if !entry.pipes.iter().any(|item| item == request.pipe) {
        return Err(deny(DenyCode::Pipe));
    }
    Ok(Permit {
        origin: origin.key(),
        method,
        pipe: request.pipe.to_string(),
    })
}

fn deny(code: DenyCode) -> Denial {
    Denial {
        code,
        detail: code.notice().to_string(),
    }
}

pub fn dry_dock(permit: Permit, body: &[u8]) -> Receipt {
    Receipt {
        origin: permit.origin,
        method: permit.method,
        pipe: permit.pipe,
        bytes: body.len(),
        sent: false,
    }
}

pub fn permit_notice() -> &'static str {
    "Permit issued. The dry dock kept the plate. Nothing was sent."
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy_with(extra: Vec<Allow>) -> Policy {
        let mut allow = vec![Allow {
            origin: parse_origin("https://hooks.hull.invalid:443").unwrap(),
            methods: vec!["POST".into()],
            pipes: vec!["egress.probe".into()],
        }];
        allow.extend(extra);
        Policy {
            https_only: true,
            max_body: 4096,
            content_types: vec!["application/json".into()],
            allow,
        }
    }

    fn ask<'a>(
        policy: &Policy,
        destination: &'a str,
        pipe: &'a str,
        body: &'a [u8],
    ) -> Result<Permit, Denial> {
        authorize(
            policy,
            EgressRequest {
                pipe,
                method: "POST",
                destination,
                content_type: "application/json",
                body,
            },
        )
    }

    #[test]
    fn probe_matrix_and_dry_dock_never_sends() {
        let policy = policy_with(vec![]);
        let denied = ask(
            &policy,
            Probe::Unlisted.destination(),
            "egress.probe",
            b"{}",
        )
        .unwrap_err();
        assert_eq!(denied.code, DenyCode::NotAllowlisted);
        let meta = ask(
            &policy,
            Probe::Metadata.destination(),
            "egress.probe",
            b"{}",
        )
        .unwrap_err();
        assert_eq!(meta.code, DenyCode::HullClass);
        let plain = ask(
            &policy,
            Probe::PlainHttp.destination(),
            "egress.probe",
            b"{}",
        )
        .unwrap_err();
        assert_eq!(plain.code, DenyCode::Scheme);
        let permit = ask(
            &policy,
            Probe::Allowlisted.destination(),
            "egress.probe",
            b"{\"ok\":true}",
        )
        .unwrap();
        let receipt = dry_dock(permit, b"{\"ok\":true}");
        assert!(!receipt.sent());
        assert_eq!(receipt.bytes(), 11);
        assert_eq!(receipt.origin(), "https://hooks.hull.invalid:443");
    }

    #[test]
    fn allowlist_cannot_override_hull_class() {
        let sneaky = [
            "http://169.254.169.254:80",
            "https://127.0.0.1:443",
            "http://[::1]:80",
            "http://10.0.0.8:80",
            "http://localhost:80",
            "http://metadata.google.internal:80",
        ];
        let extra = sneaky
            .iter()
            .map(|origin| Allow {
                origin: parse_origin(origin).unwrap(),
                methods: vec!["POST".into()],
                pipes: vec!["egress.probe".into()],
            })
            .collect();
        let policy = policy_with(extra);
        for origin in sneaky {
            let denial = ask(&policy, origin, "egress.probe", b"{}").unwrap_err();
            assert_eq!(denial.code, DenyCode::HullClass, "{origin}");
        }
        let mapped = ask(
            &policy,
            "http://[::ffff:169.254.169.254]:80/",
            "egress.probe",
            b"{}",
        )
        .unwrap_err();
        assert_eq!(mapped.code, DenyCode::HullClass);
    }

    #[test]
    fn method_pipe_size_and_type_are_separate_denials() {
        let policy = policy_with(vec![]);
        let method = authorize(
            &policy,
            EgressRequest {
                pipe: "egress.probe",
                method: "PUT",
                destination: Probe::Allowlisted.destination(),
                content_type: "application/json",
                body: b"{}",
            },
        )
        .unwrap_err();
        assert_eq!(method.code, DenyCode::Method);
        let pipe = ask(
            &policy,
            Probe::Allowlisted.destination(),
            "egress.other",
            b"{}",
        )
        .unwrap_err();
        assert_eq!(pipe.code, DenyCode::Pipe);
        let huge = ask(
            &policy,
            Probe::Allowlisted.destination(),
            "egress.probe",
            &vec![0; 5000],
        )
        .unwrap_err();
        assert_eq!(huge.code, DenyCode::BodyTooLarge);
        let typed = authorize(
            &policy,
            EgressRequest {
                pipe: "egress.probe",
                method: "POST",
                destination: Probe::Allowlisted.destination(),
                content_type: "text/plain",
                body: b"{}",
            },
        )
        .unwrap_err();
        assert_eq!(typed.code, DenyCode::ContentType);
    }
}
