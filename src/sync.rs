//! Handshake plus a ring of sparse patches. Clients that fall off the ring get a reset.

use std::collections::VecDeque;

use crate::json::{Number, Value};

pub const TAPE_CAP: usize = 200;

#[derive(Clone, Debug, PartialEq)]
pub struct Patch {
    pub cursor: u64,
    pub path: String,
    pub preview: String,
    pub value: Value,
}

#[derive(Clone, Debug)]
pub struct Tape {
    cursor: u64,
    patches: VecDeque<Patch>,
}

#[derive(Clone, Debug)]
pub struct Delta {
    pub reset: bool,
    pub cursor: u64,
    pub patches: Vec<Patch>,
}

impl Tape {
    pub fn new() -> Self {
        Self {
            cursor: 0,
            patches: VecDeque::new(),
        }
    }

    pub fn cursor(&self) -> u64 {
        self.cursor
    }

    pub fn record(&mut self, path: impl Into<String>, value: &Value) {
        self.cursor += 1;
        let path = path.into();
        self.patches.push_back(Patch {
            cursor: self.cursor,
            path,
            preview: preview(value),
            value: value.clone(),
        });
        while self.patches.len() > TAPE_CAP {
            self.patches.pop_front();
        }
    }

    pub fn since(&self, cursor: Option<u64>) -> Delta {
        let Some(cursor) = cursor else {
            return Delta {
                reset: true,
                cursor: self.cursor,
                patches: Vec::new(),
            };
        };
        if cursor > self.cursor {
            return Delta {
                reset: true,
                cursor: self.cursor,
                patches: Vec::new(),
            };
        }
        if cursor == self.cursor {
            return Delta {
                reset: false,
                cursor: self.cursor,
                patches: Vec::new(),
            };
        }
        let oldest = self.patches.front().map(|patch| patch.cursor);
        let needs_reset = match oldest {
            Some(oldest) => cursor + 1 < oldest,
            None => cursor != self.cursor,
        };
        if needs_reset {
            return Delta {
                reset: true,
                cursor: self.cursor,
                patches: Vec::new(),
            };
        }
        Delta {
            reset: false,
            cursor: self.cursor,
            patches: self
                .patches
                .iter()
                .filter(|patch| patch.cursor > cursor)
                .cloned()
                .collect(),
        }
    }

    pub fn recent(&self, limit: usize) -> Vec<&Patch> {
        self.patches.iter().rev().take(limit).collect()
    }
}

impl Default for Tape {
    fn default() -> Self {
        Self::new()
    }
}

pub fn preview(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(Number::Int(number)) => number.to_string(),
        Value::Number(Number::Raw(raw)) => raw.clone(),
        Value::String(text) => {
            let mut clipped: String = text.chars().take(48).collect();
            if text.chars().count() > 48 {
                clipped.push('…');
            }
            format!("\"{clipped}\"")
        }
        Value::Array(items) => format!("[{} items]", items.len()),
        Value::Object(_) => "{…}".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_patches_and_a_stale_cursor_resets() {
        let mut tape = Tape::new();
        tape.record(
            "/cards/card.covers/body/value",
            &Value::Number(Number::Int(1)),
        );
        tape.record("/cards/card.pass/body/items", &Value::Array(vec![]));
        let delta = tape.since(Some(1));
        assert!(!delta.reset);
        assert_eq!(delta.patches.len(), 1);
        assert_eq!(delta.patches[0].path, "/cards/card.pass/body/items");
        assert!(tape.since(Some(2)).patches.is_empty());
        assert!(tape.since(None).reset);
        for index in 0..TAPE_CAP + 5 {
            tape.record(
                format!("/n/{index}"),
                &Value::Number(Number::Int(index as i64)),
            );
        }
        assert!(tape.since(Some(1)).reset);
        assert!(!tape.since(Some(tape.cursor() - 1)).reset);
    }
}
