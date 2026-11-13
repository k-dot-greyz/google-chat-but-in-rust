use hull::Hull;
use hull::http::Request;

fn hull() -> Hull {
    Hull::boot_embedded()
}

fn json_req(method: &str, path: &str, query: &str, body: &str) -> Request {
    Request::from_parts(
        method,
        path,
        query,
        vec![
            ("content-type".into(), "application/json".into()),
            ("x-airlock-intent".into(), "1".into()),
            ("x-profile".into(), "shell".into()),
        ],
        body.as_bytes().to_vec(),
    )
    .unwrap()
}

#[test]
fn plate_updates_the_pass_the_count_and_the_tape() {
    let mut hull = hull();
    let response = hull.dispatch(&json_req(
        "POST",
        "/api/pipes/ingress.pass/push",
        "",
        r#"{"dish":"omelette","note":"baveuse"}"#,
    ));
    assert_eq!(response.status, 200, "{}", response.text());
    let page = hull.dispatch(&Request::from_parts("GET", "/", "", vec![], vec![]).unwrap());
    let html = page.text();
    assert!(html.contains("omelette"));
    assert!(html.contains(">1<"));
    assert!(html.contains("/cards/card.covers/body/value"));
    let delta = hull.dispatch(
        &Request::from_parts("GET", "/api/sync/delta", "cursor=0", vec![], vec![]).unwrap(),
    );
    let text = delta.text();
    assert!(text.contains("\"reset\":false"));
    assert!(text.contains("/cards/card.pass/body/items"));
    let stale = hull.dispatch(
        &Request::from_parts("GET", "/api/sync/delta", "cursor=99999", vec![], vec![]).unwrap(),
    );
    assert!(stale.text().contains("\"reset\":true"));
}

#[test]
fn visitor_markup_stays_text() {
    let mut hull = hull();
    let response = hull.dispatch(&json_req(
        "POST",
        "/api/pipes/ingress.pass/push",
        "",
        r#"{"dish":"<script>alert(1)</script>","note":"\"quoted\""}"#,
    ));
    assert_eq!(response.status, 200, "{}", response.text());
    let html = hull
        .dispatch(&Request::from_parts("GET", "/", "", vec![], vec![]).unwrap())
        .text();
    assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert_eq!(html.matches("<script").count(), 2);
    assert!(
        html.contains("&quot;quoted&quot;") || html.contains("&#39;") || html.contains("quoted")
    );
}

