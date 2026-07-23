use super::*;
use crate::service_keyset_cursor::{decode_cursor_for_method, encode_cursor};
use crate::service_local_session_io::LocalSessionReadLimits;
use crate::service_local_sessions::local_session_row_id;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ORDERED_SESSION_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct OrderedSessionFixture {
    fixture: PathBuf,
    root: PathBuf,
    host: ServiceHost,
}

impl OrderedSessionFixture {
    fn new(test_name: &str, rows: &[(&str, i64)]) -> Self {
        use std::time::{Duration, UNIX_EPOCH};

        let fixture = env::temp_dir().join(format!(
            "skills-copilot-session-order-{test_name}-{}-{}",
            std::process::id(),
            NEXT_ORDERED_SESSION_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let root = fixture.join("sessions");
        fs::create_dir_all(&root).expect("create ordered session fixture");
        for (title, modified_at) in rows {
            let path = root.join(format!("{}.jsonl", title.to_ascii_lowercase()));
            fs::write(
                &path,
                json!({"type": "session", "title": title}).to_string(),
            )
            .expect("write ordered session");
            fs::File::open(&path)
                .expect("open ordered session")
                .set_times(
                    fs::FileTimes::new()
                        .set_modified(UNIX_EPOCH + Duration::from_millis(*modified_at as u64)),
                )
                .expect("set ordered session modification time");
        }
        let host = ServiceHost {
            app_data_dir: fixture.join("app-data"),
            adapter_ctx: AdapterContext {
                user_home: fixture.join("home"),
                project_root: None,
                project_cwd: None,
                extra_roots: Vec::new(),
            },
        };
        Self {
            fixture,
            root,
            host,
        }
    }

    fn request(&self, extra_params: Value) -> ServiceResponse {
        let mut params = json!({
            "agent": "codex",
            "authorized_roots": [self.root.to_string_lossy()],
            "auto_discover": false,
            "limit": 100,
            "max_files": 100
        });
        params
            .as_object_mut()
            .expect("object params")
            .extend(extra_params.as_object().expect("extra object").clone());
        self.host.handle(ServiceRequest {
            id: Some("ordered-session-preview".to_string()),
            method: "session.previewLocalSessions".to_string(),
            params,
        })
    }
}

impl Drop for OrderedSessionFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.fixture);
    }
}

fn ordered_result(fixture: &OrderedSessionFixture, params: Value) -> Value {
    let response = fixture.request(params);
    assert!(response.ok, "{:?}", response.error);
    response.result.expect("ordered session preview result")
}

fn ordered_values(result: &Value, field: &str) -> Vec<Value> {
    result["session_rows"]
        .as_array()
        .expect("session rows")
        .iter()
        .map(|row| row[field].clone())
        .collect()
}

fn values(expected: Value) -> Vec<Value> {
    expected.as_array().expect("expected values").clone()
}

fn three_ordered_sessions() -> OrderedSessionFixture {
    OrderedSessionFixture::new("three", &[("Alpha", 100), ("Bravo", 300), ("Charlie", 200)])
}

#[test]
fn preview_sorts_title_ascending_and_descending() {
    let fixture = three_ordered_sessions();
    let asc = ordered_result(&fixture, json!({"sort": "title"}));
    let desc = ordered_result(&fixture, json!({"sort": "title", "direction": "desc"}));
    assert_eq!(
        ordered_values(&asc, "title"),
        values(json!(["Alpha", "Bravo", "Charlie"]))
    );
    assert_eq!(
        ordered_values(&desc, "title"),
        values(json!(["Charlie", "Bravo", "Alpha"]))
    );
}

#[test]
fn preview_sorts_modified_at_ascending_and_descending() {
    let fixture = three_ordered_sessions();
    let asc = ordered_result(&fixture, json!({"sort": "modified_at", "direction": "asc"}));
    let desc = ordered_result(&fixture, json!({"sort": "recent"}));
    assert_eq!(
        ordered_values(&asc, "modified_at"),
        values(json!([100, 200, 300]))
    );
    assert_eq!(
        ordered_values(&desc, "modified_at"),
        values(json!([300, 200, 100]))
    );
}

#[test]
fn pagination_is_applied_after_server_side_sort() {
    let fixture = three_ordered_sessions();
    let result = ordered_result(
        &fixture,
        json!({"sort": "title", "direction": "asc", "offset": 1, "limit": 1}),
    );
    assert_eq!(ordered_values(&result, "title"), values(json!(["Bravo"])));
}

