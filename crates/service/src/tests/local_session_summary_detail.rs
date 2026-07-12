use super::*;
use crate::service_local_session_io::{LocalSessionIoContext, LocalSessionReadLimits};
use crate::service_local_sessions::local_session_row_id;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SUMMARY_DETAIL_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct SummaryDetailFixture {
    fixture: PathBuf,
    root: PathBuf,
    app_data: PathBuf,
    host: ServiceHost,
    paths: Vec<PathBuf>,
}

impl SummaryDetailFixture {
    fn new(test_name: &str, session_count: usize) -> Self {
        let fixture = env::temp_dir().join(format!(
            "skills-copilot-session-summary-detail-{test_name}-{}-{}",
            std::process::id(),
            NEXT_SUMMARY_DETAIL_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let root = fixture.join("sessions");
        let app_data = fixture.join("app-data");
        fs::create_dir_all(&root).expect("create session summary/detail fixture");
        let paths = (0..session_count)
            .map(|index| {
                let path = root.join(format!("session-{index}.jsonl"));
                fs::write(
                    &path,
                    format!(
                        "{}\n{}\n",
                        json!({
                            "type": "session",
                            "title": format!("Session {index}"),
                            "timestamp": 1_700_000_000_000_i64 + index as i64,
                        }),
                        json!({
                            "type": "user",
                            "role": "user",
                            "text": format!("SUMMARY_DETAIL_SENTINEL_{index}"),
                            "timestamp": 1_700_000_001_000_i64 + index as i64,
                        })
                    ),
                )
                .expect("write session summary/detail fixture");
                path.canonicalize().expect("canonical session fixture path")
            })
            .collect::<Vec<_>>();
        let host = ServiceHost {
            app_data_dir: app_data.clone(),
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
            app_data,
            host,
            paths,
        }
    }

    fn params(&self) -> LocalSessionPreviewParams {
        serde_json::from_value(json!({
            "agent": "codex",
            "authorized_roots": [self.root.to_string_lossy()],
            "auto_discover": false,
            "scope": "all",
            "limit": 100,
            "max_files": 100,
        }))
        .expect("decode local session preview params")
    }
}

impl Drop for SummaryDetailFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.fixture);
    }
}

fn files_beneath(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(files_beneath(&path));
        } else if path.is_file() {
            files.push(path);
        }
    }
    files
}

#[test]
fn max_files_reports_candidate_set_truncated() {
    let fixture = SummaryDetailFixture::new("truncated", 2);
    let mut params = fixture.params();
    params.max_files = Some(1);
    let result = fixture
        .host
        .preview_local_sessions(params)
        .expect("preview truncated candidate set");
    assert!(result.candidate_set_truncated);
}

#[test]
fn complete_inventory_reports_candidate_set_not_truncated() {
    let fixture = SummaryDetailFixture::new("complete", 2);
    let result = fixture
        .host
        .preview_local_sessions(fixture.params())
        .expect("preview complete candidate set");
    assert!(!result.candidate_set_truncated);
}

#[test]
fn large_bounded_session_content_does_not_mark_candidate_inventory_truncated() {
    let fixture = SummaryDetailFixture::new("large-bounded", 1);
    let path = &fixture.paths[0];
    let mut content = fs::read_to_string(path).expect("read base session");
    let padding_record = format!(
        "{}\n",
        json!({
            "type": "assistant",
            "role": "assistant",
            "text": "x".repeat(4_096),
            "timestamp": 1_700_000_002_000_i64,
        })
    );
    while content.len() < 1024 * 1024 {
        content.push_str(&padding_record);
    }
    fs::write(path, content).expect("write large session");
    let mut params = fixture.params();
    params.include_content_items = Some(false);

    let result = fixture
        .host
        .preview_local_sessions(params)
        .expect("preview large bounded session");

    assert_eq!(result.session_rows.len(), 1);
    assert!(
        !result.candidate_set_truncated,
        "bounded per-file content is not an incomplete candidate inventory"
    );
    assert_eq!(
        result.source_completeness,
        ListSourceCompleteness::Enumerable
    );
    assert_eq!(result.incomplete_reason, None);
}

#[test]
fn summary_rows_omit_content_items_and_mark_not_included() {
    let fixture = SummaryDetailFixture::new("summary", 2);
    let mut params = fixture.params();
    params.include_content_items = Some(false);
    let result = fixture
        .host
        .preview_local_sessions(params)
        .expect("preview summary rows");
    assert_eq!(result.session_rows.len(), 2);
    assert!(result.session_rows.iter().all(|row| {
        !row.content_included
            && row.content_items.is_empty()
            && row.user_message_count == 1
            && row.total_message_count >= 1
    }));
}

#[test]
fn missing_include_content_items_preserves_legacy_detail() {
    let fixture = SummaryDetailFixture::new("legacy", 1);
    let result = fixture
        .host
        .preview_local_sessions(fixture.params())
        .expect("preview legacy detail row");
    let row = result.session_rows.first().expect("legacy row");
    assert!(row.content_included);
    assert!(!row.content_items.is_empty());
}

#[test]
fn detail_session_id_reads_only_target_candidate() {
    let fixture = SummaryDetailFixture::new("detail-read", 2);
    let mut summary_params = fixture.params();
    summary_params.include_content_items = Some(false);
    let summary = fixture
        .host
        .preview_local_sessions(summary_params)
        .expect("preview summary rows");
    let target = summary.session_rows.first().expect("target summary row");
    let target_path = fixture
        .paths
        .iter()
        .find(|path| local_session_row_id(path) == target.id)
        .expect("target path")
        .clone();

    let mut detail_params = fixture.params();
    detail_params.include_content_items = Some(true);
    detail_params.session_id = Some(target.id.clone());
    detail_params.limit = Some(1);
    detail_params.offset = Some(0);
    let mut io = LocalSessionIoContext::new(LocalSessionReadLimits::default());
    let detail = fixture
        .host
        .preview_local_sessions_with_io(detail_params, &mut io)
        .expect("preview selected session detail");

    assert_eq!(detail.session_rows.len(), 1);
    assert_eq!(detail.session_rows[0].id, target.id);
    assert!(detail.session_rows[0].content_included);
    assert_eq!(io.primary_paths_read, vec![target_path]);
}

#[test]
fn session_preview_never_persists_raw_session_content() {
    let fixture = SummaryDetailFixture::new("no-persistence", 1);
    let mut summary_params = fixture.params();
    summary_params.include_content_items = Some(false);
    let summary = fixture
        .host
        .preview_local_sessions(summary_params)
        .expect("preview summary without persistence");
    let target_id = summary.session_rows[0].id.clone();

    let mut detail_params = fixture.params();
    detail_params.include_content_items = Some(true);
    detail_params.session_id = Some(target_id);
    detail_params.limit = Some(1);
    fixture
        .host
        .preview_local_sessions(detail_params)
        .expect("preview detail without persistence");

    for path in files_beneath(&fixture.app_data) {
        let bytes = fs::read(&path).expect("read app-data evidence file");
        assert!(
            !String::from_utf8_lossy(&bytes).contains("SUMMARY_DETAIL_SENTINEL"),
            "raw session content persisted to {}",
            path.display()
        );
    }
}
