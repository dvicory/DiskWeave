use super::*;

fn coverage_fixture(name: &str) -> (PathBuf, App, KnowledgeModel) {
    let (root, app) = fixture(name);
    fs::write(
        root.join("openspec/specs/cap/spec.md"),
        specification(&[(
            "req.cap.one",
            &[],
            "The implementation SHALL preserve one boundary.",
        )]),
    )
    .unwrap();
    fs::create_dir_all(root.join("openspec/specs/documentation-knowledge-architecture")).unwrap();
    fs::write(
        root.join("openspec/specs/documentation-knowledge-architecture/spec.md"),
        specification(&[(
            COVERAGE_REQUIREMENT_ID,
            &[],
            "The repository SHALL evaluate current coverage.",
        )]),
    )
    .unwrap();
    let model = knowledge_model(&app).unwrap();
    (root, app, model)
}

fn evidence_manifest(missing: Option<&str>, artifact_path: &str) -> String {
    let field = |name: &str, value: &str| {
        if missing == Some(name) {
            String::new()
        } else {
            format!("{value}\n")
        }
    };
    format!(
        "schema = \"{VERIFICATION_SCHEMA}\"\n\n[[evidence]]\n{}{}{}{}{}{}{}requirements = [\"req.cap.one\"]\n{}\n",
        field("stable identity", "id = \"evidence.cap-one\""),
        field("executable input", &format!("path = \"{artifact_path}\"")),
        field("evidence class or role", "class = \"evidence-artifact\""),
        field("observed outcome", "claim = \"The bounded check passes.\""),
        field("evidence tier", "tier = \"portable-test\""),
        field(
            "exercised mechanism or invariant",
            "fault_model = [\"boundary violation\"]",
        ),
        field("bounded scope", "scope = [\"one bounded case\"]"),
        field(
            "closest unsupported claim boundary and explicit non-claims",
            "non_claims = [\"No platform claim.\"]",
        ),
    )
}

fn write_complete_evidence(root: &Path) {
    fs::create_dir_all(root.join("verification")).unwrap();
    fs::create_dir_all(root.join("docs/verification")).unwrap();
    fs::write(
        root.join("docs/verification/cap-one.md"),
        "bounded deterministic result\n",
    )
    .unwrap();
    fs::write(
        root.join("verification/manifest.toml"),
        evidence_manifest(None, "docs/verification/cap-one.md"),
    )
    .unwrap();
}

fn disposition(
    requirement: &RequirementObject,
    target: &str,
    trigger: &str,
    digest: &str,
) -> CoverageDisposition {
    CoverageDisposition {
        requirement_id: requirement.semantic_id.clone(),
        target: target.to_owned(),
        kind: "deferred".to_owned(),
        scope: vec![format!("current {target} target")],
        non_claims: vec![format!("No {target} coverage is claimed.")],
        reason: format!("The {target} target has no reviewed current endpoint."),
        review_trigger: ReviewTrigger {
            condition: trigger.to_owned(),
        },
        local_fingerprint: requirement.local_semantic_fingerprint.clone(),
        effective_fingerprint: requirement.effective_semantic_fingerprint.clone(),
        endpoint_set_digest: digest.to_owned(),
    }
}

fn add_disposition(state: &mut ReviewedState, disposition: CoverageDisposition) {
    state
        .coverage_dispositions
        .entry(disposition.requirement_id.clone())
        .or_default()
        .insert(disposition.target.clone(), disposition);
}

fn accept_record(state: &mut ReviewedState, record: &RequirementCoverage) {
    state.coverage_endpoint_digests.insert(
        record.semantic_id.clone(),
        ReviewedTargetDigests {
            implementation: Some(record.implementation.current_endpoint_set_digest.clone()),
            evidence: Some(record.evidence.current_endpoint_set_digest.clone()),
        },
    );
    state
        .outcomes
        .insert(record.semantic_id.clone(), "reviewed".to_owned());
    state.reasons.insert(
        record.semantic_id.clone(),
        "The focused fixture was reviewed.".to_owned(),
    );
}