#[test]
fn consecutive_pages_equal_unpaged_order_without_duplicates() {
    let fixture = three_ordered_sessions();
    let unpaged = ordered_result(&fixture, json!({"sort": "title", "limit": 100}));
    let first = ordered_result(&fixture, json!({"sort": "title", "limit": 2}));
    let second = ordered_result(&fixture, json!({"sort": "title", "limit": 2, "offset": 2}));
    let expected = ordered_values(&unpaged, "id");
    let actual = ordered_values(&first, "id")
        .into_iter()
        .chain(ordered_values(&second, "id"))
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    assert_eq!(
        actual
            .iter()
            .filter_map(Value::as_str)
            .collect::<BTreeSet<_>>()
            .len(),
        actual.len()
    );
}

#[test]
fn max_files_selects_newest_candidates() {
    let fixture = OrderedSessionFixture::new(
        "max-files",
        &[
            ("One", 100),
            ("Two", 200),
            ("Three", 300),
            ("Four", 400),
            ("Five", 500),
        ],
    );
    let result = ordered_result(&fixture, json!({"max_files": 2}));
    assert_eq!(
        ordered_values(&result, "modified_at"),
        values(json!([500, 400]))
    );
    assert_eq!(result["total_candidate_count"], json!(5));
    assert!(result["gap_notes"]
        .as_array()
        .is_some_and(|notes| notes.iter().any(|note| note
            .as_str()
            .is_some_and(|note| note.contains("candidate set was truncated")))));
}

#[test]
fn invalid_sort_or_direction_is_rejected() {
    let fixture = three_ordered_sessions();
    for params in [json!({"sort": "size"}), json!({"direction": "sideways"})] {
        let response = fixture.request(params);
        assert_eq!(
            response.error.map(|error| error.code),
            Some("invalid_request".to_string())
        );
    }
}

#[test]
fn keyset_paging_requires_explicit_opt_in_on_the_first_page() {
    let fixture = OrderedSessionFixture::new("keyset-opt-in", &[("Alpha", 100), ("Bravo", 200)]);
    let legacy = fixture.host.handle(ServiceRequest {
        id: Some("legacy-summary".to_string()),
        method: "session.previewLocalSessions".to_string(),
        params: json!({
            "agent": "codex",
            "authorized_roots": [fixture.root.to_string_lossy()],
            "auto_discover": false,
            "scope": "all",
            "include_content_items": false,
            "limit": 1
        }),
    });
    assert!(legacy.ok, "{:?}", legacy.error);
    let legacy = legacy.result.expect("legacy summary result");
    assert_eq!(legacy["next_cursor"], Value::Null);
    assert_eq!(legacy["source_revision"], Value::Null);
    assert_eq!(legacy["next_offset"], json!(1));

    let keyset = fixture.host.handle(ServiceRequest {
        id: Some("keyset-summary".to_string()),
        method: "session.previewLocalSessions".to_string(),
        params: json!({
            "agent": "codex",
            "authorized_roots": [fixture.root.to_string_lossy()],
            "auto_discover": false,
            "scope": "all",
            "include_content_items": false,
            "paging_mode": "keyset",
            "limit": 1
        }),
    });
    assert!(keyset.ok, "{:?}", keyset.error);
    let keyset = keyset.result.expect("keyset summary result");
    assert!(keyset["next_cursor"].as_str().is_some());
    assert!(keyset["source_revision"].as_str().is_some());
    assert_eq!(keyset["next_offset"], Value::Null);
}

