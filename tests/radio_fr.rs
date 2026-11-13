use hull::radio::{self, Approve, Fault, FileIssue, LlmRequest, Status, Via};

fn world() -> radio::World {
    radio::phase1()
}

#[test]
fn fr1_screenshot_is_ingested_not_fetched() {
    let world = world();
    let drop = world.screenshot();
    assert_eq!(
        radio::ingest(&world, &drop),
        Ok(world.expect_ingested(&drop))
    );
    let link = world.link();
    let linked = world.expect_ingested(&link);
    assert!(!linked.fetched, "a bare url is stored, not fetched");
    assert!(!linked.rendered);
    assert_eq!(radio::ingest(&world, &link), Ok(linked));
}

#[test]
fn fr1_prose_approve_does_not_approve() {
    let world = world();
    let poison = world.prose_injection();
    let ingested = world.expect_ingested(&poison);
    assert_ne!(ingested.status, Status::Approved);
    assert_eq!(radio::ingest(&world, &poison), Ok(ingested));
    let slice = world.slice_of(&poison, Status::Ingested);
    assert_eq!(
        radio::approve(
            &world,
            &Approve {
                by: "operator@gsuite.test",
                via: Via::Prose,
                block_id: slice.block_id,
            },
        ),
        Err(Fault::ProseCannotApprove)
    );
}

#[test]
fn fr1_operator_button_approves() {
    let world = world();
    let drop = world.screenshot();
    let slice = world.slice_of(&drop, Status::Ingested);
    assert_eq!(
        radio::approve(
            &world,
            &Approve {
                by: "operator@gsuite.test",
                via: Via::Button,
                block_id: slice.block_id.clone(),
            },
        ),
        Ok(world.expect_approved(&drop))
    );
    assert_eq!(
        radio::approve(
            &world,
            &Approve {
                by: "agent-radio@gsuite.test",
                via: Via::Button,
                block_id: slice.block_id,
            },
        ),
        Err(Fault::Forbidden)
    );
}

#[test]
fn fr2_issue_stays_on_the_named_repo() {
    let world = world();
    let slice = world.slice_of(&world.screenshot(), Status::Approved);
    assert_eq!(
        radio::file_issue(
            &world,
            &FileIssue {
                installation: "inst-zenos",
                repo: "k-dot-greyz/zenOS",
                block_id: slice.block_id.clone(),
            },
        ),
        Ok(world.expect_issue(&slice))
    );
}

#[test]
fn fr2_other_installation_is_isolated() {
    let world = world();
    let slice = world.slice_of(&world.screenshot(), Status::Approved);
    assert_eq!(
        radio::file_issue(
            &world,
            &FileIssue {
                installation: "inst-hull",
                repo: "k-dot-greyz/zenOS",
                block_id: slice.block_id,
            },
        ),
        Err(Fault::Isolation)
    );
}

#[test]
fn fr3_missing_session_is_401() {
    let world = world();
    let slice = world.slice_of(&world.screenshot(), Status::Approved);
    assert_eq!(
        radio::llm(
            &world,
            &LlmRequest {
                authorization: None,
                hd: world.hd,
                block_id: slice.block_id,
                block_status: slice.status,
            },
        ),
        Ok(radio::LlmResponse {
            status: 401,
            model_called: false,
            outbound_has_raw_bytes: false,
            outbound_sha: None,
        })
    );
}

#[test]
fn fr3_radio_principal_is_403() {
    let world = world();
    let slice = world.slice_of(&world.screenshot(), Status::Approved);
    assert_eq!(
        radio::llm(
            &world,
            &LlmRequest {
                authorization: Some("tok-radio"),
                hd: "other.test",
                block_id: slice.block_id,
                block_status: slice.status,
            },
        ),
        Ok(radio::LlmResponse {
            status: 403,
            model_called: false,
            outbound_has_raw_bytes: false,
            outbound_sha: None,
        })
    );
}

#[test]
fn fr3_unapproved_block_does_not_call_the_model() {
    let world = world();
    let slice = world.slice_of(&world.screenshot(), Status::Ingested);
    assert_eq!(
        radio::llm(
            &world,
            &LlmRequest {
                authorization: Some("tok-llm"),
                hd: world.hd,
                block_id: slice.block_id,
                block_status: slice.status,
            },
        ),
        Ok(radio::LlmResponse {
            status: 409,
            model_called: false,
            outbound_has_raw_bytes: false,
            outbound_sha: None,
        })
    );
}

#[test]
fn fr3_approved_slice_is_the_only_outbound() {
    let world = world();
    let slice = world.slice_of(&world.screenshot(), Status::Approved);
    assert_eq!(
        radio::llm(
            &world,
            &LlmRequest {
                authorization: Some("tok-llm"),
                hd: world.hd,
                block_id: slice.block_id.clone(),
                block_status: slice.status,
            },
        ),
        Ok(world.expect_llm_ok(&slice))
    );
}
