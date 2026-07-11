use super::*;
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
