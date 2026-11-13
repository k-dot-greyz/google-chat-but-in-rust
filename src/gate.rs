//! Write gate. A profile is a name with a closed allowlist, not a credential.
//! The table is compiled in. A watched directory must not be able to mint one.

use crate::http::Request;
use crate::json::{self, Value};

const TABLE: &str = include_str!("../modules/profiles.json");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitGrant {
    pub installation: String,
    pub repo: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    pub id: String,
    pub pipes: Vec<String>,
    pub verbs: Vec<String>,
    pub git: Vec<GitGrant>,
}

pub enum Need<'a> {
    Cards,
    Pipe(&'a str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateFault {
    pub status: u16,
    pub code: &'static str,
    pub detail: String,
}

pub fn profiles() -> Vec<Profile> {
    parse_table(TABLE).expect("modules/profiles.json")
}

pub fn admit(request: &Request, need: Need<'_>) -> Result<Profile, GateFault> {
    reject_query_secrets(request)?;
    let id = profile_header(request)?;
    let profile = profiles()
        .into_iter()
        .find(|profile| profile.id == id)
        .ok_or_else(unknown)?;
    match need {
        Need::Cards => {
            if !profile.verbs.iter().any(|verb| verb == "mutate-cards") {
                return Err(scope());
            }
        }
        Need::Pipe(pipe) => {
            if pipe.starts_with("internal.") || !profile.pipes.iter().any(|allowed| allowed == pipe)
            {
                return Err(scope());
            }
        }
    }
    Ok(profile)
}

pub fn may_file(profile: &Profile, installation: &str, repo: &str) -> bool {
    profile.verbs.iter().any(|verb| verb == "file-issue")
        && profile
            .git
            .iter()
            .any(|grant| grant.installation == installation && grant.repo == repo)
}

fn reject_query_secrets(request: &Request) -> Result<(), GateFault> {
    const SECRET: &[&str] = &[
        "profile",
        "x-profile",
        "token",
        "authorization",
        "access_token",
        "id_token",
        "code",
        "secret",
    ];
    let leaked = request.query.iter().any(|(key, _)| {
        let key = key.to_ascii_lowercase();
        SECRET.contains(&key.as_str())
    });
    if leaked {
        Err(GateFault {
            status: 400,
            code: "query",
            detail: "Secrets do not ride the query string.".into(),
        })
    } else {
        Ok(())
    }
}

fn profile_header(request: &Request) -> Result<String, GateFault> {
    let values: Vec<&str> = request
        .headers
        .iter()
        .filter(|(name, _)| name == "x-profile")
        .map(|(_, value)| value.as_str())
        .collect();
    let [value] = values.as_slice() else {
        return Err(unknown());
    };
    if !valid_id(value) {
        return Err(unknown());
    }
    Ok((*value).to_string())
}

fn valid_id(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    value.len() <= 32
        && first.is_ascii_lowercase()
        && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

fn unknown() -> GateFault {
    GateFault {
        status: 401,
        code: "profile",
        detail: "That profile is not on the hull.".into(),
    }
}

fn scope() -> GateFault {
    GateFault {
        status: 403,
        code: "scope",
        detail: "That profile cannot take this action.".into(),
    }
}

fn parse_table(text: &str) -> Result<Vec<Profile>, String> {
    let value = json::parse(text).map_err(|error| error.message)?;
    let pairs = object(&value, &["profiles"])?;
    let items = pairs
        .iter()
        .find(|(key, _)| key == "profiles")
        .and_then(|(_, value)| value.as_array())
        .ok_or_else(|| "profiles must be an array".to_string())?;
    if items.is_empty() || items.len() > 16 {
        return Err("profile table length".into());
    }
    let mut profiles = Vec::with_capacity(items.len());
    for item in items {
        profiles.push(parse_profile(item)?);
    }
    let mut ids: Vec<&str> = profiles.iter().map(|profile| profile.id.as_str()).collect();
    ids.sort_unstable();
    if ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("duplicate profile id".into());
    }
    Ok(profiles)
}

fn parse_profile(value: &Value) -> Result<Profile, String> {
    let pairs = object(value, &["id", "pipes", "verbs", "git"])?;
    let id = req_str(pairs, "id")?;
    if !valid_id(id) {
        return Err(format!("bad profile id {id}"));
    }
    Ok(Profile {
        id: id.to_string(),
        pipes: req_strings(pairs, "pipes", valid_pipe)?,
        verbs: req_strings(pairs, "verbs", valid_id)?,
        git: req_grants(pairs)?,
    })
}

fn req_grants(pairs: &[(String, Value)]) -> Result<Vec<GitGrant>, String> {
    let items = pairs
        .iter()
        .find(|(key, _)| key == "git")
        .and_then(|(_, value)| value.as_array())
        .ok_or_else(|| "git must be an array".to_string())?;
    if items.len() > 8 {
        return Err("too many git grants".into());
    }
    items
        .iter()
        .map(|item| {
            let fields = object(item, &["installation", "repo"])?;
            let installation = req_str(fields, "installation")?;
            let repo = req_str(fields, "repo")?;
            if !valid_id(installation) || !valid_repo(repo) {
                return Err("bad git grant".into());
            }
            Ok(GitGrant {
                installation: installation.to_string(),
                repo: repo.to_string(),
            })
        })
        .collect()
}

fn req_strings(
    pairs: &[(String, Value)],
    key: &str,
    valid: fn(&str) -> bool,
) -> Result<Vec<String>, String> {
    let items = pairs
        .iter()
        .find(|(name, _)| name == key)
        .and_then(|(_, value)| value.as_array())
        .ok_or_else(|| format!("{key} must be an array"))?;
    if items.len() > 16 {
        return Err(format!("{key} is too long"));
    }
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(text) = item.as_str() else {
            return Err(format!("{key} entries are strings"));
        };
        if !valid(text) {
            return Err(format!("bad {key} entry"));
        }
        if out.iter().any(|seen| seen == text) {
            return Err(format!("duplicate {key} entry"));
        }
        out.push(text.to_string());
    }
    Ok(out)
}

fn valid_pipe(value: &str) -> bool {
    !value.starts_with("internal.")
        && value.len() <= 64
        && value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '.' || ch == '-')
        && value.contains('.')
        && !value.contains("..")
        && !value.starts_with('.')
        && !value.ends_with('.')
}

fn valid_repo(value: &str) -> bool {
    let Some((owner, name)) = value.split_once('/') else {
        return false;
    };
    !value.contains("//")
        && value.len() <= 80
        && !owner.is_empty()
        && !name.is_empty()
        && !value.chars().any(char::is_whitespace)
        && owner
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '.')
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '.' || ch == '_')
}

fn object<'a>(value: &'a Value, allowed: &[&str]) -> Result<&'a [(String, Value)], String> {
    let Value::Object(pairs) = value else {
        return Err("expected an object".into());
    };
    for (key, _) in pairs {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unknown field {key}"));
        }
    }
    for key in allowed {
        if !pairs.iter().any(|(name, _)| name == key) {
            return Err(format!("missing {key}"));
        }
    }
    Ok(pairs)
}

fn req_str<'a>(pairs: &'a [(String, Value)], key: &str) -> Result<&'a str, String> {
    pairs
        .iter()
        .find(|(name, _)| name == key)
        .and_then(|(_, value)| value.as_str())
        .ok_or_else(|| format!("{key} must be a string"))
}
