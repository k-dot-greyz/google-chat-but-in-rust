//! The hull. Dispatch, pipes, and rollbacks live here. Permits do not.

use std::path::Path;

use crate::airlock::{self, EgressRequest, authorize, dry_dock, parse_probe, permit_notice};
use crate::gate::{self, Need};
use crate::http::{self, Request, Response, parse_form};
use crate::json::{self, Number, Value};
use crate::load::{self, Bundle, Direction, Pipe, Step};
use crate::render;
use crate::schema::{self, Kind};
use crate::sync::{Patch, Tape};

const CARD_CAP: usize = 32;
const LIST_CAP: usize = 24;
const STAT_CAP: i64 = 10_000;

pub struct Hull {
    epoch: String,
    cards: Vec<Value>,
    pipes: Vec<Pipe>,
    policy: airlock::Policy,
    tape: Tape,
    seal_word: String,
    seal_detail: String,
    fault: Option<String>,
    notice: String,
    verdict: Option<Value>,
}

struct Fault {
    status: u16,
    code: &'static str,
    detail: String,
}

impl Fault {
    fn new(status: u16, code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            status,
            code,
            detail: detail.into(),
        }
    }
}

struct Snap {
    cards: Vec<Value>,
    tape: Tape,
    seal_word: String,
    seal_detail: String,
}

struct Expect {
    required: Vec<String>,
    optional: Vec<String>,
    probe: bool,
}

impl Hull {
    pub fn boot_embedded() -> Self {
        Self::from_bundle(load::from_embedded().expect("embedded modules"), None)
    }

    pub fn boot_dir(path: &Path) -> Self {
        match load::from_dir(path) {
            Ok(bundle) => Self::from_bundle(bundle, None),
            Err(detail) => Self::from_bundle(
                load::from_embedded().expect("embedded modules"),
                Some(detail),
            ),
        }
    }

    pub fn reload_dir(&mut self, path: &Path) {
        match load::from_dir(path) {
            Ok(bundle) if bundle.epoch == self.epoch => self.fault = None,
            Ok(bundle) => {
                *self = Self::from_bundle(bundle, None);
                self.notice = "Modules reloaded from disk.".into();
            }
            Err(detail) => self.fault = Some(detail),
        }
    }

    fn from_bundle(bundle: Bundle, fault: Option<String>) -> Self {
        Self {
            epoch: bundle.epoch,
            cards: bundle.cards,
            pipes: bundle.pipes,
            policy: bundle.policy,
            tape: Tape::new(),
            seal_word: "SEALED".into(),
            seal_detail: "Default deny. No probe yet.".into(),
            fault,
            notice: String::new(),
            verdict: None,
        }
    }

    pub fn epoch(&self) -> &str {
        &self.epoch
    }
    pub fn cursor(&self) -> u64 {
        self.tape.cursor()
    }
    pub fn notice(&self) -> &str {
        &self.notice
    }
    pub fn fault(&self) -> Option<&str> {
        self.fault.as_deref()
    }
    pub fn seal_word(&self) -> &str {
        &self.seal_word
    }
    pub fn seal_detail(&self) -> &str {
        &self.seal_detail
    }
    pub fn pipes(&self) -> &[Pipe] {
        &self.pipes
    }
    pub fn card(&self, id: &str) -> Option<&Value> {
        self.cards.iter().find(|card| id_of(card) == id)
    }
    pub fn cards(&self) -> &[Value] {
        &self.cards
    }
    pub fn tape_recent(&self, limit: usize) -> Vec<&Patch> {
        self.tape.recent(limit)
    }

