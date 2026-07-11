use super::*;
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
fn keyset_continuation_rejects_source_change() {
    let fixture = OrderedSessionFixture::new(
        "keyset-source-change",
        &[("Alpha", 100), ("Bravo", 300), ("Charlie", 200)],
    );
    let first = ordered_result(
        &fixture,
        json!({"limit": 1, "max_files": null, "include_content_items": false}),
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
        "cursor": first["next_cursor"],
        "source_revision": first["source_revision"],
    }));
    assert_eq!(
        response.error.map(|error| error.code),
        Some("source_changed".to_string())
    );
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
