//! Load cards, pipes, and policy. Files are data. The step vocabulary is not.

use std::fs;
use std::path::{Path, PathBuf};

use crate::airlock::{Allow, Policy, parse_origin};
use crate::embed::{EMBEDDED_CARDS, EMBEDDED_PIPES, EMBEDDED_POLICY};
use crate::json::{self, Value};
use crate::schema::{self, Kind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Ingress,
    Internal,
    Egress,
}

impl Direction {
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::Ingress => "ingress",
            Direction::Internal => "internal",
            Direction::Egress => "egress",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "ingress" => Some(Direction::Ingress),
            "internal" => Some(Direction::Internal),
            "egress" => Some(Direction::Egress),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Step {
    AppendList {
        card: String,
        title: String,
        detail: String,
    },
    IncStat {
        card: String,
        by: i64,
    },
    SetGauge {
        card: String,
        value: u8,
        caption: String,
    },
    Airlock {
        log: String,
        gauge: String,
    },
}

impl Step {
    pub fn summary(&self) -> String {
        match self {
            Step::AppendList {
                card,
                title,
                detail,
            } => format!("{} {card} {title}/{detail}", tight("append-list")),
            Step::IncStat { card, by } => format!("{} {card} +{by}", tight("inc-stat")),
            Step::SetGauge { card, value, .. } => format!("{} {card} {value}", tight("set-gauge")),
            Step::Airlock { log, gauge } => format!("airlock {log} {gauge}"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Pipe {
    pub id: String,
    pub direction: Direction,
    pub binds: Vec<String>,
    pub then: Vec<String>,
    pub steps: Vec<Step>,
}

fn tight(op: &str) -> String {
    op.replace('-', "\u{2011}")
}

impl Pipe {
    pub fn summary(&self) -> String {
        let mut parts: Vec<String> = self.steps.iter().map(Step::summary).collect();
        if !self.then.is_empty() {
            parts.push(format!("then {}", self.then.join(" ")));
        }
        parts.join(" · ")
    }
}

#[derive(Clone, Debug)]
pub struct Bundle {
    pub epoch: String,
    pub cards: Vec<Value>,
    pub pipes: Vec<Pipe>,
    pub policy: Policy,
}

pub fn from_embedded() -> Result<Bundle, String> {
    let mut hash = FNV_OFFSET;
    let mut cards = Vec::new();
    for (name, text) in EMBEDDED_CARDS {
        hash = mix(hash, name.as_bytes());
        hash = mix(hash, text.as_bytes());
        cards.push(parse_card(name, text)?);
    }
    let mut pipes = Vec::new();
    for (name, text) in EMBEDDED_PIPES {
        hash = mix(hash, name.as_bytes());
        hash = mix(hash, text.as_bytes());
        pipes.push(parse_pipe(name, text)?);
    }
    hash = mix(hash, EMBEDDED_POLICY.as_bytes());
    let policy = parse_policy("policy.json", EMBEDDED_POLICY)?;
    link(&cards, &pipes)?;
    check_policy_pipes(&policy, &pipes)?;
    Ok(Bundle {
        epoch: format!("{hash:016x}"),
        cards,
        pipes,
        policy,
    })
}

pub fn from_dir(dir: &Path) -> Result<Bundle, String> {
    let root = dir
        .canonicalize()
        .map_err(|_| format!("{} is not a modules directory", dir.display()))?;
    let mut hash = FNV_OFFSET;
    let mut cards = Vec::new();
    for (name, text) in read_tree(&root, "cards")? {
        hash = mix(hash, name.as_bytes());
        hash = mix(hash, text.as_bytes());
        cards.push(parse_card(&name, &text)?);
    }
    let mut pipes = Vec::new();
    for (name, text) in read_tree(&root, "pipes")? {
        hash = mix(hash, name.as_bytes());
        hash = mix(hash, text.as_bytes());
        pipes.push(parse_pipe(&name, &text)?);
    }
    let policy_path = rooted(&root, &root.join("policy.json"))?;
    let policy_text = read_capped(&policy_path)?;
    hash = mix(hash, policy_text.as_bytes());
    let policy = parse_policy("policy.json", &policy_text)?;
    link(&cards, &pipes)?;
    check_policy_pipes(&policy, &pipes)?;
    Ok(Bundle {
        epoch: format!("{hash:016x}"),
        cards,
        pipes,
        policy,
    })
}

const FNV_OFFSET: u64 = 0xcbf29ce484222325;

fn mix(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn parse_card(name: &str, text: &str) -> Result<Value, String> {
    let value = json::parse(text)
        .map_err(|error| format!("{name}: {} at {}", error.message, error.position))?;
    schema::validate_card(&value)
        .map_err(|issues| format!("{name}: {}", schema::issues_text(&issues)))?;
    let id = value.get("id").and_then(Value::as_str).unwrap_or("");
    let stem = name.trim_end_matches(".json");
    if stem != id {
        return Err(format!("{name}: file name must match id `{id}`"));
    }
    Ok(value)
}

fn parse_pipe(name: &str, text: &str) -> Result<Pipe, String> {
    let value = json::parse(text)
        .map_err(|error| format!("{name}: {} at {}", error.message, error.position))?;
    let Value::Object(pairs) = &value else {
        return Err(format!("{name}: expected an object"));
    };
    reject_unknown(name, pairs, &["id", "direction", "binds", "then", "steps"])?;
    let id = req_str(name, pairs, "id")?;
    if !schema::is_id(id) {
        return Err(format!("{name}: bad id"));
    }
    if name.trim_end_matches(".json") != id {
        return Err(format!("{name}: file name must match id `{id}`"));
    }
    let direction = Direction::parse(req_str(name, pairs, "direction")?)
        .ok_or_else(|| format!("{name}: bad direction"))?;
    let binds = string_list(name, pairs, "binds")?;
    let then = string_list(name, pairs, "then")?;
    for item in binds.iter().chain(then.iter()) {
        if !schema::is_id(item) {
            return Err(format!("{name}: `{item}` is not an id"));
        }
    }
    let steps_value = req(name, pairs, "steps")?;
    let Value::Array(steps_raw) = steps_value else {
        return Err(format!("{name}: steps must be an array"));
    };
    if steps_raw.is_empty() || steps_raw.len() > 8 {
        return Err(format!("{name}: a pipe has 1 to 8 steps"));
    }
    let mut steps = Vec::new();
    for step in steps_raw {
        steps.push(parse_step(name, direction, step)?);
    }
    Ok(Pipe {
        id: id.to_string(),
        direction,
        binds,
        then,
        steps,
    })
}

fn parse_step(name: &str, direction: Direction, value: &Value) -> Result<Step, String> {
    let Value::Object(pairs) = value else {
        return Err(format!("{name}: step must be an object"));
    };
    let op = req_str(name, pairs, "op")?;
    match op {
        "append-list" => {
            reject_unknown(name, pairs, &["op", "card", "title", "detail"])?;
            if direction != Direction::Ingress {
                return Err(format!("{name}: append-list is ingress only"));
            }
            let card = id_field(name, pairs, "card")?;
            let title = field_name(name, pairs, "title")?;
            let detail = field_name(name, pairs, "detail")?;
            Ok(Step::AppendList {
                card,
                title,
                detail,
            })
        }
        "inc-stat" => {
            reject_unknown(name, pairs, &["op", "card", "by"])?;
            let card = id_field(name, pairs, "card")?;
            let by = req_int(name, pairs, "by")?;
            if !(1..=100).contains(&by) {
                return Err(format!("{name}: by is out of range"));
            }
            Ok(Step::IncStat { card, by })
        }
        "set-gauge" => {
            reject_unknown(name, pairs, &["op", "card", "value", "caption"])?;
            let card = id_field(name, pairs, "card")?;
            let value = req_int(name, pairs, "value")?;
            if !(0..=100).contains(&value) {
                return Err(format!("{name}: gauge value is out of range"));
            }
            let caption = req_str(name, pairs, "caption")?;
            schema::check_plain(caption, 0, 180).map_err(|detail| format!("{name}: {detail}"))?;
            Ok(Step::SetGauge {
                card,
                value: u8::try_from(value).unwrap_or(0),
                caption: caption.to_string(),
            })
        }
        "airlock" => {
            reject_unknown(name, pairs, &["op", "log", "gauge"])?;
            if direction != Direction::Egress {
                return Err(format!("{name}: airlock is egress only"));
            }
            Ok(Step::Airlock {
                log: id_field(name, pairs, "log")?,
                gauge: id_field(name, pairs, "gauge")?,
            })
        }
        _ => Err(format!("{name}: unknown step `{op}`")),
    }
}

fn parse_policy(name: &str, text: &str) -> Result<Policy, String> {
    let value = json::parse(text)
        .map_err(|error| format!("{name}: {} at {}", error.message, error.position))?;
    let Value::Object(pairs) = &value else {
        return Err(format!("{name}: expected an object"));
    };
    reject_unknown(
        name,
        pairs,
        &["https_only", "max_body", "content_types", "allow"],
    )?;
    let https_only = match req(name, pairs, "https_only")? {
        Value::Bool(flag) => *flag,
        _ => return Err(format!("{name}: https_only must be a boolean")),
    };
    let max_body = req_int(name, pairs, "max_body")?;
    if !(1..=65_536).contains(&max_body) {
        return Err(format!("{name}: max_body is out of range"));
    }
    let types_value = req(name, pairs, "content_types")?;
    let Value::Array(types) = types_value else {
        return Err(format!("{name}: content_types must be an array"));
    };
    if types.is_empty() {
        return Err(format!("{name}: content_types is empty"));
    }
    let mut content_types = Vec::new();
    for item in types {
        let Some(text) = item.as_str() else {
            return Err(format!("{name}: content type must be a string"));
        };
        if !is_media(text) {
            return Err(format!("{name}: bad content type"));
        }
        content_types.push(text.to_string());
    }
    let allow_value = req(name, pairs, "allow")?;
    let Value::Array(entries) = allow_value else {
        return Err(format!("{name}: allow must be an array"));
    };
    let mut allow = Vec::new();
    for entry in entries {
        let Value::Object(fields) = entry else {
            return Err(format!("{name}: allow entry must be an object"));
        };
        reject_unknown(name, fields, &["origin", "methods", "pipes"])?;
        let origin = parse_origin(req_str(name, fields, "origin")?)
            .map_err(|_| format!("{name}: bad origin"))?;
        let mut methods = string_list(name, fields, "methods")?;
        if methods.is_empty() {
            return Err(format!("{name}: methods is empty"));
        }
        for method in &mut methods {
            *method = method.to_ascii_uppercase();
            if !matches!(method.as_str(), "GET" | "POST" | "PUT" | "PATCH" | "DELETE") {
                return Err(format!("{name}: bad method"));
            }
        }
        let pipes = string_list(name, fields, "pipes")?;
        if pipes.is_empty() {
            return Err(format!("{name}: pipes is empty"));
        }
        allow.push(Allow {
            origin,
            methods,
            pipes,
        });
    }
    Ok(Policy {
        https_only,
        max_body: max_body as usize,
        content_types,
        allow,
    })
}

fn link(cards: &[Value], pipes: &[Pipe]) -> Result<(), String> {
    let mut seen_cards = Vec::new();
    for card in cards {
        let id = card.get("id").and_then(Value::as_str).unwrap_or("");
        if seen_cards.contains(&id) {
            return Err(format!("duplicate card `{id}`"));
        }
        seen_cards.push(id);
    }
    let mut seen_pipes = Vec::new();
    for pipe in pipes {
        if seen_pipes.contains(&pipe.id) {
            return Err(format!("duplicate pipe `{}`", pipe.id));
        }
        seen_pipes.push(pipe.id.clone());
    }
    for pipe in pipes {
        for next in &pipe.then {
            let target = pipes
                .iter()
                .find(|item| item.id == *next)
                .ok_or_else(|| format!("{} then `{next}` is missing", pipe.id))?;
            if target.direction != Direction::Internal {
                return Err(format!("{} then `{next}` is not internal", pipe.id));
            }
        }
        for bind in &pipe.binds {
            card_by(cards, bind)?;
        }
        for step in &pipe.steps {
            match step {
                Step::AppendList { card, .. } => {
                    require_kind(cards, card, Kind::List)?;
                }
                Step::IncStat { card, .. } => {
                    require_kind(cards, card, Kind::Stat)?;
                }
                Step::SetGauge { card, .. } => {
                    require_kind(cards, card, Kind::Gauge)?;
                }
                Step::Airlock { log, gauge } => {
                    require_kind(cards, log, Kind::List)?;
                    require_kind(cards, gauge, Kind::Gauge)?;
                }
            }
        }
    }
    for card in cards {
        if schema::Kind::parse(card.get("kind").and_then(Value::as_str).unwrap_or(""))
            == Some(Kind::Form)
        {
            let pipe_id = card
                .get("body")
                .and_then(|body| body.get("pipe"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let pipe = pipes
                .iter()
                .find(|pipe| pipe.id == pipe_id)
                .ok_or_else(|| format!("form pipe `{pipe_id}` is missing"))?;
            if pipe.direction != Direction::Ingress {
                return Err(format!("form pipe `{pipe_id}` is not ingress"));
            }
        }
    }
    let mut color = vec![0u8; pipes.len()];
    for index in 0..pipes.len() {
        walk(pipes, index, &mut color)?;
    }
    Ok(())
}

fn walk(pipes: &[Pipe], index: usize, color: &mut [u8]) -> Result<(), String> {
    if color[index] == 1 {
        return Err(format!("pipe cycle at `{}`", pipes[index].id));
    }
    if color[index] == 2 {
        return Ok(());
    }
    color[index] = 1;
    for next in &pipes[index].then {
        let target = pipes
            .iter()
            .position(|pipe| pipe.id == *next)
            .expect("then target checked");
        walk(pipes, target, color)?;
    }
    color[index] = 2;
    Ok(())
}

fn check_policy_pipes(policy: &Policy, pipes: &[Pipe]) -> Result<(), String> {
    for entry in &policy.allow {
        for pipe_id in &entry.pipes {
            let pipe = pipes
                .iter()
                .find(|pipe| pipe.id == *pipe_id)
                .ok_or_else(|| format!("policy names missing pipe `{pipe_id}`"))?;
            if pipe.direction != Direction::Egress {
                return Err(format!("policy allowlists non-egress pipe `{pipe_id}`"));
            }
        }
    }
    Ok(())
}

fn card_by<'a>(cards: &'a [Value], id: &str) -> Result<&'a Value, String> {
    cards
        .iter()
        .find(|card| card.get("id").and_then(Value::as_str) == Some(id))
        .ok_or_else(|| format!("missing card `{id}`"))
}

fn require_kind(cards: &[Value], id: &str, kind: Kind) -> Result<(), String> {
    let card = card_by(cards, id)?;
    let found = schema::Kind::parse(card.get("kind").and_then(Value::as_str).unwrap_or(""));
    if found != Some(kind) {
        return Err(format!("`{id}` must be a {} card", kind.as_str()));
    }
    Ok(())
}

fn read_tree(root: &Path, folder: &str) -> Result<Vec<(String, String)>, String> {
    let dir = rooted(root, &root.join(folder))?;
    let mut files = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|_| format!("cannot read {}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|_| format!("cannot read {}", dir.display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.ends_with(".json") || name.contains('/') || name.contains('\\') {
            continue;
        }
        let path = rooted(root, &entry.path())?;
        files.push((name, read_capped(&path)?));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    if files.is_empty() {
        return Err(format!("{folder} has no json"));
    }
    if files.len() > 64 {
        return Err(format!("{folder} has too many files"));
    }
    Ok(files)
}

fn rooted(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let canonical = path
        .canonicalize()
        .map_err(|_| format!("{} is not readable", path.display()))?;
    if !canonical.starts_with(root) {
        return Err(format!("{} is outside modules", path.display()));
    }
    Ok(canonical)
}

fn read_capped(path: &Path) -> Result<String, String> {
    let meta = fs::metadata(path).map_err(|_| format!("{} is not readable", path.display()))?;
    if !meta.is_file() || meta.len() > 65_536 {
        return Err(format!("{} is too large or not a file", path.display()));
    }
    fs::read_to_string(path).map_err(|_| format!("{} is not utf-8", path.display()))
}

fn reject_unknown(name: &str, pairs: &[(String, Value)], allowed: &[&str]) -> Result<(), String> {
    for (key, _) in pairs {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("{name}: unknown field `{key}`"));
        }
    }
    Ok(())
}

fn req<'a>(name: &str, pairs: &'a [(String, Value)], key: &str) -> Result<&'a Value, String> {
    pairs
        .iter()
        .find(|(field, _)| field == key)
        .map(|(_, value)| value)
        .ok_or_else(|| format!("{name}: missing `{key}`"))
}

fn req_str<'a>(name: &str, pairs: &'a [(String, Value)], key: &str) -> Result<&'a str, String> {
    req(name, pairs, key)?
        .as_str()
        .ok_or_else(|| format!("{name}: `{key}` must be a string"))
}