    pub fn dispatch(&mut self, request: &Request) -> Response {
        if request.method == "OPTIONS" {
            return Response::empty(204);
        }
        let head = request.method == "HEAD";
        let method = if head { "GET" } else { request.method.as_str() };
        let segs: Vec<&str> = request.segments.iter().map(String::as_str).collect();
        let response = match (method, segs.as_slice()) {
            ("GET", []) => Response::html(200, render::page(self, None, surface_of(request))),
            ("GET", ["favicon.ico"]) => self.asset("mark.svg"),
            ("GET", ["assets", name]) => self.asset(name),
            ("GET", ["api", "health"]) => self.health(),
            ("GET", ["api", "schema"]) => Response::json(200, &schema::document()),
            ("GET", ["api", "view"]) => Response::json(200, &render::view(self)),
            ("GET", ["api", "cards"]) => self.cards_json(),
            ("GET", ["api", "cards", id]) => self.one_card(id),
            ("POST", ["api", "cards"]) => self.create_card(request),
            ("PATCH", ["api", "cards", id]) => self.patch_card(request, id),
            ("DELETE", ["api", "cards", id]) => self.delete_card(request, id),
            ("GET", ["api", "pipes"]) => self.pipes_json(),
            ("POST", ["api", "pipes", id, "push"]) => self.push_route(request, id),
            ("GET", ["api", "airlock"]) => self.airlock_json(),
            ("GET", ["api", "pipeline"]) => self.pipeline_json(),
            ("GET", ["api", "sync", "handshake"]) => self.handshake(),
            ("GET", ["api", "sync", "delta"]) => self.delta(request),
            _ => self.miss(request),
        };
        if head {
            response.without_body()
        } else {
            response
        }
    }

    fn asset(&self, name: &str) -> Response {
        match render::asset(name) {
            Some((content_type, body)) => Response::asset(content_type, &body),
            None => Response::fail(404, "missing", "That asset is not on the pass."),
        }
    }

    fn miss(&self, request: &Request) -> Response {
        if request.segments.first().map(String::as_str) == Some("api") || request.method != "GET" {
            Response::fail(404, "missing", "That route is not on the pass.")
        } else {
            Response::html(
                404,
                render::page(
                    self,
                    Some("That route is not on the pass."),
                    surface_of(request),
                ),
            )
        }
    }

    fn health(&self) -> Response {
        Response::json(
            200,
            &obj(vec![
                ("ok", Value::Bool(true)),
                ("service", Value::String("hull".into())),
                ("epoch", Value::String(self.epoch.clone())),
                ("cursor", json_u64(self.cursor())),
                ("seal", Value::String(self.seal_word.clone())),
            ]),
        )
    }

    fn handshake(&self) -> Response {
        Response::json(
            200,
            &obj(vec![
                ("ok", Value::Bool(true)),
                ("protocol", Value::String("sysex-delta/1".into())),
                ("schema_rev", json_i64(schema::SCHEMA_REV)),
                ("epoch", Value::String(self.epoch.clone())),
                ("cursor", json_u64(self.cursor())),
                (
                    "types",
                    Value::Array(
                        ["Card", "Plate", "Probe", "AirlockDecision", "Delta"]
                            .into_iter()
                            .map(|name| Value::String(name.into()))
                            .collect(),
                    ),
                ),
            ]),
        )
    }

    fn delta(&self, request: &Request) -> Response {
        let since = match cursor_query(request) {
            Ok(since) => since,
            Err(fault) => return Response::fail(fault.status, fault.code, &fault.detail),
        };
        Response::json(200, &self.delta_value(since))
    }

    fn cards_json(&self) -> Response {
        Response::json(
            200,
            &obj(vec![
                ("ok", Value::Bool(true)),
                ("cards", Value::Array(self.cards.clone())),
            ]),
        )
    }

    fn one_card(&self, id: &str) -> Response {
        match self.card(id) {
            Some(card) => Response::json(
                200,
                &obj(vec![("ok", Value::Bool(true)), ("card", card.clone())]),
            ),
            None => Response::fail(404, "missing", "No card by that id."),
        }
    }

    fn pipes_json(&self) -> Response {
        let pipes = self
            .pipes
            .iter()
            .map(|pipe| {
                obj(vec![
                    ("id", Value::String(pipe.id.clone())),
                    ("direction", Value::String(pipe.direction.as_str().into())),
                    (
                        "binds",
                        Value::Array(pipe.binds.iter().cloned().map(Value::String).collect()),
                    ),
                    (
                        "then",
                        Value::Array(pipe.then.iter().cloned().map(Value::String).collect()),
                    ),
                    ("summary", Value::String(pipe.summary())),
                ])
            })
            .collect();
        Response::json(
            200,
            &obj(vec![
                ("ok", Value::Bool(true)),
                ("pipes", Value::Array(pipes)),
            ]),
        )
    }