#[test]
fn coverage_requires_reviewed_meaningful_owner_and_complete_evidence() {
    let (root, app, model) = coverage_fixture("coverage-complete");
    let marker = concat!("/// dwv:", "req req.cap.one\n");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/owner.rs"),
        format!("{marker}pub struct Owner;\n\n{marker}#[test]\nfn test_only() {{}}\n"),
    )
    .unwrap();
    write_complete_evidence(&root);
    let references = scan_references(&app).unwrap();
    let mut state = reviewed_state(&model.objects);
    let before = coverage_records(&app, &model, &state, &references).unwrap();
    let selected = before
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    assert_eq!(selected.implementation.endpoints.len(), 1);
    assert_eq!(selected.evidence.endpoints.len(), 1);
    assert_eq!(selected.aggregate_state, "missing");

    accept_record(&mut state, selected);
    let after = coverage_records(&app, &model, &state, &references).unwrap();
    let selected = after
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    assert_eq!(selected.aggregate_state, "covered");
    assert_eq!(selected.implementation.state, "covered");
    assert_eq!(selected.evidence.state, "covered");

    fs::write(root.join(REVIEWED_PATH), toml::to_string(&state).unwrap()).unwrap();
    let packet = ownership(&app, "req.cap.one".to_owned()).unwrap();
    assert_eq!(packet["coverage"]["aggregate_state"], "covered");
    assert_eq!(
        packet["coverage"]["implementation"]["current_endpoint_set_digest"],
        packet["coverage"]["implementation"]["reviewed_endpoint_set_digest"]
    );
    let exported = export(&app).unwrap();
    assert_eq!(exported["schema"], OBJECTS_SCHEMA);
    assert!(exported["coverage"]["targets"]["covered"].as_u64().unwrap() >= 2);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn evidence_endpoints_preserve_test_model_scenario_and_artifact_classes() {
    let (root, app, model) = coverage_fixture("evidence-classes");
    let entries = [
        ("test", "tests/cap.rs", "test"),
        ("model", "models/quint/cap.qnt", "executable-model"),
        ("scenario", "verification/cap.json", "executable-scenario"),
        ("artifact", "docs/verification/cap.md", "evidence-artifact"),
    ];
    let mut manifest = format!("schema = \"{VERIFICATION_SCHEMA}\"\n");
    for (id, path, class) in entries {
        let artifact = root.join(path);
        fs::create_dir_all(artifact.parent().unwrap()).unwrap();
        fs::write(&artifact, format!("bounded {id}\n")).unwrap();
        manifest.push_str(&format!(
            "\n[[evidence]]\nid = \"evidence.{id}\"\npath = \"{path}\"\nclass = \"{class}\"\nclaim = \"The bounded {id} passes.\"\ntier = \"portable\"\nfault_model = [\"boundary violation\"]\nscope = [\"one bounded case\"]\nrequirements = [\"req.cap.one\"]\nnon_claims = [\"No platform claim.\"]\n"
        ));
    }
    fs::create_dir_all(root.join("verification")).unwrap();
    fs::write(root.join("verification/manifest.toml"), manifest).unwrap();

    let records = coverage_records(
        &app,
        &model,
        &reviewed_state(&model.objects),
        &scan_references(&app).unwrap(),
    )
    .unwrap();
    let classes = records
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap()
        .evidence
        .endpoints
        .iter()
        .map(|endpoint| endpoint.class.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        classes,
        BTreeSet::from([
            "evidence-artifact:portable",
            "executable-model:portable",
            "executable-scenario:portable",
            "test:portable",
        ])
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ownership_command_reports_missing_dispositioned_invalid_and_mixed_targets() {
    let (root, app, model) = coverage_fixture("ownership-target-states");
    let requirement = model
        .objects
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    let references = scan_references(&app).unwrap();
    let mut state = reviewed_state(&model.objects);
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join(REVIEWED_PATH), toml::to_string(&state).unwrap()).unwrap();
    let missing = ownership(&app, "req.cap.one".to_owned()).unwrap();
    assert_eq!(missing["coverage"]["aggregate_state"], "missing");
    assert!(
        missing["coverage"]["implementation"]["diagnostics"][0]["next_action"]
            .as_str()
            .is_some()
    );
    assert!(missing["bounds"]["max_context_bytes"].as_u64().is_some());
    assert!(missing["omitted"]["req.cap.one"].is_object());

    let initial = coverage_records(&app, &model, &state, &references).unwrap();
    let selected = initial
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    accept_record(&mut state, selected);
    add_disposition(
        &mut state,
        disposition(
            requirement,
            "implementation",
            "owner-present",
            &selected.implementation.current_endpoint_set_digest,
        ),
    );
    add_disposition(
        &mut state,
        disposition(
            requirement,
            "evidence",
            "evidence-present",
            &selected.evidence.current_endpoint_set_digest,
        ),
    );
    fs::write(root.join(REVIEWED_PATH), toml::to_string(&state).unwrap()).unwrap();
    let dispositioned = ownership(&app, "req.cap.one".to_owned()).unwrap();
    assert_eq!(
        dispositioned["coverage"]["aggregate_state"],
        "dispositioned"
    );
    assert_eq!(
        dispositioned["coverage"]["evidence"]["disposition"]["target"],
        "evidence"
    );

    let marker = concat!("/// dwv:", "req req.cap.one\n");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/helper.rs"),
        format!("{marker}use std::io;\n"),
    )
    .unwrap();
    let mixed = ownership(&app, "req.cap.one".to_owned()).unwrap();
    assert_eq!(mixed["coverage"]["aggregate_state"], "invalid");
    assert_eq!(mixed["coverage"]["evidence"]["state"], "dispositioned");
    assert_eq!(mixed["coverage"]["implementation"]["state"], "invalid");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn coverage_rejects_unscoped_markers_and_stale_locator_digests() {
    let (root, app, model) = coverage_fixture("coverage-owner-scope");
    let marker = concat!("/// dwv:", "req req.cap.one\n");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/helper.rs"),
        format!("{marker}use std::io;\n"),
    )
    .unwrap();
    let references = scan_references(&app).unwrap();
    let state = reviewed_state(&model.objects);
    let records = coverage_records(&app, &model, &state, &references).unwrap();
    let selected = records
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    assert_eq!(selected.implementation.state, "invalid");
    assert_eq!(
        selected.implementation.diagnostics[0]["gate"],
        "unscoped-implementation-marker"
    );

    fs::write(
        root.join("src/helper.rs"),
        format!("{marker}pub struct Owner;\n"),
    )
    .unwrap();
    let references = scan_references(&app).unwrap();
    let mut state = reviewed_state(&model.objects);
    let records = coverage_records(&app, &model, &state, &references).unwrap();
    let selected = records
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    accept_record(&mut state, selected);
    fs::rename(root.join("src/helper.rs"), root.join("src/moved.rs")).unwrap();
    let moved = coverage_records(&app, &model, &state, &scan_references(&app).unwrap()).unwrap();
    let selected = moved
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    assert_eq!(selected.implementation.state, "invalid");
    assert!(
        selected
            .implementation
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic["gate"] == "stale-endpoint-set-digest" })
    );
    fs::write(root.join(REVIEWED_PATH), toml::to_string(&state).unwrap()).unwrap();
    let stale = ownership(&app, "req.cap.one".to_owned()).unwrap();
    assert_eq!(stale["coverage"]["implementation"]["state"], "invalid");
    assert!(
        stale["coverage"]["implementation"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| diagnostic["gate"] == "stale-endpoint-set-digest")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn evidence_contract_rejects_each_missing_fact_and_uninspectable_artifact() {
    for missing in [
        "stable identity",
        "executable input",
        "evidence class or role",
        "observed outcome",
        "evidence tier",
        "exercised mechanism or invariant",
        "bounded scope",
        "closest unsupported claim boundary and explicit non-claims",
    ] {
        let (root, app, model) = coverage_fixture(&format!("evidence-{missing}"));
        fs::create_dir_all(root.join("verification")).unwrap();
        fs::create_dir_all(root.join("docs/verification")).unwrap();
        fs::write(root.join("docs/verification/cap-one.md"), "result\n").unwrap();
        fs::write(
            root.join("verification/manifest.toml"),
            evidence_manifest(Some(missing), "docs/verification/cap-one.md"),
        )
        .unwrap();
        let state = reviewed_state(&model.objects);
        let records =
            coverage_records(&app, &model, &state, &scan_references(&app).unwrap()).unwrap();
        let selected = records
            .iter()
            .find(|record| record.semantic_id == "req.cap.one")
            .unwrap();
        assert_eq!(selected.evidence.state, "invalid", "{missing}");
        assert!(
            selected.evidence.diagnostics[0]["missing_facts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|fact| fact == missing)
        );
        fs::remove_dir_all(root).unwrap();
    }

    let (root, app, model) = coverage_fixture("evidence-uninspectable");
    fs::create_dir_all(root.join("verification")).unwrap();
    fs::write(
        root.join("verification/manifest.toml"),
        evidence_manifest(None, "docs/verification/missing.md"),
    )
    .unwrap();
    let records = coverage_records(
        &app,
        &model,
        &reviewed_state(&model.objects),
        &scan_references(&app).unwrap(),
    )
    .unwrap();
    let selected = records
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    assert!(
        selected.evidence.diagnostics[0]["missing_facts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|fact| fact == "reproducible replay or inspection")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn target_dispositions_are_independent_triggered_and_conservative() {
    let (root, app, model) = coverage_fixture("coverage-dispositions");
    let references = scan_references(&app).unwrap();
    let requirement = model
        .objects
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    let mut state = reviewed_state(&model.objects);
    let initial = coverage_records(&app, &model, &state, &references).unwrap();
    let selected = initial
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    accept_record(&mut state, selected);
    add_disposition(
        &mut state,
        disposition(
            requirement,
            "implementation",
            "owner-present",
            &selected.implementation.current_endpoint_set_digest,
        ),
    );
    let one_deferred = coverage_records(&app, &model, &state, &references).unwrap();
    let selected = one_deferred
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    assert_eq!(selected.implementation.state, "dispositioned");
    assert_eq!(selected.evidence.state, "missing");
    assert_eq!(selected.aggregate_state, "missing");

    add_disposition(
        &mut state,
        disposition(
            requirement,
            "evidence",
            "evidence-present",
            &selected.evidence.current_endpoint_set_digest,
        ),
    );
    let both_deferred = coverage_records(&app, &model, &state, &references).unwrap();
    let selected = both_deferred
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    assert_eq!(selected.aggregate_state, "dispositioned");

    let marker = concat!("/// dwv:", "req req.cap.one\n");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/owner.rs"),
        format!("{marker}pub struct Owner;\n"),
    )
    .unwrap();
    let triggered =
        coverage_records(&app, &model, &state, &scan_references(&app).unwrap()).unwrap();
    let selected = triggered
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    assert_eq!(selected.aggregate_state, "invalid");
    assert!(
        selected
            .implementation
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic["gate"] == "deferred-trigger-fired" })
    );
    assert_eq!(selected.evidence.state, "dispositioned");

    write_complete_evidence(&root);
    let evidence_triggered =
        coverage_records(&app, &model, &state, &scan_references(&app).unwrap()).unwrap();
    let selected = evidence_triggered
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    assert!(
        selected
            .evidence
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic["gate"] == "deferred-trigger-fired" })
    );

    let disposition = state
        .coverage_dispositions
        .get_mut("req.cap.one")
        .unwrap()
        .get_mut("implementation")
        .unwrap();
    disposition.review_trigger.condition = "fingerprint-changed".to_owned();
    disposition.local_fingerprint = "stale".to_owned();
    let fingerprint_triggered =
        coverage_records(&app, &model, &state, &scan_references(&app).unwrap()).unwrap();
    let selected = fingerprint_triggered
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    assert!(
        selected
            .implementation
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic["gate"] == "deferred-trigger-fired" })
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn deleting_the_last_reviewed_endpoint_is_invalid() {
    let (root, app, model) = coverage_fixture("coverage-deletion");
    let marker = concat!("/// dwv:", "req req.cap.one\n");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/owner.rs"),
        format!("{marker}pub struct Owner;\n"),
    )
    .unwrap();
    write_complete_evidence(&root);
    let references = scan_references(&app).unwrap();
    let mut state = reviewed_state(&model.objects);
    let initial = coverage_records(&app, &model, &state, &references).unwrap();
    let selected = initial
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    accept_record(&mut state, selected);

    fs::remove_file(root.join("src/owner.rs")).unwrap();
    let after_owner_delete =
        coverage_records(&app, &model, &state, &scan_references(&app).unwrap()).unwrap();
    let selected = after_owner_delete
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    assert_eq!(selected.implementation.state, "invalid");
    assert!(
        selected
            .implementation
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic["gate"] == "deleted-endpoint-set" })
    );

    fs::remove_file(root.join("docs/verification/cap-one.md")).unwrap();
    let after_evidence_delete =
        coverage_records(&app, &model, &state, &scan_references(&app).unwrap()).unwrap();
    let selected = after_evidence_delete
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    assert_eq!(selected.evidence.state, "invalid");
    assert!(
        selected
            .evidence
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic["gate"] == "deleted-endpoint-set" })
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn marker_retargeting_at_the_same_locator_is_stale() {
    let (root, app, model) = coverage_fixture("coverage-retarget");
    let marker = concat!("/// dwv:", "req req.cap.one\n");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/owner.rs"),
        format!("{marker}pub struct Owner;\n"),
    )
    .unwrap();
    let references = scan_references(&app).unwrap();
    let mut state = reviewed_state(&model.objects);
    let initial = coverage_records(&app, &model, &state, &references).unwrap();
    let selected = initial
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    let old_identity = selected.implementation.endpoints[0].identity.clone();
    state.coverage_endpoint_digests.insert(
        "req.cap.one".to_owned(),
        ReviewedTargetDigests {
            implementation: Some(selected.implementation.current_endpoint_set_digest.clone()),
            evidence: Some(selected.evidence.current_endpoint_set_digest.clone()),
        },
    );
    state
        .outcomes
        .insert("req.cap.one".to_owned(), "reviewed".to_owned());
    state.reasons.insert(
        "req.cap.one".to_owned(),
        "The focused marker was reviewed.".to_owned(),
    );

    fs::write(
        root.join("src/owner.rs"),
        format!("{marker}pub struct Replacement;\n"),
    )
    .unwrap();
    let retargeted =
        coverage_records(&app, &model, &state, &scan_references(&app).unwrap()).unwrap();
    let selected = retargeted
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    assert_ne!(selected.implementation.endpoints[0].identity, old_identity);
    assert_eq!(selected.implementation.state, "invalid");
    assert!(
        selected
            .implementation
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic["gate"] == "stale-endpoint-set-digest" })
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn resolve_only_currentizes_the_selected_target() {
    let (root, app, model) = coverage_fixture("coverage-target-review");
    let marker = concat!("/// dwv:", "req req.cap.one\n");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/owner.rs"),
        format!("{marker}pub struct Owner;\n"),
    )
    .unwrap();
    write_complete_evidence(&root);
    let references = scan_references(&app).unwrap();
    let mut state = reviewed_state(&model.objects);
    let initial = coverage_records(&app, &model, &state, &references).unwrap();
    let selected = initial
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    accept_record(&mut state, selected);
    let old_evidence_digest = state
        .coverage_endpoint_digests
        .get("req.cap.one")
        .and_then(|digests| digests.evidence.clone())
        .unwrap();
    fs::write(root.join(REVIEWED_PATH), toml::to_string(&state).unwrap()).unwrap();
    fs::write(
        root.join("docs/verification/cap-one.md"),
        "changed result\n",
    )
    .unwrap();

    let result = resolve(
        &app,
        "req.cap.one",
        "implementation",
        "reviewed",
        "Implementation target reviewed.",
    )
    .unwrap();
    assert_eq!(result["target"], "implementation");
    let updated: ReviewedState = read_toml(&root, REVIEWED_PATH).unwrap();
    let digests = updated
        .coverage_endpoint_digests
        .get("req.cap.one")
        .unwrap();
    assert_ne!(digests.implementation, None);
    assert_eq!(digests.evidence, Some(old_evidence_digest));
    let records =
        coverage_records(&app, &model, &updated, &scan_references(&app).unwrap()).unwrap();
    let selected = records
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    assert_eq!(selected.implementation.state, "covered");
    assert_eq!(selected.evidence.state, "invalid");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn export_rejects_unknown_relationships_and_partial_review_state_without_writing() {
    let (root, app, model) = coverage_fixture("coverage-export-validation");
    let state = reviewed_state(&model.objects);
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join(REVIEWED_PATH), toml::to_string(&state).unwrap()).unwrap();
    export(&app).unwrap();
    let before = fs::read(root.join(OBJECTS_PATH)).unwrap();
    let mut fingerprints_only = reviewed_state(&model.objects);
    fingerprints_only.outcomes.clear();
    fingerprints_only.reasons.clear();
    fs::write(
        root.join(REVIEWED_PATH),
        toml::to_string(&fingerprints_only).unwrap(),
    )
    .unwrap();
    let error = export(&app).unwrap_err();
    assert_eq!(error.code, "knowledge_projection_invalid");
    assert_eq!(fs::read(root.join(OBJECTS_PATH)).unwrap(), before);
    fs::write(root.join(REVIEWED_PATH), toml::to_string(&state).unwrap()).unwrap();

    fs::create_dir_all(root.join("verification")).unwrap();
    fs::write(
        root.join("verification/manifest.toml"),
        format!(
            "schema = \"{VERIFICATION_SCHEMA}\"\n\n[[evidence]]\nid = \"evidence.unknown\"\npath = \"docs/verification/unknown.md\"\nclass = \"evidence-artifact\"\nclaim = \"Unknown.\"\ntier = \"portable\"\nfault_model = [\"unknown\"]\nscope = [\"unknown\"]\nrequirements = [\"req.cap.unknown\"]\nnon_claims = [\"No claim.\"]\n"
        ),
    )
    .unwrap();
    let error = export(&app).unwrap_err();
    assert_eq!(error.code, "knowledge_projection_invalid");
    assert_eq!(fs::read(root.join(OBJECTS_PATH)).unwrap(), before);

    let mut partial = state;
    let initial =
        coverage_records(&app, &model, &partial, &scan_references(&app).unwrap()).unwrap();
    let selected = initial
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    accept_record(&mut partial, selected);
    partial.outcomes.remove("req.cap.one");
    partial.reasons.remove("req.cap.one");
    fs::write(root.join(REVIEWED_PATH), toml::to_string(&partial).unwrap()).unwrap();
    let error = export(&app).unwrap_err();
    assert_eq!(error.code, "knowledge_projection_invalid");
    assert_eq!(fs::read(root.join(OBJECTS_PATH)).unwrap(), before);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn export_rejects_malformed_rust_marker_without_salvaging_prefix() {
    let (root, app, model) = coverage_fixture("coverage-malformed-rust-marker");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/owner.rs"),
        concat!("/// dwv:", "req req.cap.oneTYPO\npub struct Owner;\n"),
    )
    .unwrap();
    write_complete_evidence(&root);
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join(REVIEWED_PATH),
        toml::to_string(&reviewed_state(&model.objects)).unwrap(),
    )
    .unwrap();

    let error = export(&app).unwrap_err();
    assert_eq!(error.code, "knowledge_projection_invalid");
    let diagnostics = error.details.unwrap()["diagnostics"]
        .as_array()
        .unwrap()
        .clone();
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic["gate"] == "unknown-reference"
            && diagnostic["semantic_id"] == "invalid requirement marker: req.cap.oneTYPO"
    }));
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn explicit_evidence_symlinks_are_confined_and_retargeted() {
    let (root, app, model) = coverage_fixture("coverage-symlink");
    fs::create_dir_all(root.join("verification")).unwrap();
    fs::create_dir_all(root.join("docs/verification")).unwrap();
    fs::write(root.join("docs/verification/a.md"), "same\n").unwrap();
    fs::write(root.join("docs/verification/b.md"), "same\n").unwrap();
    std::os::unix::fs::symlink("a.md", root.join("docs/verification/link.md")).unwrap();
    fs::write(
        root.join("verification/manifest.toml"),
        evidence_manifest(None, "docs/verification/link.md"),
    )
    .unwrap();
    let references = scan_references(&app).unwrap();
    let mut state = reviewed_state(&model.objects);
    let initial = coverage_records(&app, &model, &state, &references).unwrap();
    let selected = initial
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    assert_eq!(
        selected.evidence.endpoints[0].resolved_path.as_deref(),
        Some("docs/verification/a.md")
    );
    accept_record(&mut state, selected);

    fs::remove_file(root.join("docs/verification/link.md")).unwrap();
    std::os::unix::fs::symlink("b.md", root.join("docs/verification/link.md")).unwrap();
    let retargeted =
        coverage_records(&app, &model, &state, &scan_references(&app).unwrap()).unwrap();
    let selected = retargeted
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    assert_eq!(selected.evidence.state, "invalid");
    assert!(
        selected
            .evidence
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic["gate"] == "stale-endpoint-set-digest" })
    );

    fs::remove_file(root.join("docs/verification/link.md")).unwrap();
    let outside = root.with_extension("outside");
    fs::write(&outside, "outside\n").unwrap();
    std::os::unix::fs::symlink(&outside, root.join("docs/verification/link.md")).unwrap();
    let escaped = coverage_records(
        &app,
        &model,
        &reviewed_state(&model.objects),
        &scan_references(&app).unwrap(),
    )
    .unwrap();
    let selected = escaped
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    assert_eq!(selected.evidence.state, "invalid");
    assert!(
        selected.evidence.diagnostics[0]["missing_facts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|fact| fact == "repository path confinement")
    );
    fs::remove_file(root.join("docs/verification/link.md")).unwrap();
    std::os::unix::fs::symlink("missing.md", root.join("docs/verification/link.md")).unwrap();
    let broken = coverage_records(
        &app,
        &model,
        &reviewed_state(&model.objects),
        &scan_references(&app).unwrap(),
    )
    .unwrap();
    let selected = broken
        .iter()
        .find(|record| record.semantic_id == "req.cap.one")
        .unwrap();
    assert!(
        selected.evidence.diagnostics[0]["missing_facts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|fact| fact == "reproducible replay or inspection")
    );
    fs::remove_file(root.join("docs/verification/link.md")).unwrap();
    std::os::unix::fs::symlink("cycle-b.md", root.join("docs/verification/cycle-a.md")).unwrap();
    std::os::unix::fs::symlink("cycle-a.md", root.join("docs/verification/cycle-b.md")).unwrap();
    assert!(matches!(
        resolve_declared_file(&app, "docs/verification/cycle-a.md").unwrap(),
        DeclaredFile::Invalid(_)
    ));
    fs::remove_file(root.join("docs/verification/cycle-a.md")).unwrap();
    fs::remove_file(root.join("docs/verification/cycle-b.md")).unwrap();
    fs::remove_file(outside).unwrap();
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn export_rejects_non_current_requirement_fingerprints_without_writing() {
    let (root, app, model) = coverage_fixture("coverage-fingerprint-validation");
    let state = reviewed_state(&model.objects);
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join(REVIEWED_PATH), toml::to_string(&state).unwrap()).unwrap();
    export(&app).unwrap();
    let before = fs::read(root.join(OBJECTS_PATH)).unwrap();

    fs::write(
        root.join("openspec/specs/cap/spec.md"),
        specification(&[(
            "req.cap.one",
            &[],
            "The implementation SHALL preserve another boundary.",
        )]),
    )
    .unwrap();
    let error = export(&app).unwrap_err();
    assert_eq!(error.code, "knowledge_projection_invalid");
    assert_eq!(fs::read(root.join(OBJECTS_PATH)).unwrap(), before);
    fs::remove_dir_all(root).unwrap();
}