fn req_int(name: &str, pairs: &[(String, Value)], key: &str) -> Result<i64, String> {
    req(name, pairs, key)?
        .as_i64()
        .ok_or_else(|| format!("{name}: `{key}` must be an integer"))
}

fn id_field(name: &str, pairs: &[(String, Value)], key: &str) -> Result<String, String> {
    let value = req_str(name, pairs, key)?;
    if !schema::is_id(value) {
        return Err(format!("{name}: `{key}` is not an id"));
    }
    Ok(value.to_string())
}

fn field_name(name: &str, pairs: &[(String, Value)], key: &str) -> Result<String, String> {
    let value = req_str(name, pairs, key)?;
    if !schema::is_field(value) {
        return Err(format!("{name}: `{key}` is not a field name"));
    }
    Ok(value.to_string())
}

fn string_list(name: &str, pairs: &[(String, Value)], key: &str) -> Result<Vec<String>, String> {
    let Value::Array(items) = req(name, pairs, key)? else {
        return Err(format!("{name}: `{key}` must be an array"));
    };
    let mut out = Vec::new();
    for item in items {
        let Some(text) = item.as_str() else {
            return Err(format!("{name}: `{key}` must be strings"));
        };
        out.push(text.to_string());
    }
    Ok(out)
}

fn is_media(value: &str) -> bool {
    let Some((left, right)) = value.split_once('/') else {
        return false;
    };
    let token = |part: &str| {
        !part.is_empty()
            && part.len() <= 64
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
    };
    token(left) && token(right) && !value[left.len() + 1..].contains('/')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_bundle_links() {
        let bundle = from_embedded().expect("embedded modules");
        assert!(
            bundle
                .pipes
                .iter()
                .any(|pipe| pipe.direction == Direction::Internal)
        );
        assert!(bundle.policy.https_only);
    }

    #[test]
    fn a_symlink_outside_modules_is_refused() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("hull-link-{nanos}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("cards")).unwrap();
        fs::create_dir_all(dir.join("pipes")).unwrap();
        for (name, text) in EMBEDDED_CARDS {
            fs::write(dir.join("cards").join(name), text).unwrap();
        }
        for (name, text) in EMBEDDED_PIPES {
            fs::write(dir.join("pipes").join(name), text).unwrap();
        }
        fs::write(dir.join("policy.json"), EMBEDDED_POLICY).unwrap();
        std::os::unix::fs::symlink("/etc/passwd", dir.join("cards").join("evil.json")).unwrap();
        let error = from_dir(&dir).unwrap_err();
        assert!(
            error.contains("outside") || error.contains("evil.json"),
            "{error}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