    fn airlock_json(&self) -> Response {
        let allow = self
            .policy
            .allow
            .iter()
            .map(|entry| {
                obj(vec![
                    ("origin", Value::String(entry.origin.key())),
                    (
                        "methods",
                        Value::Array(entry.methods.iter().cloned().map(Value::String).collect()),
                    ),
                    (
                        "pipes",
                        Value::Array(entry.pipes.iter().cloned().map(Value::String).collect()),
                    ),
                ])
            })
            .collect();
        Response::json(
            200,
            &obj(vec![
                ("ok", Value::Bool(true)),
                ("https_only", Value::Bool(self.policy.https_only)),
                ("max_body", json_u64(self.policy.max_body as u64)),
                ("allow", Value::Array(allow)),
                (
                    "seal",
                    obj(vec![
                        ("word", Value::String(self.seal_word.clone())),
                        ("detail", Value::String(self.seal_detail.clone())),
                    ]),
                ),
                ("sent_socket", Value::Bool(false)),
            ]),
        )
    }

    fn pipeline_json(&self) -> Response {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        for card in &self.cards {
            nodes.push(obj(vec![
                ("id", Value::String(id_of(card).into())),
                ("type", Value::String("card".into())),
                (
                    "kind",
                    Value::String(
                        card.get("kind")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .into(),
                    ),
                ),
            ]));
        }
        for pipe in &self.pipes {
            nodes.push(obj(vec![
                ("id", Value::String(pipe.id.clone())),
                ("type", Value::String("pipe".into())),
                ("direction", Value::String(pipe.direction.as_str().into())),
            ]));
            for bind in &pipe.binds {
                edges.push(edge(&pipe.id, bind, "binds"));
            }
            for next in &pipe.then {
                edges.push(edge(&pipe.id, next, "then"));
            }
        }
        Response::json(
            200,
            &obj(vec![
                ("ok", Value::Bool(true)),
                ("epoch", Value::String(self.epoch.clone())),
                (
                    "fault",
                    self.fault.clone().map(Value::String).unwrap_or(Value::Null),
                ),
                ("nodes", Value::Array(nodes)),
                ("edges", Value::Array(edges)),
            ]),
        )
    }

    fn create_card(&mut self, request: &Request) -> Response {
        let before = self.cursor();
        let result = (|| {
            admit_request(request, Need::Cards)?;
            let mut card = require_json(request)?;
            if self.cards.len() >= CARD_CAP {
                return Err(Fault::new(422, "cap", "The board holds 32 cards."));
            }
            set_field(&mut card, "rev", json_i64(1));
            schema::validate_card(&card)
                .map_err(|issues| Fault::new(422, "schema", schema::issues_text(&issues)))?;
            let id = id_of(&card).to_string();
            if self.card(&id).is_some() {
                return Err(Fault::new(
                    409,
                    "conflict",
                    "A card with that id is already on the board.",
                ));
            }
            self.cards.push(card);
            let stored = self.cards.last().cloned().expect("card");
            self.tape.record(format!("/cards/{id}"), &stored);
            self.notice = format!("Card {id} is on the board.");
            Ok(())
        })();
        self.settle(request, before, result)
    }

    fn patch_card(&mut self, request: &Request, id: &str) -> Response {
        let before = self.cursor();
        let result = (|| {
            admit_request(request, Need::Cards)?;
            require_intent(request)?;
            let patch = require_json(request)?;
            let Value::Object(pairs) = &patch else {
                return Err(Fault::new(422, "schema", "A patch is an object."));
            };
            for (key, _) in pairs {
                if key != "title" && key != "body" {
                    return Err(Fault::new(422, "schema", format!("Unknown field `{key}`.")));
                }
            }
            let index = self
                .index_of(id)
                .ok_or_else(|| Fault::new(404, "missing", "No card by that id."))?;
            let mut next = self.cards[index].clone();
            if let Some(title) = patch.get("title") {
                let Some(text) = title.as_str() else {
                    return Err(Fault::new(422, "schema", "Title must be a string."));
                };
                schema::check_plain(text, 1, 80)
                    .map_err(|detail| Fault::new(422, "schema", detail))?;
                set_field(&mut next, "title", Value::String(text.to_string()));
            }
            if let Some(body_patch) = patch.get("body") {
                merge_body(&mut next, body_patch)?;
            }
            let rev = next
                .get("rev")
                .and_then(Value::as_i64)
                .unwrap_or(1)
                .saturating_add(1);
            set_field(&mut next, "rev", json_i64(rev));
            schema::validate_card(&next)
                .map_err(|issues| Fault::new(422, "schema", schema::issues_text(&issues)))?;
            self.cards[index] = next;
            if patch.get("title").is_some() {
                let title = self.cards[index]
                    .get("title")
                    .cloned()
                    .unwrap_or(Value::Null);
                self.tape.record(format!("/cards/{id}/title"), &title);
            }
            if let Some(Value::Object(body_fields)) = patch.get("body") {
                for (key, _) in body_fields {
                    let value = self.cards[index]
                        .get("body")
                        .and_then(|body| body.get(key))
                        .cloned()
                        .unwrap_or(Value::Null);
                    self.tape.record(format!("/cards/{id}/body/{key}"), &value);
                }
            }
            let rev_value = self.cards[index].get("rev").cloned().unwrap_or(Value::Null);
            self.tape.record(format!("/cards/{id}/rev"), &rev_value);
            self.notice = format!("Card {id} updated.");
            Ok(())
        })();
        self.settle(request, before, result)
    }