#[test]
fn keyset_page_keeps_every_distinct_skill_even_when_session_limit_is_one() {
    let fixture = OrderedSessionFixture::new("keyset-skill-usage", &[("Alpha", 100)]);
    fs::write(
        fixture.root.join("alpha.jsonl"),
        [
            json!({"type": "user", "role": "user", "text": "skill:alpha-skill"}),
            json!({"type": "assistant", "role": "assistant", "text": "skill:beta-skill"}),
        ]
        .into_iter()
        .map(|row| row.to_string())
        .collect::<Vec<_>>()
        .join("\n"),
    )
    .expect("write session with two distinct skills");
    fs::create_dir_all(&fixture.host.app_data_dir).expect("create catalog directory");
    let catalog = Catalog::open(&fixture.host.catalog_path()).expect("open catalog");
    catalog.init().expect("initialize catalog");
    for skill_name in ["alpha-skill", "beta-skill"] {
        let skill_path = fixture.root.join(format!("{skill_name}/SKILL.md"));
        catalog
            .upsert_skill_instance(&SkillInstance {
                id: format!("{skill_name}-id"),
                agent: AgentId::Codex,
                scope: Scope::AgentGlobal,
                project_root: None,
                path: skill_path.clone(),
                display_path: skill_path,
                definition_id: format!("{skill_name}-definition"),
                name: skill_name.to_string(),
                display_name: skill_name.to_string(),
                description: "Keyset skill aggregation fixture.".to_string(),
                version: None,
                state: SkillState::Loaded,
                enabled: true,
                frontmatter_raw: format!("name: {skill_name}\ndescription: fixture\n"),
                body: "Fixture body.".to_string(),
                scripts: Vec::new(),
                permissions: PermissionRequest::default(),
                fingerprint: format!("{skill_name}-fingerprint"),
                mtime: 1,
                first_seen: 1,
                last_seen: 1,
            })
            .expect("seed catalog skill");
    }
    drop(catalog);

    let page = ordered_result(
        &fixture,
        json!({
            "paging_mode": "keyset",
            "max_files": null,
            "include_content_items": false,
            "limit": 1
        }),
    );
    let skill_ids = page["skill_usage_rows"]
        .as_array()
        .expect("skill usage rows")
        .iter()
        .filter_map(|row| row["skill_id"].as_str())
        .collect::<HashSet<_>>();
    assert_eq!(skill_ids.len(), 2, "{page}");
    assert!(skill_ids.contains("alpha-skill-id"), "{page}");
    assert!(skill_ids.contains("beta-skill-id"), "{page}");
}

#[test]
fn keyset_pages_continue_past_legacy_max_files_without_duplicates() {
    let rows = (0..1_205)
        .map(|index| (format!("Session {index:04}"), 10_000 + index as i64))
        .collect::<Vec<_>>();
    let borrowed = rows
        .iter()
        .map(|(title, modified_at)| (title.as_str(), *modified_at))
        .collect::<Vec<_>>();
    let fixture = OrderedSessionFixture::new("keyset-1205", &borrowed);
    let mut expected_paths = fs::read_dir(&fixture.root)
        .expect("read keyset fixture")
        .map(|entry| entry.expect("read keyset entry").path())
        .collect::<Vec<_>>();
    expected_paths.sort_by_key(|path| {
        std::cmp::Reverse(
            fs::metadata(path)
                .expect("keyset metadata")
                .modified()
                .expect("keyset modified time"),
        )
    });
    let unpaged_expected_ids = expected_paths
        .iter()
        .map(|path| {
            local_session_row_id(
                &path
                    .canonicalize()
                    .expect("canonical expected session path"),
            )
        })
        .collect::<Vec<_>>();

    let mut all_ids = Vec::new();
    let mut cursor = None;
    let mut source_revision = None;
    let final_page = loop {
        let page = ordered_result(
            &fixture,
            json!({
                "limit": 100,
                "max_files": null,
                "include_content_items": false,
                "paging_mode": "keyset",
                "cursor": cursor,
                "source_revision": source_revision,
            }),
        );
        all_ids.extend(
            ordered_values(&page, "id")
                .into_iter()
                .map(|id| id.as_str().expect("session id").to_string()),
        );
        if !page["has_more"].as_bool().expect("has_more") {
            break page;
        }
        cursor = page["next_cursor"].as_str().map(str::to_string);
        source_revision = page["source_revision"].as_str().map(str::to_string);
        assert!(
            cursor.is_some(),
            "enumerable continuation must expose a cursor"
        );
        assert!(
            source_revision.is_some(),
            "continuation must bind a revision"
        );
    };

    assert_eq!(all_ids.len(), 1_205);
    assert_eq!(all_ids.iter().collect::<HashSet<_>>().len(), 1_205);
    assert_eq!(
        all_ids
            .iter()
            .zip(&unpaged_expected_ids)
            .position(|(left, right)| left != right),
        None,
        "paged order must equal independently sorted metadata order"
    );
    assert!(!final_page["has_more"].as_bool().expect("final has_more"));
    assert_eq!(final_page["incomplete_reason"], Value::Null);
}

