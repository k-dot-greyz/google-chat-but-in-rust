//! Server-rendered shell. Visitor text is escaped. The island never receives HTML.

use crate::embed::{ASSET_CSS, ASSET_JS, ASSET_SVG};
use crate::hull::Hull;
use crate::json::{Number, Value};
use crate::load::Direction;
use crate::schema;

pub fn page(hull: &Hull, banner: Option<&str>, surface: &str) -> String {
    let surface = if surface == "pretext" {
        "pretext"
    } else {
        "deck"
    };
    let mut html = Markup::default();
    html.raw("<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\">");
    html.raw("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">");
    html.raw("<meta name=\"color-scheme\" content=\"dark\">");
    html.raw("<title>Hull — the pass is open</title>");
    html.raw("<link rel=\"icon\" href=\"/assets/mark.svg\" type=\"image/svg+xml\">");
    html.raw("<link rel=\"stylesheet\" href=\"/assets/shell.css?v=3\"></head><body data-epoch=\"");
    html.attr(hull.epoch());
    html.raw("\" data-cursor=\"");
    html.attr(&hull.cursor().to_string());
    html.raw("\" data-surface=\"");
    html.attr(surface);
    html.raw("\"><a class=\"skip\" href=\"#pass\">Skip to the pass</a><div class=\"wrap\">");
    header(&mut html, hull, surface);
    html.raw("<p id=\"status\" role=\"status\">");
    if !hull.notice().is_empty() && hull.notice() != hull.seal_detail() {
        html.text(hull.notice());
    }
    html.raw("</p>");
    if let Some(fault) = hull.fault() {
        html.raw("<p id=\"fault\" class=\"fault\" role=\"alert\">");
        html.text(fault);
        html.raw("</p>");
    } else {
        html.raw("<p id=\"fault\" class=\"fault\" role=\"alert\" hidden></p>");
    }
    if let Some(banner) = banner {
        html.raw("<p class=\"fault\" role=\"alert\">");
        html.text(banner);
        html.raw("</p>");
    }
    html.raw("<div class=\"layout\"><aside aria-label=\"Pipes and airlock\">");
    pipes(&mut html, hull);
    airlock_panel(&mut html, hull);
    trace_panel(&mut html, hull);
    html.raw("</aside><main id=\"pass\"><div class=\"deck\" id=\"deck\">");
    hero(&mut html, hull);
    stats(&mut html, hull);
    gauge(&mut html, hull);
    media_bays(&mut html, hull);
    html.raw("<div class=\"split\">");
    pass_list(&mut html, hull);
    plate_form(&mut html, hull);
    html.raw("</div>");
    doctrine(&mut html, hull);
    schema_block(&mut html);
    html.raw("</div>");
    pretext_block(&mut html, hull);
    html.raw("</main></div>");
    tape(&mut html, hull);
    html.raw("</div><script type=\"application/json\" id=\"hull-view\">");
    html.raw(&view(hull).to_json());
    html.raw("</script><script src=\"/assets/island.js?v=3\" defer></script></body></html>");
    let _ = (ASSET_CSS, ASSET_JS, ASSET_SVG);
    html.0
}

pub fn view(hull: &Hull) -> Value {
    obj(vec![
        ("epoch", Value::String(hull.epoch().into())),
        ("cursor", int(hull.cursor() as i64)),
        ("notice", Value::String(hull.notice().into())),
        (
            "fault",
            match hull.fault() {
                Some(fault) => Value::String(fault.into()),
                None => Value::Null,
            },
        ),
        (
            "seal",
            obj(vec![
                ("word", Value::String(hull.seal_word().into())),
                ("detail", Value::String(hull.seal_detail().into())),
            ]),
        ),
        ("covers", int(stat(hull, "card.covers"))),
        ("verdicts", int(stat(hull, "card.verdicts"))),
        (
            "gauge",
            obj(vec![
                ("value", int(gauge_value(hull))),
                ("caption", Value::String(gauge_caption(hull).into())),
            ]),
        ),
        ("pass", items_value(hull, "card.pass")),
        ("log", items_value(hull, "card.log")),
        ("trace", items_value(hull, "card.trace")),
        ("deck", Value::Array(hull.cards().to_vec())),
        (
            "tape",
            Value::Array(
                hull.tape_recent(8)
                    .into_iter()
                    .map(|patch| {
                        obj(vec![
                            ("cursor", int(patch.cursor as i64)),
                            ("path", Value::String(patch.path.clone())),
                            ("preview", Value::String(patch.preview.clone())),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "pipes",
            Value::Array(
                hull.pipes()
                    .iter()
                    .map(|pipe| {
                        obj(vec![
                            ("id", Value::String(pipe.id.clone())),
                            ("direction", Value::String(pipe.direction.as_str().into())),
                            ("summary", Value::String(pipe.summary())),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

pub fn asset(name: &str) -> Option<(&'static str, String)> {
    let embedded = match name {
        "shell.css" => ("text/css; charset=utf-8", ASSET_CSS),
        "island.js" => ("text/javascript; charset=utf-8", ASSET_JS),
        "mark.svg" => ("image/svg+xml", ASSET_SVG),
        _ => return None,
    };
    Some((embedded.0, read_asset(name, embedded.1)))
}

fn read_asset(name: &str, embedded: &str) -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = std::path::Path::new("assets").join(name);
        if let Ok(text) = std::fs::read_to_string(path)
            && text.len() <= 200_000
        {
            return text;
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = name;
    }
    embedded.to_string()
}

fn header(html: &mut Markup, hull: &Hull, surface: &str) {
    html.raw("<header class=\"mast\"><div><p class=\"mark\">Hull</p><p class=\"sub\">night shift · rust 1.100.0 · zero crates in the hull</p>");
    html.raw("<nav class=\"surfaces\" aria-label=\"Presentation\"><a data-surface-link=\"deck\" href=\"?surface=deck\"");
    if surface == "deck" {
        html.raw(" aria-current=\"page\"");
    }
    html.raw(">Deck</a><a data-surface-link=\"pretext\" href=\"?surface=pretext\"");
    if surface == "pretext" {
        html.raw(" aria-current=\"page\"");
    }
    html.raw(">Pretext</a></nav></div>");
    html.raw("<div class=\"seal-block\"><p id=\"seal-detail\" class=\"seal-caption\">");
    html.text(hull.seal_detail());
    html.raw("</p><div id=\"seal\" class=\"stamp ");
    html.raw(stamp_class(hull.seal_word()));
    html.raw("\" role=\"status\"><span class=\"sr\">Airlock </span><strong id=\"seal-word\">");
    html.text(hull.seal_word());
    html.raw("</strong><small>airlock</small></div></div></header>");
}

fn pipes(html: &mut Markup, hull: &Hull) {
    html.raw("<section class=\"section\"><h2>Pipes</h2><div id=\"pipe-list\">");
    for pipe in hull.pipes() {
        html.raw("<div class=\"pipe\"><span class=\"dir ");
        html.raw(pipe.direction.as_str());
        html.raw("\">");
        html.text(pipe.direction.as_str());
        html.raw("</span><span><span class=\"pipe-id\">");
        html.text(&pipe.id);
        html.raw("</span><span class=\"pipe-steps\">");
        html.text(&pipe.summary());
        html.raw("</span></span></div>");
    }
    html.raw("</div></section>");
}

fn airlock_panel(html: &mut Markup, hull: &Hull) {
    let action = format!("/api/pipes/{}/push", egress_id(hull));
    html.raw("<section class=\"section\"><h2>Airlock</h2><p class=\"aside-copy\">Default deny. Hull class beats the allowlist. A permit is held in the dry dock and nothing is sent.</p>");
    html.raw("<form id=\"probe-form\" class=\"probes\" method=\"post\" action=\"");
    html.attr(&action);
    html.raw("\">");
    for (value, label) in [
        ("unlisted", "Unlisted origin — example.com"),
        ("metadata", "Metadata address — 169.254.169.254"),
        ("plain-http", "Plain HTTP — scheme"),
        ("allowlisted", "Allowlisted dry dock — hooks.hull.invalid"),
    ] {
        html.raw("<button class=\"ghost\" type=\"submit\" name=\"probe\" value=\"");
        html.attr(value);
        html.raw("\">");
        html.text(label);
        html.raw("</button>");
    }
    html.raw("</form><h3>Log</h3><ol id=\"verdict-list\" class=\"tickets\">");
    write_items(
        html,
        hull,
        "card.log",
        "No verdicts yet. The dock is quiet.",
    );
    html.raw("</ol></section>");
}

fn trace_panel(html: &mut Markup, hull: &Hull) {
    html.raw("<section class=\"section\"><h2>Trace</h2><p class=\"aside-copy\">Faults stay on this machine until you copy a snapshot or file a one-line note.</p><div class=\"probes\"><button type=\"button\" class=\"ghost\" id=\"copy-snapshot\">Copy snapshot</button><button type=\"button\" class=\"ghost\" id=\"download-snapshot\">Download bundle</button><button type=\"button\" class=\"ghost\" id=\"file-trace\">File a line on the pass</button></div><p id=\"trace-status\" class=\"hint\" role=\"status\"></p><h3>Filed</h3><ol id=\"trace-filed\" class=\"tickets\">");
    write_items(
        html,
        hull,
        "card.trace",
        "Nothing filed. The bundle stays on this machine.",
    );
    html.raw("</ol><h3>This client</h3><ol id=\"trace-list\" class=\"tickets\"><li class=\"empty\">No client faults.</li></ol><pre id=\"snapshot-dump\" class=\"snapshot-dump\" hidden></pre><p id=\"client-fault\" class=\"fault\" hidden></p></section>");
}

fn media_bays(html: &mut Markup, hull: &Hull) {
    let mut any = false;
    for card in hull.cards() {
        if card.get("kind").and_then(Value::as_str) != Some("media") {
            continue;
        }
        any = true;
        let title = card.get("title").and_then(Value::as_str).unwrap_or("Bay");
        let medium = field(card, "medium").unwrap_or("video");
        let src = field(card, "src").unwrap_or("");
        let caption = field(card, "caption").unwrap_or("");
        let fallback = field(card, "fallback").unwrap_or("The bay stays dark.");
        let playable = schema::safe_media_src(src) && !src.is_empty();
        html.raw("<article class=\"bay\" data-bay=\"");
        html.attr(medium);
        html.raw("\"><h2>");
        html.text(title);
        html.raw("</h2><div class=\"stage\">");
        if playable {
            match medium {
                "audio" => {
                    html.raw("<audio controls preload=\"none\" src=\"");
                    html.attr(src);
                    html.raw("\">");
                    html.text(fallback);
                    html.raw("</audio>");
                }
                "frame" => {
                    html.raw("<iframe title=\"");
                    html.attr(title);
                    html.raw("\" sandbox=\"\" referrerpolicy=\"no-referrer\" src=\"");
                    html.attr(src);
                    html.raw("\"></iframe>");
                }
                _ => {
                    html.raw("<video controls playsinline preload=\"none\" src=\"");
                    html.attr(src);
                    html.raw("\">");
                    html.text(fallback);
                    html.raw("</video>");
                }
            }
            html.raw("<div class=\"bay-fallback\" hidden><p class=\"kicker\">Bay dark</p><p>");
            html.text(fallback);
            html.raw("</p></div>");
        } else {
            html.raw("<div class=\"bay-fallback\"><p class=\"kicker\">Bay dark</p><p>");
            html.text(fallback);
            html.raw("</p></div>");
        }
        html.raw("</div><p class=\"hint\">");
        html.text(caption);
        html.raw("</p></article>");
    }
    if !any {
        html.raw("<article class=\"bay\" data-bay=\"video\"><h2>Bay</h2><div class=\"stage\"><div class=\"bay-fallback\"><p class=\"kicker\">Bay dark</p><p>No reel loaded. The hatch does not turn white.</p></div></div></article>");
    }
}

fn pretext_block(html: &mut Markup, hull: &Hull) {
    html.raw("<section class=\"pretext\" id=\"pretext\"><h2>Pretext</h2><pre>");
    html.text(&pretext_text(hull));
    html.raw("</pre></section>");
}

fn pretext_text(hull: &Hull) -> String {
    let mut out = String::new();
    out.push_str("HULL / NIGHT / PRETEXT\n");
    out.push_str("native text surface. the deck can leave. this stays.\n\n");
    out.push_str("seal ");
    out.push_str(hull.seal_word());
    out.push_str("  ");
    out.push_str(hull.seal_detail());
    out.push_str("\nepoch ");
    out.push_str(hull.epoch());
    out.push_str("\ncursor ");
    out.push_str(&hull.cursor().to_string());
    out.push_str("\n\nCARDS\n");
    for card in hull.cards() {
        let id = card.get("id").and_then(Value::as_str).unwrap_or("");
        let kind = card.get("kind").and_then(Value::as_str).unwrap_or("");
        let title = card.get("title").and_then(Value::as_str).unwrap_or("");
        out.push_str(id);
        out.push(' ');
        out.push_str(kind);
        out.push_str("  ");
        out.push_str(title);
        out.push('\n');
    }
    out.push_str("\nPIPES\n");
    for pipe in hull.pipes() {
        out.push_str(pipe.direction.as_str());
        out.push(' ');
        out.push_str(&pipe.id);
        out.push_str("  ");
        out.push_str(&pipe.summary());
        out.push('\n');
    }
    out.push_str("\nPASS\n");
    let items = list_items(hull, "card.pass");
    if items.is_empty() {
        out.push_str("(clear)\n");
    }
    for (title, detail) in items {
        out.push_str("- ");
        out.push_str(&title);
        if !detail.is_empty() {
            out.push_str(" / ");
            out.push_str(&detail);
        }
        out.push('\n');
    }
    out.push_str("\nTAPE\n");
    let recent = hull.tape_recent(8);
    if recent.is_empty() {
        out.push_str("(handshake only)\n");
    }
    for patch in recent {
        out.push_str(&patch.cursor.to_string());
        out.push(' ');
        out.push_str(&patch.path);
        out.push_str("  ");
        out.push_str(&patch.preview);
        out.push('\n');
    }
    out
}

fn hero(html: &mut Markup, hull: &Hull) {
    let card = hull.card("card.hero");
    let title = card
        .and_then(|card| card.get("title"))
        .and_then(Value::as_str)
        .unwrap_or("Hull");
    let kicker = card
        .and_then(|card| field(card, "kicker"))
        .unwrap_or("Shell template");
    let lede = card
        .and_then(|card| field(card, "lede"))
        .unwrap_or("The pass is open.");
    html.raw("<article class=\"hero\"><p class=\"kicker\">");
    html.text(kicker);
    html.raw("</p><h1>");
    html.text(title);
    html.raw("</h1><p class=\"lede\">");
    html.text(lede);
    html.raw("</p></article>");
}

fn stats(html: &mut Markup, hull: &Hull) {
    html.raw("<div class=\"stats\">");
    stat_card(html, hull, "card.covers", "covers-value");
    stat_card(html, hull, "card.verdicts", "verdicts-value");
    html.raw("</div>");
}

fn stat_card(html: &mut Markup, hull: &Hull, id: &str, value_id: &str) {
    let card = hull.card(id);
    let label = card.and_then(|card| field(card, "label")).unwrap_or(id);
    let unit = card.and_then(|card| field(card, "unit")).unwrap_or("");
    let hint = card.and_then(|card| field(card, "hint")).unwrap_or("");
    let value = stat(hull, id);
    html.raw("<article class=\"stat\"><h2>");
    html.text(label);
    html.raw("</h2><p class=\"num\"><span id=\"");
    html.attr(value_id);
    html.raw("\">");
    html.text(&value.to_string());
    html.raw("</span>");
    if !unit.is_empty() {
        html.raw(" <span class=\"unit\">");
        html.text(unit);
        html.raw("</span>");
    }
    html.raw("</p><p class=\"hint\">");
    html.text(hint);
    html.raw("</p></article>");
}

fn gauge(html: &mut Markup, hull: &Hull) {
    let value = gauge_value(hull);
    let caption = gauge_caption(hull);
    let lit = (value.clamp(0, 100) + 5) / 10;
    let lit = lit.clamp(0, 10);
    html.raw("<article class=\"gauge\"><h2>Hull integrity</h2><div id=\"gauge-cells\" class=\"cells\" role=\"img\" aria-label=\"");
    html.attr(&format!("Hull integrity {value} out of 100"));
    html.raw("\">");
    for index in 0..10 {
        if i64::from(index) < lit {
            html.raw("<span class=\"on\"></span>");
        } else {
            html.raw("<span></span>");
        }
    }
    html.raw("</div><p class=\"hint\" id=\"gauge-caption\">");
    html.text(caption);
    html.raw("</p></article>");
}

fn pass_list(html: &mut Markup, hull: &Hull) {
    let title = hull
        .card("card.pass")
        .and_then(|card| card.get("title"))
        .and_then(Value::as_str)
        .unwrap_or("On the pass");
    html.raw("<article class=\"panel\"><h2>");
    html.text(title);
    html.raw("</h2><ol id=\"pass-list\" class=\"tickets\">");
    write_items(
        html,
        hull,
        "card.pass",
        "The pass is clear. Send something worth plating.",
    );
    html.raw("</ol></article>");
}

fn plate_form(html: &mut Markup, hull: &Hull) {
    html.raw("<article class=\"panel\"><h2>");
    let card = hull.card("card.plate");
    html.text(
        card.and_then(|card| card.get("title"))
            .and_then(Value::as_str)
            .unwrap_or("Plate"),
    );
    html.raw("</h2>");
    let Some(card) = card else {
        html.raw("<p class=\"hint\">The plate card is missing.</p></article>");
        return;
    };
    let pipe = field(card, "pipe").unwrap_or("ingress.pass");
    let submit = field(card, "submit").unwrap_or("Send it through");
    html.raw("<form id=\"plate-form\" method=\"post\" action=\"");
    html.attr(&format!("/api/pipes/{pipe}/push"));
    html.raw("\">");
    if let Some(Value::Array(fields)) = card.get("body").and_then(|body| body.get("fields")) {
        for field in fields {
            let name = field.get("name").and_then(Value::as_str).unwrap_or("field");
            let label = field.get("label").and_then(Value::as_str).unwrap_or(name);
            let max = field
                .get("max")
                .and_then(Value::as_i64)
                .unwrap_or(80)
                .clamp(1, 80);
            let required = matches!(field.get("required"), Some(Value::Bool(true)));
            html.raw("<label for=\"");
            html.attr(name);
            html.raw("\">");
            html.text(label);
            html.raw("</label><input class=\"text-input\" id=\"");
            html.attr(name);
            html.raw("\" name=\"");
            html.attr(name);
            html.raw("\" type=\"text\" maxlength=\"");
            html.attr(&max.to_string());
            html.raw("\" autocomplete=\"off\"");
            if required {
                html.raw(" required");
            }
            html.raw(">");
        }
    }
    html.raw("<button type=\"submit\">");
    html.text(submit);
    html.raw("</button></form></article>");
}

fn doctrine(html: &mut Markup, hull: &Hull) {
    let card = hull.card("card.doctrine");
    let title = card
        .and_then(|card| card.get("title"))
        .and_then(Value::as_str)
        .unwrap_or("Doctrine");
    let text = card.and_then(|card| field(card, "text")).unwrap_or("");
    html.raw("<article class=\"doctrine\"><h2>");
    html.text(title);
    html.raw("</h2><p>");
    html.text(text);
    html.raw("</p></article>");
}

fn schema_block(html: &mut Markup) {
    html.raw("<details class=\"schema\"><summary><h2>API schema</h2></summary><ul class=\"schema-list\">");
    for route in schema::routes() {
        if route.path == "/" {
            continue;
        }
        html.raw("<li><code>");
        html.text(&format!("{} {}", route.method, route.path));
        html.raw("</code><span>");
        html.text(route.summary);
        html.raw("</span></li>");
    }
    html.raw(
        "</ul><p class=\"hint\"><a href=\"/api/schema\">Open the schema document</a></p></details>",
    );
}

fn tape(html: &mut Markup, hull: &Hull) {
    html.raw("<footer class=\"tape\" aria-live=\"polite\"><div class=\"tape-head\"><span>Delta tape</span><span id=\"cursor-readout\">cursor ");
    html.text(&hull.cursor().to_string());
    html.raw(" · epoch ");
    html.text(hull.epoch());
    html.raw("</span></div><ol id=\"tape-rows\">");
    let recent = hull.tape_recent(8);
    if recent.is_empty() {
        html.raw("<li class=\"empty\">No patches yet. The handshake is the whole bird.</li>");
    } else {
        for patch in recent {
            html.raw("<li><span class=\"path\">");
            html.text(&patch.path);
            html.raw("</span><span>");
            html.text(&patch.preview);
            html.raw("</span></li>");
        }
    }
    html.raw("</ol><p class=\"tape-note\">This pass lives in memory. A cold start plates a fresh service.</p></footer>");
}

fn write_items(html: &mut Markup, hull: &Hull, id: &str, empty: &str) {
    let items = list_items(hull, id);
    if items.is_empty() {
        html.raw("<li class=\"empty\">");
        html.text(empty);
        html.raw("</li>");
        return;
    }
    for (title, detail) in items {
        html.raw("<li><span class=\"dish\">");
        html.text(&title);
        html.raw("</span><span class=\"detail\">");
        html.text(&detail);
        html.raw("</span></li>");
    }
}

fn list_items(hull: &Hull, id: &str) -> Vec<(String, String)> {
    let Some(card) = hull.card(id) else {
        return Vec::new();
    };
    let Some(Value::Array(items)) = card.get("body").and_then(|body| body.get("items")) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            Some((
                item.get("title").and_then(Value::as_str)?.to_string(),
                item.get("detail")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            ))
        })
        .collect()
}

fn items_value(hull: &Hull, id: &str) -> Value {
    Value::Array(
        list_items(hull, id)
            .into_iter()
            .map(|(title, detail)| {
                obj(vec![
                    ("title", Value::String(title)),
                    ("detail", Value::String(detail)),
                ])
            })
            .collect(),
    )
}

fn stat(hull: &Hull, id: &str) -> i64 {
    hull.card(id)
        .and_then(|card| card.get("body"))
        .and_then(|body| body.get("value"))
        .and_then(Value::as_i64)
        .unwrap_or(0)
}

fn gauge_value(hull: &Hull) -> i64 {
    stat(hull, "card.pressure").clamp(0, 100)
}

fn gauge_caption(hull: &Hull) -> &str {
    hull.card("card.pressure")
        .and_then(|card| field(card, "caption"))
        .unwrap_or("No egress has left the hull.")
}

fn field<'a>(card: &'a Value, key: &str) -> Option<&'a str> {
    card.get("body")
        .and_then(|body| body.get(key))
        .and_then(Value::as_str)
}

fn egress_id(hull: &Hull) -> &str {
    hull.pipes()
        .iter()
        .find(|pipe| pipe.direction == Direction::Egress)
        .map(|pipe| pipe.id.as_str())
        .unwrap_or("egress.probe")
}

fn stamp_class(word: &str) -> &'static str {
    match word {
        "DENIED" => "denied",
        "PERMIT" => "permit",
        _ => "sealed",
    }
}

fn int(value: i64) -> Value {
    Value::Number(Number::Int(value))
}

fn obj(pairs: Vec<(&str, Value)>) -> Value {
    Value::Object(
        pairs
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    )
}

#[derive(Default)]
struct Markup(String);

impl Markup {
    fn raw(&mut self, text: &str) {
        self.0.push_str(text);
    }
    fn text(&mut self, text: &str) {
        escape(&mut self.0, text);
    }
    fn attr(&mut self, text: &str) {
        escape(&mut self.0, text);
    }
}

fn escape(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
}