    fn delete_card(&mut self, request: &Request, id: &str) -> Response {
        let before = self.cursor();
        let result = (|| {
            admit_request(request, Need::Cards)?;
            require_intent(request)?;
            let index = self
                .index_of(id)
                .ok_or_else(|| Fault::new(404, "missing", "No card by that id."))?;
            self.cards.remove(index);
            self.tape.record(format!("/cards/{id}"), &Value::Null);
            self.notice = format!("Card {id} left the board.");
            Ok(())
        })();
        self.settle(request, before, result)
    }

    fn push_route(&mut self, request: &Request, id: &str) -> Response {
        let before = self.cursor();
        let result = (|| {
            admit_request(request, Need::Pipe(id))?;
            let body = payload(request)?;
            self.push(id, &body)
        })();
        self.settle(request, before, result)
    }

    fn settle(&mut self, request: &Request, before: u64, result: Result<(), Fault>) -> Response {
        let form = request.content_type() == "application/x-www-form-urlencoded";
        match result {
            Ok(()) if form => Response::redirect(),
            Ok(()) => self.ok_body(before),
            Err(fault) if form && fault.code != "cross-site" => {
                self.notice = fault.detail;
                Response::redirect()
            }
            Err(fault) => Response::fail(fault.status, fault.code, &fault.detail),
        }
    }

    fn ok_body(&self, before: u64) -> Response {
        let mut pairs = vec![
            ("ok", Value::Bool(true)),
            ("cursor", json_u64(self.cursor())),
            ("view", render::view(self)),
            ("delta", self.delta_value(Some(before))),
        ];
        if let Some(verdict) = &self.verdict {
            pairs.push(("verdict", verdict.clone()));
        }
        Response::json(200, &obj(pairs))
    }

    fn push(&mut self, id: &str, body: &Value) -> Result<(), Fault> {
        self.verdict = None;
        let pipe = self
            .pipes
            .iter()
            .find(|pipe| pipe.id == id)
            .cloned()
            .ok_or_else(|| Fault::new(404, "missing", "No pipe by that id."))?;
        if pipe.direction == Direction::Internal {
            return Err(Fault::new(
                403,
                "direction",
                "Internal pipes are not pushed from the street.",
            ));
        }
        let expect = self.expect_of(&pipe.id)?;
        validate_body(&expect, body)?;
        let snap = self.snapshot();
        if let Err(fault) = self.run_chain(&pipe.id, body, &mut Vec::new()) {
            self.restore(snap);
            self.notice = fault.detail.clone();
            return Err(fault);
        }
        if pipe.direction == Direction::Ingress
            && let Some(key) = expect.required.first()
        {
            let dish = trimmed(body, key);
            self.notice = format!("On the pass: {dish}.");
        }
        Ok(())
    }

    fn expect_of(&self, id: &str) -> Result<Expect, Fault> {
        let mut expect = Expect {
            required: Vec::new(),
            optional: Vec::new(),
            probe: false,
        };
        self.gather(id, &mut Vec::new(), &mut expect)?;
        Ok(expect)
    }