#[test]
fn keyset_continuation_allows_an_already_processed_active_session_to_grow() {
    let fixture = OrderedSessionFixture::new(
        "keyset-active-growth",
        &[("Alpha", 100), ("Bravo", 300), ("Charlie", 200)],
    );
    let first = ordered_result(
        &fixture,
        json!({"limit": 1, "max_files": null, "include_content_items": false, "paging_mode": "keyset"}),
    );
    fs::write(
        fixture.root.join("bravo.jsonl"),
        format!(
            "{}\n{}\n",
            json!({"type": "session", "title": "Bravo"}),
            json!({"type": "user", "role": "user", "text": "active growth"})
        ),
    )
    .expect("grow already processed active candidate");

    let response = fixture.request(json!({
        "limit": 1,
        "max_files": null,
        "include_content_items": false,
        "paging_mode": "keyset",
        "cursor": first["next_cursor"],
        "source_revision": first["source_revision"],
    }));
    assert!(response.ok, "{:?}", response.error);
    assert_eq!(
        response.result.expect("active growth continuation")["session_rows"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
}

#[test]
fn keyset_continuation_rejects_unprocessed_candidate_moving_before_cursor() {
    let fixture = OrderedSessionFixture::new(
        "keyset-source-change",
        &[("Alpha", 100), ("Bravo", 300), ("Charlie", 200)],
    );
    let first = ordered_result(
        &fixture,
        json!({"limit": 1, "max_files": null, "include_content_items": false, "paging_mode": "keyset"}),
    );
    fs::write(
        fixture.root.join("alpha.jsonl"),
        json!({"type": "session", "title": "Alpha changed"}).to_string(),
    )
    .expect("mutate candidate after first page");

    let response = fixture.request(json!({
        "limit": 1,
        "max_files": null,
        "include_content_items": false,
        "paging_mode": "keyset",
        "cursor": first["next_cursor"],
        "source_revision": first["source_revision"],
    }));
    assert_eq!(
        response.error.map(|error| error.code),
        Some("source_changed".to_string())
    );
}

#[test]
fn keyset_continuation_rejects_candidate_membership_change() {
    let fixture = OrderedSessionFixture::new(
        "keyset-membership-change",
        &[("Alpha", 300), ("Bravo", 200), ("Charlie", 100)],
    );
    let first = ordered_result(
        &fixture,
        json!({"limit": 1, "max_files": null, "include_content_items": false, "paging_mode": "keyset"}),
    );
    fs::write(
        fixture.root.join("delta.jsonl"),
        json!({"type": "session", "title": "Delta"}).to_string(),
    )
    .expect("add candidate after first page");

    let response = fixture.request(json!({
        "limit": 1,
        "max_files": null,
        "include_content_items": false,
        "paging_mode": "keyset",
        "cursor": first["next_cursor"],
        "source_revision": first["source_revision"],
    }));
    assert_eq!(
        response.error.map(|error| error.code),
        Some("source_changed".to_string())
    );
}

#[test]
fn keyset_continuation_rejects_missing_or_impossible_accepted_count() {
    let fixture = OrderedSessionFixture::new(
        "keyset-accepted-count",
        &[("Alpha", 300), ("Bravo", 200), ("Charlie", 100)],
    );
    let first = ordered_result(
        &fixture,
        json!({
            "paging_mode": "keyset",
            "max_files": null,
            "include_content_items": false,
            "limit": 1
        }),
    );
    let encoded = first["next_cursor"].as_str().expect("first cursor");
    let revision = first["source_revision"].clone();
    let cursor = decode_cursor_for_method(encoded, "session.previewLocalSessions")
        .expect("decode first cursor");

    for accepted_count in [None, Some(2)] {
        let mut tampered = cursor.clone();
        tampered.accepted_count = accepted_count;
        let response = fixture.request(json!({
            "paging_mode": "keyset",
            "max_files": null,
            "include_content_items": false,
            "limit": 1,
            "cursor": encode_cursor(&tampered).expect("encode tampered cursor"),
            "source_revision": revision
        }));
        assert_eq!(
            response.error.map(|error| error.code),
            Some("invalid_request".to_string()),
            "accepted_count={accepted_count:?} must be rejected"
        );
    }
}

#[test]
fn keyset_continuations_reject_legacy_and_detail_fields() {
    let fixture = OrderedSessionFixture::new(
        "keyset-field-rejection",
        &[("Alpha", 100), ("Bravo", 200), ("Charlie", 300)],
    );
    let first = ordered_result(
        &fixture,
        json!({
            "limit": 1,
            "max_files": null,
            "include_content_items": false,
            "paging_mode": "keyset"
        }),
    );
    let cursor = first["next_cursor"].clone();
    let revision = first["source_revision"].clone();
    for incompatible in [
        json!({"session_id": "local-session-forbidden"}),
        json!({"offset": 1}),
        json!({"max_files": 3}),
        json!({"include_content_items": true}),
    ] {
        let mut params = json!({
            "limit": 1,
            "max_files": null,
            "include_content_items": false,
            "paging_mode": "keyset",
            "cursor": cursor,
            "source_revision": revision,
        });
        params
            .as_object_mut()
            .expect("keyset request object")
            .extend(
                incompatible
                    .as_object()
                    .expect("incompatible object")
                    .clone(),
            );
        let response = fixture.request(params);
        assert_eq!(
            response.error.map(|error| error.code),
            Some("invalid_request".to_string())
        );
    }
    for changed_query in [
        json!({"agent": "pi"}),
        json!({"authorized_roots": [fixture.fixture.to_string_lossy()]}),
        json!({"scope": "project"}),
        json!({"search": "alpha"}),
        json!({"sort": "title"}),
        json!({"direction": "asc"}),
    ] {
        let mut params = json!({
            "limit": 1,
            "max_files": null,
            "include_content_items": false,
            "paging_mode": "keyset",
            "cursor": cursor,
            "source_revision": revision,
        });
        params.as_object_mut().expect("keyset query object").extend(
            changed_query
                .as_object()
                .expect("changed query object")
                .clone(),
        );
        let response = fixture.request(params);
        assert_eq!(
            response.error.map(|error| error.code),
            Some("invalid_request".to_string())
        );
    }
}

#[test]
fn keyset_rejects_legacy_fields_before_an_empty_inventory_return() {
    let fixture = OrderedSessionFixture::new("empty-keyset-validation", &[]);
    let response = fixture.host.handle(ServiceRequest {
        id: Some("empty-keyset-validation".to_string()),
        method: "session.previewLocalSessions".to_string(),
        params: json!({
            "authorized_roots": [],
            "auto_discover": false,
            "scope": "all",
            "include_content_items": false,
            "paging_mode": "keyset",
            "offset": 0,
            "limit": 1
        }),
    });
    assert_eq!(
        response.error.map(|error| error.code),
        Some("invalid_request".to_string())
    );
}

#[test]
fn overlapping_roots_contribute_one_unique_keyset_row() {
    let fixture = OrderedSessionFixture::new("overlapping-roots", &[("Only", 100)]);
    let response = fixture.host.handle(ServiceRequest {
        id: Some("overlapping-roots".to_string()),
        method: "session.previewLocalSessions".to_string(),
        params: json!({
            "agent": "codex",
            "authorized_roots": [fixture.fixture.to_string_lossy(), fixture.root.to_string_lossy()],
            "auto_discover": false,
            "scope": "all",
            "include_content_items": false,
            "paging_mode": "keyset",
            "limit": 100
        }),
    });
    assert!(response.ok, "{:?}", response.error);
    let page = response.result.expect("overlapping root page");
    assert_eq!(page["session_rows"].as_array().map(Vec::len), Some(1));
    assert_eq!(page["total_candidate_count"], json!(1));
    assert_eq!(page["total_matched_count"], json!(1));
    assert_eq!(page["has_more"], false);
}

#[test]
fn request_byte_exhaustion_is_terminal_and_keeps_accepted_rows() {
    let fixture = OrderedSessionFixture::new(
        "request-byte-budget",
        &[("Alpha", 300), ("Bravo", 200), ("Charlie", 100)],
    );
    let first_file_bytes = fs::metadata(fixture.root.join("alpha.jsonl"))
        .expect("first file metadata")
        .len() as usize;
    let params = serde_json::from_value(json!({
        "agent": "codex",
        "authorized_roots": [fixture.root.to_string_lossy()],
        "auto_discover": false,
        "scope": "all",
        "include_content_items": false,
        "paging_mode": "keyset",
        "limit": 100
    }))
    .expect("decode request-byte request");
    let result = fixture
        .host
        .preview_local_sessions_with_test_limits(
            params,
            LocalSessionReadLimits {
                max_preview_read_bytes: first_file_bytes,
                ..LocalSessionReadLimits::default()
            },
        )
        .expect("request-byte limited preview");
    assert_eq!(result.session_rows.len(), 1);
    assert!(result.candidate_set_truncated);
    assert_eq!(result.source_completeness, ListSourceCompleteness::Limited);
    assert_eq!(
        result.incomplete_reason,
        Some(ListIncompleteReason::SafetyBudget)
    );
    assert!(!result.has_more);
    assert!(result.next_cursor.is_none());
    assert_eq!(result.total_matched_count, result.session_rows.len());
}

#[test]
fn rejected_candidates_do_not_inflate_terminal_accepted_total() {
    let fixture = OrderedSessionFixture::new(
        "accepted-total",
        &[("Alpha", 300), ("Bravo", 200), ("Charlie", 100)],
    );
    fs::write(fixture.root.join("bravo.jsonl"), "").expect("empty rejected candidate");
    let page = ordered_result(
        &fixture,
        json!({
            "paging_mode": "keyset",
            "max_files": null,
            "include_content_items": false,
            "limit": 100
        }),
    );
    assert_eq!(page["session_rows"].as_array().map(Vec::len), Some(2));
    assert_eq!(page["total_candidate_count"], json!(3));
    assert_eq!(page["total_matched_count"], json!(2));
    assert_eq!(page["has_more"], false);

    let first = ordered_result(
        &fixture,
        json!({
            "paging_mode": "keyset",
            "max_files": null,
            "include_content_items": false,
            "limit": 1
        }),
    );
    assert_eq!(first["session_rows"].as_array().map(Vec::len), Some(0));
    assert_eq!(first["has_more"], true);
    let second = ordered_result(
        &fixture,
        json!({
            "paging_mode": "keyset",
            "max_files": null,
            "include_content_items": false,
            "limit": 1,
            "cursor": first["next_cursor"],
            "source_revision": first["source_revision"]
        }),
    );
    assert_eq!(second["session_rows"].as_array().map(Vec::len), Some(1));
    assert_eq!(second["total_matched_count"], json!(2));
    assert_eq!(second["has_more"], true);
    let third = ordered_result(
        &fixture,
        json!({
            "paging_mode": "keyset",
            "max_files": null,
            "include_content_items": false,
            "limit": 1,
            "cursor": second["next_cursor"],
            "source_revision": second["source_revision"]
        }),
    );
    assert_eq!(third["session_rows"].as_array().map(Vec::len), Some(1));
    assert_eq!(third["total_matched_count"], json!(2));
    assert_eq!(third["has_more"], false);
}

#[test]
fn keyset_pages_bound_processed_candidates_and_advance_across_empty_rows() {
    let fixture = OrderedSessionFixture::new(
        "candidate-page-bound",
        &[
            ("EmptyOne", 600),
            ("EmptyTwo", 500),
            ("EmptyThree", 400),
            ("EmptyFour", 300),
            ("ValidOne", 200),
            ("ValidTwo", 100),
        ],
    );
    for name in ["emptyone", "emptytwo", "emptythree", "emptyfour"] {
        fs::write(fixture.root.join(format!("{name}.jsonl")), "")
            .expect("write empty rejected candidate");
    }
    let request = |cursor: Option<String>, source_revision: Option<String>| {
        serde_json::from_value(json!({
            "agent": "codex",
            "authorized_roots": [fixture.root.to_string_lossy()],
            "auto_discover": false,
            "scope": "all",
            "include_content_items": false,
            "paging_mode": "keyset",
            "limit": 2,
            "cursor": cursor,
            "source_revision": source_revision
        }))
        .expect("decode candidate-bound request")
    };
    let no_primary_lookahead = LocalSessionReadLimits {
        max_preview_read_bytes: 0,
        ..LocalSessionReadLimits::default()
    };

    let first = fixture
        .host
        .preview_local_sessions_with_test_limits(request(None, None), no_primary_lookahead)
        .expect("first bounded candidate page");
    assert!(first.session_rows.is_empty());
    assert!(
        first.has_more,
        "two processed empties must leave later candidates"
    );
    assert!(
        first.next_cursor.is_some(),
        "an empty page must still advance its cursor"
    );
    assert!(
        !first.candidate_set_truncated,
        "lookahead must not read the third primary file"
    );

    let second = fixture
        .host
        .preview_local_sessions_with_test_limits(
            request(first.next_cursor.clone(), first.source_revision.clone()),
            no_primary_lookahead,
        )
        .expect("second bounded candidate page");
    assert!(second.session_rows.is_empty());
    assert!(
        second.has_more,
        "the second pair of empties must leave valid candidates"
    );
    assert_ne!(
        second.next_cursor, first.next_cursor,
        "an empty page cursor must progress"
    );
    assert!(
        !second.candidate_set_truncated,
        "lookahead must not read the first valid primary file"
    );

    let third = fixture
        .host
        .preview_local_sessions_with_test_limits(
            request(second.next_cursor.clone(), second.source_revision.clone()),
            LocalSessionReadLimits::default(),
        )
        .expect("third bounded candidate page");
    assert_eq!(third.session_rows.len(), 2);
    assert_eq!(
        third
            .session_rows
            .iter()
            .map(|row| row.id.as_str())
            .collect::<HashSet<_>>()
            .len(),
        2
    );
    assert!(!third.has_more);
    assert_eq!(third.total_matched_count, 2);
}

#[test]
fn primary_and_sidecar_budget_exhaustion_are_visible_terminal_limits() {
    let fixture = env::temp_dir().join(format!(
        "skills-copilot-session-budget-matrix-{}-{}",
        std::process::id(),
        NEXT_ORDERED_SESSION_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let storage = fixture.join("home/.local/share/opencode/storage");
    let session_root = storage.join("session");
    let message_root = storage.join("message/ses_budget");
    fs::create_dir_all(&session_root).expect("create opencode session root");
    fs::create_dir_all(&message_root).expect("create opencode message root");
    let first_line = format!(
        "{}\n",
        json!({"id": "ses_budget", "title": "Budget primary"})
    );
    let primary_path = session_root.join("ses_budget.json");
    fs::write(
        message_root.join("000-message.json"),
        json!({"id": "msg_budget", "role": "assistant", "content": "sidecar content that exceeds tiny file budgets"}).to_string(),
    )
    .expect("write opencode sidecar");
    let host = ServiceHost {
        app_data_dir: fixture.join("app-data"),
        adapter_ctx: AdapterContext {
            user_home: fixture.join("home"),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };
    let params = || {
        serde_json::from_value(json!({
            "agent": "opencode",
            "authorized_roots": [storage.to_string_lossy()],
            "auto_discover": false,
            "scope": "all",
            "include_content_items": false,
            "paging_mode": "keyset",
            "limit": 100
        }))
        .expect("decode budget matrix params")
    };
    for (name, limits, expected_request_limit) in [
        (
            "primary-byte",
            LocalSessionReadLimits {
                primary_head_bytes: first_line.len(),
                primary_tail_bytes: 0,
                ..LocalSessionReadLimits::default()
            },
            false,
        ),
        (
            "sidecar-file-count",
            LocalSessionReadLimits {
                max_sidecar_files: 0,
                ..LocalSessionReadLimits::default()
            },
            true,
        ),
        (
            "sidecar-session-bytes",
            LocalSessionReadLimits {
                max_sidecar_session_bytes: 0,
                ..LocalSessionReadLimits::default()
            },
            true,
        ),
        (
            "sidecar-file-bytes",
            LocalSessionReadLimits {
                max_sidecar_file_bytes: 8,
                ..LocalSessionReadLimits::default()
            },
            true,
        ),
    ] {
        let primary_content = if name == "primary-byte" {
            format!(
                "{first_line}{}\n",
                json!({"type": "user", "role": "user", "text": "primary tail"})
            )
        } else {
            first_line.clone()
        };
        fs::write(&primary_path, primary_content).expect("write opencode primary scenario");
        let result = host
            .preview_local_sessions_with_test_limits(params(), limits)
            .unwrap_or_else(|error| panic!("{name} preview failed: {error}"));
        assert_eq!(result.session_rows.len(), 1, "{name} retains primary row");
        assert_eq!(
            result.candidate_set_truncated, expected_request_limit,
            "{name} request-level limitation"
        );
        assert_eq!(
            result.source_completeness,
            if expected_request_limit {
                ListSourceCompleteness::Limited
            } else {
                ListSourceCompleteness::Enumerable
            },
            "{name} completeness"
        );
        assert_eq!(
            result.incomplete_reason,
            expected_request_limit.then_some(ListIncompleteReason::SafetyBudget),
            "{name} reason"
        );
        assert!(!result.has_more, "{name} must be terminal");
        assert!(
            result.next_cursor.is_none(),
            "{name} must not expose cursor"
        );
    }
    let _ = fs::remove_dir_all(fixture);
}

#[test]
fn entry_budget_exhaustion_keeps_rows_and_is_terminal() {
    let fixture = OrderedSessionFixture::new("entry-budget", &[("Accepted", 100)]);
    fs::write(
        fixture.root.join("omitted.jsonl"),
        json!({"type": "session", "title": "Omitted"}).to_string(),
    )
    .expect("write omitted candidate");
    let params = serde_json::from_value(json!({
        "agent": "codex",
        "authorized_roots": [fixture.root.to_string_lossy()],
        "auto_discover": false,
        "scope": "all",
        "include_content_items": false,
        "paging_mode": "keyset",
        "limit": 100
    }))
    .expect("decode entry-budget params");
    let result = fixture
        .host
        .preview_local_sessions_with_test_limits(
            params,
            LocalSessionReadLimits {
                max_inventory_entries: 1,
                ..LocalSessionReadLimits::default()
            },
        )
        .expect("entry-budget preview");
    assert_eq!(result.session_rows.len(), 1);
    assert!(result.candidate_set_truncated);
    assert_eq!(
        result.incomplete_reason,
        Some(ListIncompleteReason::SafetyBudget)
    );
    assert!(!result.has_more);
    assert!(result.next_cursor.is_none());
}

#[test]
fn equal_modified_times_continue_by_stable_identity_without_duplicates() {
    let fixture = OrderedSessionFixture::new(
        "equal-mtime",
        &[("Alpha", 100), ("Bravo", 100), ("Charlie", 100)],
    );
    let mut ids = Vec::new();
    let mut cursor = None;
    let mut revision = None;
    loop {
        let page = ordered_result(
            &fixture,
            json!({
                "paging_mode": "keyset",
                "max_files": null,
                "include_content_items": false,
                "limit": 1,
                "cursor": cursor,
                "source_revision": revision
            }),
        );
        ids.extend(
            ordered_values(&page, "id")
                .into_iter()
                .map(|id| id.as_str().expect("equal-mtime id").to_string()),
        );
        if !page["has_more"].as_bool().expect("equal-mtime has_more") {
            break;
        }
        cursor = page["next_cursor"].as_str().map(str::to_string);
        revision = page["source_revision"].as_str().map(str::to_string);
    }
    assert_eq!(ids.len(), 3);
    assert_eq!(ids.iter().collect::<HashSet<_>>().len(), 3);
}

#[test]
fn inventory_budget_truncation_keeps_rows_and_reports_typed_incompleteness() {
    let fixture = OrderedSessionFixture::new("budget-limited", &[("Accepted", 100)]);
    fs::create_dir_all(fixture.root.join("nested")).expect("create nested directory");
    fs::write(
        fixture.root.join("nested/omitted.jsonl"),
        json!({"type": "session", "title": "Omitted"}).to_string(),
    )
    .expect("write nested candidate");
    let params = serde_json::from_value(json!({
        "agent": "codex",
        "authorized_roots": [fixture.root.to_string_lossy()],
        "auto_discover": false,
        "scope": "all",
        "include_content_items": false,
        "paging_mode": "keyset",
        "limit": 100,
    }))
    .expect("decode budget request");
    let result = fixture
        .host
        .preview_local_sessions_with_test_limits(
            params,
            LocalSessionReadLimits {
                max_inventory_directories: 1,
                ..LocalSessionReadLimits::default()
            },
        )
        .expect("bounded preview");
    let page = serde_json::to_value(result).expect("serialize bounded preview");

    assert_eq!(page["session_rows"].as_array().map(Vec::len), Some(1));
    assert_eq!(page["candidate_set_truncated"], true);
    assert_eq!(page["source_completeness"], "limited");
    assert_eq!(page["incomplete_reason"], "safety_budget");
}
