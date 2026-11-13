use hull::Hull;
use hull::http::Request;
use hull::radio::{self, PNG_MAGIC};

#[test]
fn factory_phase1_three_git_three_gsuite_no_n8n() {
    let world = radio::phase1();
    assert_eq!(world.phase, 1);
    assert_eq!(world.hd, "gsuite.test");
    assert_eq!(world.gsuite.len(), 3);
    assert_eq!(world.git.len(), 3);
    let emails: Vec<_> = world.gsuite.iter().map(|p| p.email).collect();
    assert_eq!(
        emails,
        vec![
            "operator@gsuite.test",
            "agent-radio@gsuite.test",
            "agent-llm@gsuite.test",
        ]
    );
    assert!(world.gsuite.iter().all(|p| p.hd == world.hd));
    assert_eq!(
        world.gsuite.iter().filter(|p| p.may_approve).count(),
        1,
        "only the operator approves"
    );
    assert_eq!(
        world.gsuite.iter().filter(|p| p.may_call_model).count(),
        1,
        "only the llm principal calls the model"
    );
    let repos: Vec<_> = world.git.iter().map(|g| g.repo).collect();
    assert_eq!(repos.len(), 3);
    assert!(repos.windows(2).all(|pair| pair[0] != pair[1]));
    assert!(world.git.iter().all(|g| {
        !g.scopes
            .iter()
            .any(|scope| *scope == "workflow" || *scope == "admin")
    }));
    assert_eq!(
        world.sinks,
        vec!["ledger", "chat-card", "github-issue", "llm-route"]
    );
    assert!(!world.sinks.contains(&"n8n"));
    let plate = world.screenshot();
    assert!(plate.bytes.starts_with(PNG_MAGIC));
    assert!(plate.bytes.len() <= 512_000);
}

#[test]
fn factory_sha256_matches_the_abc_vector() {
    assert_eq!(
        radio::sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let world = radio::phase1();
    let plate = world.screenshot();
    assert_eq!(
        radio::sha256_hex(&plate.bytes),
        "3f8c44dbd2b8da1c89ba2942b4ad10e2d535a69bb4ef579818d749604c7efb82"
    );
    assert_eq!(
        radio::block_id(&radio::sha256_hex(&plate.bytes)),
        "blk_3f8c44dbd2b8da1c"
    );
}

#[test]
fn deck_shows_the_factory_cast() {
    let world = radio::phase1();
    let mut hull = Hull::boot_embedded();
    let page = hull
        .dispatch(&Request::from_parts("GET", "/", "", vec![], vec![]).unwrap())
        .text();
    for principal in &world.gsuite {
        assert!(
            page.contains(principal.email),
            "missing {}",
            principal.email
        );
    }
    for install in &world.git {
        assert!(page.contains(install.id), "missing {}", install.id);
        assert!(page.contains(install.repo), "missing {}", install.repo);
    }
    assert!(page.contains("FR-1 Ingest"));
    assert!(page.contains("FR-2 Git"));
    assert!(page.contains("FR-3 SSO"));
    assert!(page.contains("n8n stays dark"));
    assert!(!page.contains(">n8n<"));
    assert!(page.contains("Chat is the radio"));
}