    fn gather(&self, id: &str, seen: &mut Vec<String>, expect: &mut Expect) -> Result<(), Fault> {
        if seen.iter().any(|item| item == id) {
            return Err(Fault::new(422, "pipe", "Pipe cycle."));
        }
        seen.push(id.to_string());
        let pipe = self
            .pipes
            .iter()
            .find(|pipe| pipe.id == id)
            .ok_or_else(|| Fault::new(404, "missing", format!("No pipe `{id}`.")))?;
        let steps = pipe.steps.clone();
        let next = pipe.then.clone();
        for step in &steps {
            match step {
                Step::AppendList { title, detail, .. } => {
                    if !contains(&expect.required, title) && !contains(&expect.optional, title) {
                        expect.required.push(title.clone());
                    }
                    if !contains(&expect.required, detail) && !contains(&expect.optional, detail) {
                        expect.optional.push(detail.clone());
                    }
                }
                Step::Airlock { .. } => expect.probe = true,
                Step::IncStat { .. } | Step::SetGauge { .. } => {}
            }
        }
        for id in next {
            self.gather(&id, seen, expect)?;
        }
        Ok(())
    }

    fn run_chain(&mut self, id: &str, body: &Value, seen: &mut Vec<String>) -> Result<(), Fault> {
        if seen.iter().any(|item| item == id) {
            return Err(Fault::new(422, "pipe", "Pipe cycle."));
        }
        seen.push(id.to_string());
        let pipe = self
            .pipes
            .iter()
            .find(|pipe| pipe.id == id)
            .cloned()
            .ok_or_else(|| Fault::new(404, "missing", format!("No pipe `{id}`.")))?;
        for step in &pipe.steps {
            self.run_step(&pipe.id, step, body)?;
        }
        for next in pipe.then {
            self.run_chain(&next, body, seen)?;
        }
        Ok(())
    }

    fn run_step(&mut self, pipe_id: &str, step: &Step, body: &Value) -> Result<(), Fault> {
        match step {
            Step::AppendList {
                card,
                title,
                detail,
            } => self.append_item(card, &trimmed(body, title), &trimmed(body, detail), "ink"),
            Step::IncStat { card, by } => self.inc_stat(card, *by),
            Step::SetGauge {
                card,
                value,
                caption,
            } => self.set_gauge(card, *value, caption),
            Step::Airlock { log, gauge } => self.run_airlock(pipe_id, log, gauge, body),
        }
    }

    fn run_airlock(
        &mut self,
        pipe_id: &str,
        log: &str,
        gauge: &str,
        body: &Value,
    ) -> Result<(), Fault> {
        let name = trimmed(body, "probe");
        let probe = parse_probe(&name).ok_or_else(|| {
            Fault::new(
                422,
                "schema",
                "Probe must be unlisted, metadata, plain-http, or allowlisted.",
            )
        })?;
        let destination = probe.destination();
        let outbound = obj(vec![
            ("probe", Value::String(probe.as_str().into())),
            ("pipe", Value::String(pipe_id.into())),
        ])
        .to_json()
        .into_bytes();
        match authorize(
            &self.policy,
            EgressRequest {
                pipe: pipe_id,
                method: "POST",
                destination,
                content_type: "application/json",
                body: &outbound,
            },
        ) {
            Ok(permit) => {
                let receipt = dry_dock(permit, &outbound);
                let detail = format!(
                    "Permit issued. Dry dock held {} bytes. Nothing was sent.",
                    receipt.bytes()
                );
                self.append_item(log, "permit", permit_notice(), "sea")?;
                self.set_caption(gauge, &detail)?;
                self.set_seal("PERMIT", permit_notice());
                self.notice = permit_notice().into();
                self.verdict = Some(obj(vec![
                    ("code", Value::String("permit".into())),
                    ("detail", Value::String(permit_notice().into())),
                    ("origin", Value::String(receipt.origin().into())),
                    ("bytes", json_u64(receipt.bytes() as u64)),
                    ("sent", Value::Bool(receipt.sent())),
                ]));
                Ok(())
            }
            Err(denial) => {
                self.append_item(log, denial.code.as_str(), &denial.detail, "deny")?;
                self.set_caption(gauge, &denial.detail)?;
                self.set_seal("DENIED", &denial.detail);
                self.notice = denial.detail.clone();
                self.verdict = Some(obj(vec![
                    ("code", Value::String(denial.code.as_str().into())),
                    ("detail", Value::String(denial.detail.clone())),
                    ("origin", Value::String(destination.into())),
                    ("sent", Value::Bool(false)),
                ]));
                Ok(())
            }
        }
    }