#[test]
fn airlock_matrix_never_sends() {
    let mut hull = hull();
    for (probe, needle) in [
        ("metadata", "hull"),
        ("plain-http", "scheme"),
        ("unlisted", "allowlist"),
    ] {
        let response = hull.dispatch(&json_req(
            "POST",
            "/api/pipes/egress.probe/push",
            "",
            &format!(r#"{{"probe":"{probe}"}}"#),
        ));
        let text = response.text();
        assert_eq!(response.status, 200, "{probe} {text}");
        assert!(text.contains(needle), "{probe} {text}");
        assert!(text.contains("\"sent\":false"), "{probe} {text}");
        assert!(!text.contains("\"sent\":true"), "{probe} {text}");
    }
    let response = hull.dispatch(&json_req(
        "POST",
        "/api/pipes/egress.probe/push",
        "",
        r#"{"probe":"allowlisted"}"#,
    ));
    let text = response.text();
    assert_eq!(response.status, 200, "{text}");
    assert!(text.contains("\"code\":\"permit\""), "{text}");
    assert!(text.contains("\"sent\":false"), "{text}");
    assert!(text.contains("Nothing was sent"), "{text}");
    let page = hull
        .dispatch(&Request::from_parts("GET", "/", "", vec![], vec![]).unwrap())
        .text();
    assert!(page.contains("PERMIT"));
    assert!(page.contains(">4<"));
}

#[test]
fn gates_around_the_hatch() {
    let mut hull = hull();
    let missing = Request::from_parts(
        "POST",
        "/api/pipes/ingress.pass/push",
        "",
        vec![
            ("content-type".into(), "application/json".into()),
            ("x-profile".into(), "shell".into()),
        ],
        br#"{"dish":"eggs","note":""}"#.to_vec(),
    )
    .unwrap();
    assert_eq!(hull.dispatch(&missing).status, 400);

    let cross = Request::from_parts(
        "POST",
        "/api/pipes/ingress.pass/push",
        "",
        vec![
            (
                "content-type".into(),
                "application/x-www-form-urlencoded".into(),
            ),
            ("sec-fetch-site".into(), "cross-site".into()),
            ("x-profile".into(), "shell".into()),
        ],
        b"dish=eggs&note=".to_vec(),
    )
    .unwrap();
    assert_eq!(hull.dispatch(&cross).status, 403);
    let page = hull
        .dispatch(&Request::from_parts("GET", "/", "", vec![], vec![]).unwrap())
        .text();
    assert!(!page.contains("eggs"));

    let weird = hull.dispatch(&json_req(
        "POST",
        "/api/pipes/ingress.pass/push",
        "",
        r#"{"dish":"eggs","note":"","url":"http://169.254.169.254/"}"#,
    ));
    assert_eq!(weird.status, 422, "{}", weird.text());

    let internal = hull.dispatch(&json_req(
        "POST",
        "/api/pipes/internal.tally/push",
        "",
        "{}",
    ));
    assert_eq!(internal.status, 403, "{}", internal.text());

    let climbed = Request::from_parts("GET", "/assets/%2e%2e/Cargo.toml", "", vec![], vec![]);
    assert!(climbed.is_err());
}

#[test]
fn a_failed_push_rolls_back() {
    let mut hull = hull();
    let patch = hull.dispatch(&json_req(
        "PATCH",
        "/api/cards/card.covers",
        "",
        r#"{"body":{"value":10000}}"#,
    ));
    assert_eq!(patch.status, 200, "{}", patch.text());
    let pushed = hull.dispatch(&json_req(
        "POST",
        "/api/pipes/ingress.pass/push",
        "",
        r#"{"dish":"souffle","note":"too tall"}"#,
    ));
    assert_eq!(pushed.status, 422, "{}", pushed.text());
    let page = hull
        .dispatch(&Request::from_parts("GET", "/", "", vec![], vec![]).unwrap())
        .text();
    assert!(!page.contains("souffle"));
    assert!(page.contains(">10000<"));
}

#[test]
fn night_shift_never_falls_back_to_a_white_canvas() {
    let mut hull = hull();
    let page = hull
        .dispatch(&Request::from_parts("GET", "/", "", vec![], vec![]).unwrap())
        .text();
    assert!(page.contains("content=\"dark\""));
    assert!(page.contains("data-surface=\"deck\""));
    assert!(page.contains("No reel loaded"));
    assert!(page.contains("HULL / NIGHT / PRETEXT"));
    let pretext = hull
        .dispatch(&Request::from_parts("GET", "/", "surface=pretext", vec![], vec![]).unwrap())
        .text();
    assert!(pretext.contains("data-surface=\"pretext\""));
    let css = hull
        .dispatch(&Request::from_parts("GET", "/assets/shell.css", "", vec![], vec![]).unwrap())
        .text()
        .to_ascii_lowercase();
    assert!(css.contains("color-scheme: dark"));
    assert!(!css.contains("color-scheme: light"));
    assert!(!css.contains("background: white"));
    assert!(!css.contains("background:#fff"));
    assert!(!css.contains("background: #fff"));
    assert!(!css.contains("#ffffff"));
}

#[test]
fn a_bad_reload_keeps_the_last_good_bundle() {
    let tick = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!("hull-reload-{tick}-{}", std::process::id()));
    let _keep = TempModules(&root);
    let _ = std::fs::remove_dir_all(&root);
    copy_modules(&root);
    let mut hull = Hull::boot_dir(&root);
    assert!(
        hull.fault().is_none(),
        "{}",
        hull.fault().unwrap_or("boot fault")
    );
    let epoch = hull.epoch().to_string();
    let pushed = hull.dispatch(&json_req(
        "POST",
        "/api/pipes/ingress.pass/push",
        "",
        r#"{"dish":"omelette","note":"held"}"#,
    ));
    assert_eq!(pushed.status, 200, "{}", pushed.text());
    let policy_path = root.join("policy.json");
    let policy = std::fs::read(&policy_path).unwrap();
    std::fs::write(&policy_path, b"{").unwrap();
    hull.reload_dir(&root);
    assert!(hull.fault().is_some());
    assert_eq!(hull.epoch(), epoch);
    let broken = hull
        .dispatch(&Request::from_parts("GET", "/", "", vec![], vec![]).unwrap())
        .text();
    assert!(broken.contains("omelette"), "{broken}");
    assert!(broken.contains("id=\"fault\" class=\"fault\" role=\"alert\">"));
    std::fs::write(&policy_path, policy).unwrap();
    hull.reload_dir(&root);
    assert_eq!(hull.fault(), None);
    assert_eq!(hull.epoch(), epoch);
    let healed = hull
        .dispatch(&Request::from_parts("GET", "/", "", vec![], vec![]).unwrap())
        .text();
    assert!(healed.contains("omelette"), "{healed}");
    assert!(healed.contains("id=\"fault\" class=\"fault\" role=\"alert\" hidden"));
}

