use hull::Hull;
use hull::gate::{self, Need};
use hull::http::Request;

fn write(method: &str, path: &str, query: &str, profile: Option<&str>, body: &str) -> Request {
    let mut headers = vec![
        ("content-type".into(), "application/json".into()),
        ("x-airlock-intent".into(), "1".into()),
    ];
    if let Some(profile) = profile {
        headers.push(("x-profile".into(), profile.into()));
    }
    Request::from_parts(method, path, query, headers, body.as_bytes().to_vec()).unwrap()
}

#[test]
fn profile_table_is_seven_and_closed() {
    let profiles = gate::profiles();
    assert_eq!(profiles.len(), 7);
    let shell = profiles
        .iter()
        .find(|profile| profile.id == "shell")
        .unwrap();
    assert!(shell.verbs.iter().any(|verb| verb == "mutate-cards"));
    assert!(shell.channels == ["deck"]);
    assert!(gate::may_io(shell, "deck", gate::Io::Query));
    assert!(gate::may_io(shell, "deck", gate::Io::Read));
    assert!(gate::may_io(shell, "deck", gate::Io::Edit));
    assert!(!gate::may_io(shell, "other", gate::Io::Read));
    assert!(!shell.verbs.iter().any(|verb| verb == "approve"));
    let operator = profiles.iter().find(|p| p.id == "operator").unwrap();
    assert!(operator.verbs == ["approve"]);
    assert!(operator.git.is_empty());
    let zenos = profiles.iter().find(|p| p.id == "git-zenos").unwrap();
    assert!(gate::may_file(zenos, "inst-zenos", "k-dot-greyz/zenOS"));
    assert!(!gate::may_file(zenos, "inst-hull", "k-dot-greyz/zenOS"));
    assert!(!gate::may_file(
        zenos,
        "inst-zenos",
        "k-dot-greyz/rust-app-core"
    ));
    let hull_git = profiles.iter().find(|p| p.id == "git-hull").unwrap();
    assert!(gate::may_file(
        hull_git,
        "inst-hull",
        "k-dot-greyz/rust-app-core"
    ));
    assert!(!gate::may_file(hull_git, "inst-zenos", "k-dot-greyz/zenOS"));
    assert!(!gate::may_file(operator, "inst-zenos", "k-dot-greyz/zenOS"));
    let llm = profiles.iter().find(|p| p.id == "llm").unwrap();
    assert!(llm.verbs.iter().any(|verb| verb == "complete"));
    assert!(!llm.verbs.iter().any(|verb| verb == "ingest"));
    let radio = profiles.iter().find(|p| p.id == "radio").unwrap();
    assert!(radio.verbs == ["ingest"]);
}

#[test]
fn a_write_without_a_profile_is_refused() {
    let mut hull = Hull::boot_embedded();
    let response = hull.dispatch(&write(
        "POST",
        "/api/pipes/ingress.pass/push",
        "",
        None,
        r#"{"dish":"eggs","note":""}"#,
    ));
    assert_eq!(response.status, 401, "{}", response.text());
    assert!(response.text().contains("\"code\":\"profile\""));
    let page = hull
        .dispatch(&Request::from_parts("GET", "/", "", vec![], vec![]).unwrap())
        .text();
    assert!(!page.contains("eggs"));
}

#[test]
fn a_bearer_header_is_not_a_profile() {
    let mut hull = Hull::boot_embedded();
    let request = Request::from_parts(
        "POST",
        "/api/pipes/ingress.pass/push",
        "",
        vec![
            ("content-type".into(), "application/json".into()),
            ("x-airlock-intent".into(), "1".into()),
            ("authorization".into(), "Bearer shell".into()),
        ],
        br#"{"dish":"eggs","note":""}"#.to_vec(),
    )
    .unwrap();
    assert_eq!(hull.dispatch(&request).status, 401);
}

#[test]
fn a_profile_in_the_query_is_refused() {
    let mut hull = Hull::boot_embedded();
    let response = hull.dispatch(&write(
        "POST",
        "/api/pipes/ingress.pass/push",
        "profile=shell",
        Some("shell"),
        r#"{"dish":"eggs","note":""}"#,
    ));
    assert_eq!(response.status, 400, "{}", response.text());
    assert!(response.text().contains("\"code\":\"query\""));
}

#[test]
fn radio_cannot_push_the_shell_pipe_or_edit_cards() {
    let mut hull = Hull::boot_embedded();
    let push = hull.dispatch(&write(
        "POST",
        "/api/pipes/ingress.pass/push",
        "",
        Some("radio"),
        r#"{"dish":"eggs","note":""}"#,
    ));
    assert_eq!(push.status, 403, "{}", push.text());
    assert!(push.text().contains("\"code\":\"scope\""));
    let patch = hull.dispatch(&write(
        "PATCH",
        "/api/cards/card.hero",
        "",
        Some("operator"),
        r#"{"title":"taken"}"#,
    ));
    assert_eq!(patch.status, 403, "{}", patch.text());
    let page = hull
        .dispatch(&Request::from_parts("GET", "/", "", vec![], vec![]).unwrap())
        .text();
    assert!(!page.contains("taken"));
    assert!(!page.contains("eggs"));
}

#[test]
fn two_profile_headers_are_not_one_profile() {
    let mut hull = Hull::boot_embedded();
    let request = Request::from_parts(
        "POST",
        "/api/pipes/ingress.pass/push",
        "",
        vec![
            ("content-type".into(), "application/json".into()),
            ("x-airlock-intent".into(), "1".into()),
            ("x-profile".into(), "shell".into()),
            ("x-profile".into(), "operator".into()),
        ],
        br#"{"dish":"eggs","note":""}"#.to_vec(),
    )
    .unwrap();
    assert_eq!(hull.dispatch(&request).status, 401);
}

#[test]
fn shell_still_pushes_its_own_pipes() {
    let mut hull = Hull::boot_embedded();
    let response = hull.dispatch(&write(
        "POST",
        "/api/pipes/ingress.pass/push",
        "",
        Some("shell"),
        r#"{"dish":"omelette","note":"held"}"#,
    ));
    assert_eq!(response.status, 200, "{}", response.text());
    let internal = hull.dispatch(&write(
        "POST",
        "/api/pipes/internal.tally/push",
        "",
        Some("shell"),
        "{}",
    ));
    assert_eq!(internal.status, 403, "{}", internal.text());
}

#[test]
fn admit_matches_the_compiled_table() {
    let request = write("POST", "/api/cards", "", Some("shell"), "{}");
    let profile = gate::admit(&request, Need::Cards).unwrap();
    assert_eq!(profile.id, "shell");
    let radio = write("POST", "/api/cards", "", Some("radio"), "{}");
    assert_eq!(gate::admit(&radio, Need::Cards).unwrap_err().status, 403);
    let missing = write("POST", "/api/cards", "", None, "{}");
    assert_eq!(gate::admit(&missing, Need::Cards).unwrap_err().status, 401);
}