    fn append_item(
        &mut self,
        card_id: &str,
        title: &str,
        detail: &str,
        tone: &str,
    ) -> Result<(), Fault> {
        if !schema::is_tone(tone) {
            return Err(Fault::new(500, "tone", "Tone is not in the closed set."));
        }
        let title = clip(title, 48);
        let detail = clip(detail, 180);
        let index = self
            .index_of(card_id)
            .ok_or_else(|| Fault::new(422, "pipe", format!("`{card_id}` is not on the board.")))?;
        if schema::Kind::parse(
            self.cards[index]
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or(""),
        ) != Some(Kind::List)
        {
            return Err(Fault::new(
                422,
                "pipe",
                format!("`{card_id}` is not a list."),
            ));
        }
        {
            let card = &mut self.cards[index];
            let Some(items) = card
                .get_mut("body")
                .and_then(|body| body.get_mut("items"))
                .and_then(Value::as_array_mut)
            else {
                return Err(Fault::new(
                    422,
                    "pipe",
                    format!("`{card_id}` has no items."),
                ));
            };
            items.push(obj(vec![
                ("title", Value::String(title)),
                ("detail", Value::String(detail)),
                ("tone", Value::String(tone.into())),
            ]));
            while items.len() > LIST_CAP {
                items.remove(0);
            }
        }
        bump_rev(&mut self.cards[index]);
        self.record_card_parts(card_id, &["items"]);
        Ok(())
    }