#[test]
fn a_trace_line_is_capped_at_the_pipe() {
    let mut hull = hull();
    let over_title = hull.dispatch(&json_req(
        "POST",
        "/api/pipes/ingress.trace/push",
        "",
        &format!(r#"{{"title":"{}","detail":"ok"}}"#, "t".repeat(49)),
    ));
    assert_eq!(over_title.status, 422, "{}", over_title.text());
    let over_detail = hull.dispatch(&json_req(
        "POST",
        "/api/pipes/ingress.trace/push",
        "",
        &format!(r#"{{"title":"client","detail":"{}"}}"#, "d".repeat(81)),
    ));
    assert_eq!(over_detail.status, 422, "{}", over_detail.text());
    let accepted = hull.dispatch(&json_req(
        "POST",
        "/api/pipes/ingress.trace/push",
        "",
        &format!(
            r#"{{"title":"{}","detail":"{}"}}"#,
            "t".repeat(48),
            "d".repeat(80)
        ),
    ));
    assert_eq!(accepted.status, 200, "{}", accepted.text());
}

#[test]
fn the_island_ablates_on_the_night_shift() {
    let mut hull = hull();
    let js = hull
        .dispatch(&Request::from_parts("GET", "/assets/island.js", "", vec![], vec![]).unwrap())
        .text();
    assert!(js.contains("querySelector(\"#pretext pre\")"));
    assert!(js.contains("node.textContent"));
    assert!(js.contains("dataset.surface = \"pretext\""));
    assert!(
        js.contains("Client fault held on the night shift. Copy a snapshot if you want it read.")
    );
    assert!(js.contains("Holding the last good view. The pass did not answer."));
    assert!(js.contains("POLL_CAP_MS = 8000"));
    assert!(js.contains("sameOriginAsset"));
    assert!(!js.contains("setInterval"));
    assert!(!js.contains("http://"));
    assert!(!js.contains("https://"));
    let css = hull
        .dispatch(&Request::from_parts("GET", "/assets/shell.css", "", vec![], vec![]).unwrap())
        .text();
    assert!(css.contains("color: #1a120e"));
    assert!(css.contains("min-height: 44px"));
}

struct TempModules<'a>(&'a std::path::Path);

impl Drop for TempModules<'_> {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.0);
    }
}

fn copy_modules(dest: &std::path::Path) {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("modules");
    for folder in ["cards", "pipes"] {
        let from = src.join(folder);
        let to = dest.join(folder);
        std::fs::create_dir_all(&to).unwrap();
        for entry in std::fs::read_dir(&from).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            std::fs::copy(&path, to.join(entry.file_name())).unwrap();
        }
    }
    std::fs::copy(src.join("policy.json"), dest.join("policy.json")).unwrap();
}

#[test]
fn schema_and_handshake_are_the_contract() {
    let mut hull = hull();
    let schema =
        hull.dispatch(&Request::from_parts("GET", "/api/schema", "", vec![], vec![]).unwrap());
    assert!(schema.text().contains("sysex-delta/1"));
    assert!(schema.text().contains("gauge"));
    let hello = hull
        .dispatch(&Request::from_parts("GET", "/api/sync/handshake", "", vec![], vec![]).unwrap());
    assert!(hello.text().contains("\"protocol\":\"sysex-delta/1\""));
    let health =
        hull.dispatch(&Request::from_parts("GET", "/api/health", "", vec![], vec![]).unwrap());
    assert!(health.text().contains("\"seal\":\"SEALED\""));
}
