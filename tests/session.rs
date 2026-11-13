use hull::Hull;
use hull::http::Request;
use hull::json::{self, Value};

fn req(method: &str, path: &str, query: &str, headers: &[(&str, &str)], body: &str) -> Request {
    let mut all = vec![("content-type".into(), "application/json".into())];
    for (name, value) in headers {
        all.push(((*name).into(), (*value).into()));
    }
    Request::from_parts(method, path, query, all, body.as_bytes().to_vec()).unwrap()
}

fn field(response: &hull::http::Response, key: &str) -> String {
    json::parse(&response.text())
        .unwrap()
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

#[test]
fn a_local_session_queries_ids_and_does_not_fall_back() {
    let mut hull = Hull::boot_embedded();
    let minted = hull.dispatch(&req(
        "POST",
        "/api/sessions",
        "",
        &[("x-profile", "shell")],
        r#"{"channel":"deck"}"#,
    ));
    assert_eq!(minted.status, 201, "{}", minted.text());
    assert!(minted.text().contains("\"retry\":false"));
    let session = field(&minted, "session");
    assert!(session.starts_with("ses_"));
    assert!(!minted.text().contains("lede"));

    let again = hull.dispatch(&req(
        "POST",
        "/api/sessions",
        "",
        &[("x-profile", "shell")],
        r#"{"channel":"deck"}"#,
    ));
    assert_eq!(again.status, 200, "{}", again.text());
    assert_eq!(field(&again, "session"), session);

    let queried = hull.dispatch(&req(
        "QUERY",
        "/api/channels/deck",
        "",
        &[("x-session", session.as_str())],
        r#"{"select":["id"]}"#,
    ));
    assert_eq!(queried.status, 200, "{}", queried.text());
    assert_eq!(queried.code, "ok");
    let body = json::parse(&queried.text()).unwrap();
    let cards = body.get("cards").unwrap();
    let Value::Array(cards) = cards else {
        panic!("cards")
    };
    assert!(!cards.is_empty());
    for card in cards {
        let Value::Object(pairs) = card else {
            panic!("card")
        };
        assert!(pairs.iter().all(|(key, _)| key == "id"));
    }
    assert!(!queried.text().contains("lede"));

    let mismatched = hull.dispatch(&req(
        "QUERY",
        "/api/channels/deck",
        "",
        &[("x-session", session.as_str()), ("x-profile", "operator")],
        r#"{"select":["id"]}"#,
    ));
    assert_eq!(mismatched.status, 401, "{}", mismatched.text());
    assert!(!mismatched.text().contains("card."));
    assert!(mismatched.text().contains("\"retry\":false"));
}

#[test]
fn the_query_string_provider_and_wrong_profile_do_not_widen() {
    let mut hull = Hull::boot_embedded();
    let radio = hull.dispatch(&req(
        "POST",
        "/api/sessions",
        "",
        &[("x-profile", "radio")],
        r#"{"channel":"deck"}"#,
    ));
    assert_eq!(radio.status, 403, "{}", radio.text());

    let google = hull.dispatch(&req(
        "POST",
        "/api/sessions",
        "",
        &[("x-profile", "shell")],
        r#"{"channel":"deck","provider":"google"}"#,
    ));
    assert_eq!(google.status, 501, "{}", google.text());
    assert!(google.text().contains("not used instead"));

    let leaked = hull.dispatch(&req(
        "QUERY",
        "/api/channels/deck",
        "select=id",
        &[("x-session", "ses_0123456789abcdef0123456789abcdef")],
        r#"{"select":["id"]}"#,
    ));
    assert_eq!(leaked.status, 400, "{}", leaked.text());

    let get = hull.dispatch(&req("GET", "/api/channels/deck", "", &[], ""));
    assert_eq!(get.status, 405, "{}", get.text());
    assert!(
        get.headers
            .iter()
            .any(|(name, value)| name == "allow" && value == "QUERY")
    );

    let providers = hull.dispatch(&req("GET", "/api/providers", "", &[], ""));
    assert_eq!(providers.status, 200, "{}", providers.text());
    assert!(providers.text().contains("\"send\":false"));
    assert!(!providers.text().contains("google"));
}

#[test]
fn revoke_and_a_secret_in_the_query_stop_the_request() {
    let mut hull = Hull::boot_embedded();
    let minted = hull.dispatch(&req(
        "POST",
        "/api/sessions",
        "",
        &[("x-profile", "shell")],
        r#"{"channel":"deck"}"#,
    ));
    let session = field(&minted, "session");
    let revoked = hull.dispatch(&req(
        "DELETE",
        "/api/sessions",
        "",
        &[("x-session", session.as_str())],
        "",
    ));
    assert_eq!(revoked.status, 200, "{}", revoked.text());
    let queried = hull.dispatch(&req(
        "QUERY",
        "/api/channels/deck",
        "",
        &[("x-session", session.as_str())],
        r#"{"select":["id"]}"#,
    ));
    assert_eq!(queried.status, 401, "{}", queried.text());

    let write = hull.dispatch(&req(
        "POST",
        "/api/pipes/ingress.pass/push",
        "session=nope",
        &[("x-profile", "shell"), ("x-airlock-intent", "1")],
        r#"{"dish":"eggs","note":""}"#,
    ));
    assert_eq!(write.status, 400, "{}", write.text());
}