    fn inc_stat(&mut self, card_id: &str, by: i64) -> Result<(), Fault> {
        let index = self
            .index_of(card_id)
            .ok_or_else(|| Fault::new(422, "pipe", format!("`{card_id}` is not on the board.")))?;
        if schema::Kind::parse(
            self.cards[index]
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or(""),
        ) != Some(Kind::Stat)
        {
            return Err(Fault::new(
                422,
                "pipe",
                format!("`{card_id}` is not a stat."),
            ));
        }
        let current = self.cards[index]
            .get("body")
            .and_then(|body| body.get("value"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let next = current
            .checked_add(by)
            .ok_or_else(|| Fault::new(422, "cap", "That stat cannot go any higher."))?;
        if next > STAT_CAP {
            return Err(Fault::new(422, "cap", "That stat cannot go any higher."));
        }
        if let Some(body) = self.cards[index].get_mut("body") {
            set_field(body, "value", json_i64(next));
        } else {
            return Err(Fault::new(422, "pipe", "Missing body."));
        }
        bump_rev(&mut self.cards[index]);
        self.record_card_parts(card_id, &["value"]);
        Ok(())
    }

    fn set_gauge(&mut self, card_id: &str, value: u8, caption: &str) -> Result<(), Fault> {
        self.write_gauge(card_id, Some(i64::from(value)), caption)
    }

    fn set_caption(&mut self, card_id: &str, caption: &str) -> Result<(), Fault> {
        self.write_gauge(card_id, None, caption)
    }

    fn write_gauge(
        &mut self,
        card_id: &str,
        value: Option<i64>,
        caption: &str,
    ) -> Result<(), Fault> {
        let index = self
            .index_of(card_id)
            .ok_or_else(|| Fault::new(422, "pipe", format!("`{card_id}` is not on the board.")))?;
        if schema::Kind::parse(
            self.cards[index]
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or(""),
        ) != Some(Kind::Gauge)
        {
            return Err(Fault::new(
                422,
                "pipe",
                format!("`{card_id}` is not a gauge."),
            ));
        }
        let caption = clip(caption, 180);
        {
            let Some(body) = self.cards[index].get_mut("body") else {
                return Err(Fault::new(422, "pipe", "Missing body."));
            };
            if let Some(value) = value {
                set_field(body, "value", json_i64(value));
            }
            set_field(body, "caption", Value::String(caption));
        }
        bump_rev(&mut self.cards[index]);
        let fields: &[&str] = if value.is_some() {
            &["value", "caption"]
        } else {
            &["caption"]
        };
        self.record_card_parts(card_id, fields);
        Ok(())
    }

    fn set_seal(&mut self, word: &str, detail: &str) {
        self.seal_word = word.to_string();
        self.seal_detail = detail.to_string();
        self.tape
            .record("/seal/word", &Value::String(self.seal_word.clone()));
        self.tape
            .record("/seal/detail", &Value::String(self.seal_detail.clone()));
    }

    fn record_card_parts(&mut self, card_id: &str, fields: &[&str]) {
        let Some(index) = self.index_of(card_id) else {
            return;
        };
        for field in fields {
            let value = self.cards[index]
                .get("body")
                .and_then(|body| body.get(field))
                .cloned()
                .unwrap_or(Value::Null);
            self.tape
                .record(format!("/cards/{card_id}/body/{field}"), &value);
        }
        let rev = self.cards[index].get("rev").cloned().unwrap_or(Value::Null);
        self.tape.record(format!("/cards/{card_id}/rev"), &rev);
    }

    fn index_of(&self, id: &str) -> Option<usize> {
        self.cards.iter().position(|card| id_of(card) == id)
    }

    fn snapshot(&self) -> Snap {
        Snap {
            cards: self.cards.clone(),
            tape: self.tape.clone(),
            seal_word: self.seal_word.clone(),
            seal_detail: self.seal_detail.clone(),
        }
    }

    fn restore(&mut self, snap: Snap) {
        self.cards = snap.cards;
        self.tape = snap.tape;
        self.seal_word = snap.seal_word;
        self.seal_detail = snap.seal_detail;
        self.verdict = None;
    }

    fn delta_value(&self, since: Option<u64>) -> Value {
        let delta = self.tape.since(since);
        let patches = delta
            .patches
            .iter()
            .map(|patch| {
                obj(vec![
                    ("cursor", json_u64(patch.cursor)),
                    ("path", Value::String(patch.path.clone())),
                    ("preview", Value::String(patch.preview.clone())),
                    ("value", patch.value.clone()),
                ])
            })
            .collect();
        let mut pairs = vec![
            ("epoch", Value::String(self.epoch.clone())),
            ("cursor", json_u64(delta.cursor)),
            ("reset", Value::Bool(delta.reset)),
            ("patches", Value::Array(patches)),
        ];
        if delta.reset {
            pairs.push(("view", render::view(self)));
        }
        obj(pairs)
    }
}

fn surface_of(request: &Request) -> &'static str {
    match request.query_one("surface") {
        Some("pretext") => "pretext",
        _ => "deck",
    }
}

fn admit_request(request: &Request, need: Need<'_>) -> Result<(), Fault> {
    gate::admit(request, need)
        .map(|_| ())
        .map_err(|denied| Fault::new(denied.status, denied.code, denied.detail))
}

fn validate_body(expect: &Expect, body: &Value) -> Result<(), Fault> {
    let Value::Object(pairs) = body else {
        return Err(Fault::new(422, "schema", "A push body is an object."));
    };
    for (key, _) in pairs {
        let known = contains(&expect.required, key)
            || contains(&expect.optional, key)
            || (expect.probe && key == "probe");
        if !known {
            return Err(Fault::new(422, "schema", format!("Unknown field `{key}`.")));
        }
    }
    for key in &expect.required {
        let Some(Value::String(text)) = pairs
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
        else {
            return Err(Fault::new(422, "schema", format!("`{key}` is required.")));
        };
        schema::check_plain(text.trim(), 1, 48)
            .map_err(|detail| Fault::new(422, "schema", format!("`{key}` {detail}.")))?;
    }
    for key in &expect.optional {
        if let Some(value) = pairs
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
        {
            let Value::String(text) = value else {
                return Err(Fault::new(
                    422,
                    "schema",
                    format!("`{key}` must be a string."),
                ));
            };
            schema::check_plain(text.trim(), 0, 80)
                .map_err(|detail| Fault::new(422, "schema", format!("`{key}` {detail}.")))?;
        }
    }
    if expect.probe {
        let Some(Value::String(text)) = pairs
            .iter()
            .find(|(name, _)| name == "probe")
            .map(|(_, value)| value)
        else {
            return Err(Fault::new(422, "schema", "`probe` is required."));
        };
        if parse_probe(text.trim()).is_none() {
            return Err(Fault::new(
                422,
                "schema",
                "Probe must be unlisted, metadata, plain-http, or allowlisted.",
            ));
        }
    }
    Ok(())
}

fn payload(request: &Request) -> Result<Value, Fault> {
    match request.content_type() {
        "application/json" => require_json(request),
        "application/x-www-form-urlencoded" => {
            match request.header("sec-fetch-site") {
                None | Some("same-origin" | "same-site" | "none") => {}
                _ => {
                    return Err(Fault::new(
                        403,
                        "cross-site",
                        "Cross-site form posts are refused.",
                    ));
                }
            }
            let text = std::str::from_utf8(&request.body)
                .map_err(|_| Fault::new(400, "body", "Body is not utf-8."))?;
            let pairs = parse_form(text).map_err(|detail| Fault::new(400, "form", detail))?;
            Ok(Value::Object(
                pairs
                    .into_iter()
                    .map(|(key, value)| (key, Value::String(value)))
                    .collect(),
            ))
        }
        _ => Err(Fault::new(415, "type", "Send JSON or a form body.")),
    }
}

fn require_json(request: &Request) -> Result<Value, Fault> {
    require_intent(request)?;
    if request.content_type() != "application/json" {
        return Err(Fault::new(415, "type", "Send JSON."));
    }
    let text = std::str::from_utf8(&request.body)
        .map_err(|_| Fault::new(400, "body", "Body is not utf-8."))?;
    json::parse(text).map_err(|error| Fault::new(400, "json", http::json_error_message(&error)))
}

fn require_intent(request: &Request) -> Result<(), Fault> {
    if request.header("x-airlock-intent") == Some("1") {
        Ok(())
    } else {
        Err(Fault::new(
            400,
            "intent",
            "JSON writes need the X-Airlock-Intent: 1 header.",
        ))
    }
}

fn merge_body(card: &mut Value, patch: &Value) -> Result<(), Fault> {
    let kind = schema::Kind::parse(card.get("kind").and_then(Value::as_str).unwrap_or(""))
        .ok_or_else(|| Fault::new(422, "schema", "Card kind is not valid."))?;
    let allowed: &[&str] = match kind {
        Kind::Hero => &["lede", "kicker"],
        Kind::Stat => &["label", "value", "unit", "hint"],
        Kind::Note => &["text", "tone"],
        Kind::Gauge => &["label", "value", "caption"],
        Kind::Media => &["caption", "fallback", "src"],
        Kind::List | Kind::Form => {
            return Err(Fault::new(
                422,
                "schema",
                "This kind only accepts a title patch.",
            ));
        }
    };
    let Value::Object(pairs) = patch else {
        return Err(Fault::new(422, "schema", "Body patch is an object."));
    };
    if pairs.is_empty() {
        return Err(Fault::new(422, "schema", "Body patch is empty."));
    }
    for (key, value) in pairs {
        if !allowed.contains(&key.as_str()) {
            return Err(Fault::new(
                422,
                "schema",
                format!("Unknown body field `{key}`."),
            ));
        }
        let body = card
            .get_mut("body")
            .ok_or_else(|| Fault::new(422, "schema", "Missing body."))?;
        set_field(body, key, value.clone());
    }
    Ok(())
}

fn cursor_query(request: &Request) -> Result<Option<u64>, Fault> {
    match request.query_one("cursor") {
        None => Ok(None),
        Some(text)
            if !text.is_empty()
                && text.len() <= 20
                && text.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            text.parse::<u64>()
                .map(Some)
                .map_err(|_| Fault::new(400, "cursor", "Cursor is not a number."))
        }
        Some(_) => Err(Fault::new(400, "cursor", "Cursor is not a number.")),
    }
}

