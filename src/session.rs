//! In-memory sessions. A session binds one profile to one channel until it expires.
//! Restart drops the table. Nothing here is written to disk or to a log.

const CAP: usize = 32;
pub const TTL_MS: u64 = 15 * 60 * 1000;

struct Row {
    id: String,
    profile: String,
    channel: String,
    expires_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issued {
    pub id: String,
    pub profile: String,
    pub channel: String,
    pub expires_at: u64,
    pub reused: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    Unauthorized,
    Unavailable,
}

#[derive(Default)]
pub struct Store {
    rows: Vec<Row>,
}

impl Store {
    pub fn mint(
        &mut self,
        profile: &str,
        channel: &str,
        now: u64,
        entropy: [u8; 16],
    ) -> Result<Issued, Fault> {
        self.purge(now);
        if let Some(row) = self
            .rows
            .iter()
            .find(|row| row.profile == profile && row.channel == channel)
        {
            return Ok(issued(row, true));
        }
        if self.rows.len() >= CAP {
            return Err(Fault::Unavailable);
        }
        let id = session_id(&entropy);
        if self.rows.iter().any(|row| row.id == id) {
            return Err(Fault::Unavailable);
        }
        let row = Row {
            id,
            profile: profile.to_string(),
            channel: channel.to_string(),
            expires_at: now.saturating_add(TTL_MS),
        };
        let issued = issued(&row, false);
        self.rows.push(row);
        Ok(issued)
    }

    pub fn resume(&mut self, id: &str, now: u64) -> Result<(String, String), Fault> {
        self.purge(now);
        self.rows
            .iter()
            .find(|row| row.id == id)
            .map(|row| (row.profile.clone(), row.channel.clone()))
            .ok_or(Fault::Unauthorized)
    }

    pub fn revoke(&mut self, id: &str, now: u64) -> Result<(), Fault> {
        self.purge(now);
        let before = self.rows.len();
        self.rows.retain(|row| row.id != id);
        if self.rows.len() == before {
            Err(Fault::Unauthorized)
        } else {
            Ok(())
        }
    }
}

pub fn looks_like_id(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("ses_") else {
        return false;
    };
    rest.len() == 32
        && rest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn session_id(entropy: &[u8; 16]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::from("ses_");
    for byte in entropy {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0xf) as usize] as char);
    }
    out
}

fn issued(row: &Row, reused: bool) -> Issued {
    Issued {
        id: row.id.clone(),
        profile: row.profile.clone(),
        channel: row.channel.clone(),
        expires_at: row.expires_at,
        reused,
    }
}

impl Store {
    fn purge(&mut self, now: u64) {
        self.rows.retain(|row| row.expires_at > now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expiry_is_the_same_as_unknown_and_a_live_one_is_reused() {
        let mut store = Store::default();
        let first = store.mint("shell", "deck", 1_000, [1u8; 16]).unwrap();
        let again = store.mint("shell", "deck", 1_000, [2u8; 16]).unwrap();
        assert!(again.reused);
        assert_eq!(again.id, first.id);
        assert!(store.resume(&first.id, 1_000).is_ok());
        assert!(store.resume(&first.id, first.expires_at).is_err());
        let fresh = store.mint("shell", "deck", 2_000, [3u8; 16]).unwrap();
        store.revoke(&fresh.id, 2_000).unwrap();
        assert!(store.resume(&fresh.id, 2_000).is_err());
    }

    #[test]
    fn a_full_table_does_not_drop_a_live_session() {
        let mut store = Store::default();
        for n in 0..32 {
            let mut entropy = [0u8; 16];
            entropy[0] = n;
            store.mint("shell", &format!("c{n}"), 1, entropy).unwrap();
        }
        assert!(store.mint("shell", "overflow", 1, [9u8; 16]).is_err());
        assert!(store.resume(&session_id(&[0u8; 16]), 1).is_ok());
    }
}