fn bump_rev(card: &mut Value) {
    let rev = card
        .get("rev")
        .and_then(Value::as_i64)
        .unwrap_or(1)
        .saturating_add(1);
    set_field(card, "rev", json_i64(rev));
}

fn set_field(value: &mut Value, key: &str, next: Value) {
    let Value::Object(pairs) = value else { return };
    if let Some((_, slot)) = pairs.iter_mut().find(|(name, _)| name == key) {
        *slot = next;
    } else {
        pairs.push((key.to_string(), next));
    }
}

fn id_of(card: &Value) -> &str {
    card.get("id").and_then(Value::as_str).unwrap_or("")
}

fn trimmed(body: &Value, key: &str) -> String {
    body.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string()
}

fn clip(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

fn contains(items: &[String], key: &str) -> bool {
    items.iter().any(|item| item == key)
}

fn edge(from: &str, to: &str, via: &str) -> Value {
    obj(vec![
        ("from", Value::String(from.into())),
        ("to", Value::String(to.into())),
        ("via", Value::String(via.into())),
    ])
}

fn obj(pairs: Vec<(&str, Value)>) -> Value {
    Value::Object(
        pairs
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    )
}

fn json_i64(value: i64) -> Value {
    Value::Number(Number::Int(value))
}

fn json_u64(value: u64) -> Value {
    i64::try_from(value)
        .map(json_i64)
        .unwrap_or_else(|_| Value::String(value.to_string()))
}
