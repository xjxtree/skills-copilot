use super::ServiceError;
use serde_json::{Map, Value};
use std::{
    collections::HashMap,
    fs,
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

#[cfg(unix)]
use std::collections::BinaryHeap;

#[cfg(unix)]
use std::{
    os::fd::{AsFd, OwnedFd},
    os::unix::ffi::OsStrExt,
    path::Component,
};

const MAX_READ_CHUNK_BYTES: usize = 64 * 1024;

#[cfg(all(test, unix))]
static SCHEDULED_SIDECAR_READ_FAULTS: std::sync::Mutex<Vec<(PathBuf, usize)>> =
    std::sync::Mutex::new(Vec::new());

#[cfg(all(test, unix))]
pub(crate) fn install_scheduled_sidecar_read_fault(path: PathBuf, bytes_before_error: usize) {
    let mut faults = SCHEDULED_SIDECAR_READ_FAULTS
        .lock()
        .expect("lock scheduled sidecar read faults");
    assert!(
        !faults.iter().any(|(scheduled, _)| scheduled == &path),
        "sidecar read fault already scheduled for {}",
        path.display()
    );
    faults.push((path, bytes_before_error));
}

#[cfg(all(test, unix))]
fn take_scheduled_sidecar_read_fault(path: &Path) -> Option<usize> {
    let mut faults = SCHEDULED_SIDECAR_READ_FAULTS
        .lock()
        .expect("lock scheduled sidecar read faults");
    let index = faults.iter().position(|(scheduled, _)| scheduled == path)?;
    Some(faults.swap_remove(index).1)
}

#[cfg(all(test, unix))]
struct ScheduledReadFault<R> {
    inner: R,
    bytes_before_error: usize,
}

#[cfg(all(test, unix))]
impl<R: Read> Read for ScheduledReadFault<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.bytes_before_error == 0 {
            return Err(io::Error::other("injected sidecar read fault"));
        }
        let allowed = buffer.len().min(self.bytes_before_error);
        let read = self.inner.read(&mut buffer[..allowed])?;
        self.bytes_before_error = self.bytes_before_error.saturating_sub(read);
        Ok(read)
    }
}

#[cfg(all(test, unix))]
impl<R: Seek> Seek for ScheduledReadFault<R> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek(position)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BoundedReadSpec {
    pub(crate) head_bytes: usize,
    pub(crate) tail_bytes: usize,
    pub(crate) line_fragment_bytes: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct BoundedText {
    pub(crate) head: String,
    pub(crate) tail: String,
    pub(crate) retained_head_end: u64,
    pub(crate) retained_tail_start: u64,
    pub(crate) tail_starts_at_line_boundary: bool,
    pub(crate) gap_stays_on_same_line: bool,
    pub(crate) record_provenance: Option<BoundedRecordProvenance>,
    pub(crate) modified_at_millis: Option<i64>,
    pub(crate) truncated: bool,
    pub(crate) request_budget_exhausted: bool,
    pub(crate) bytes_read: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct BoundedRecordProvenance {
    record_type: ScalarProvenance,
    role: ScalarProvenance,
    text: ScalarProvenance,
    content: ScalarProvenance,
    title: ScalarProvenance,
    ai_title: ScalarProvenance,
    timestamp: ScalarProvenance,
    session_id: ScalarProvenance,
    id: ScalarProvenance,
    cwd: ScalarProvenance,
}

impl BoundedRecordProvenance {
    pub(crate) fn merge_into(&self, fields: &mut Map<String, Value>) {
        self.record_type.merge_classification_into(fields, "type");
        self.role.merge_classification_into(fields, "role");
        self.text.merge_into(fields, "text");
        self.content.merge_into(fields, "content");
        self.title.merge_into(fields, "title");
        self.ai_title.merge_into(fields, "aiTitle");
        self.timestamp.merge_into(fields, "timestamp");
        self.session_id.merge_into(fields, "sessionId");
        self.id.merge_into(fields, "id");
        self.cwd.merge_into(fields, "cwd");
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
enum ScalarProvenance {
    #[default]
    Missing,
    Scalar(Value),
    Unsupported,
}

impl ScalarProvenance {
    fn merge_into(&self, fields: &mut Map<String, Value>, key: &str) {
        match self {
            Self::Missing => {}
            Self::Scalar(value) => {
                fields.insert(key.to_string(), value.clone());
            }
            Self::Unsupported => {
                fields.remove(key);
            }
        }
    }

    fn merge_classification_into(&self, fields: &mut Map<String, Value>, key: &str) {
        match self {
            Self::Missing => {}
            Self::Scalar(value) => {
                fields.insert(key.to_string(), value.clone());
            }
            Self::Unsupported => {
                fields.insert(key.to_string(), Value::Null);
            }
        }
    }
}

pub(crate) const MAX_PROVENANCE_TOKEN_BYTES: usize = 4 * 1024;
const MAX_PROVENANCE_NESTING: usize = 128;

fn is_json_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ProvenanceKey {
    RecordType,
    Role,
    Text,
    Content,
    Title,
    AiTitle,
    Timestamp,
    SessionId,
    Id,
    Cwd,
    Other,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum RootParseState {
    Start,
    FirstKeyOrEnd,
    Key,
    Colon,
    Value,
    AfterValue,
    Complete,
    Invalid,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum StringPurpose {
    RootKey,
    RootValue(ProvenanceKey),
    NestedKey,
    NestedValue,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum PrimitivePurpose {
    RootValue(ProvenanceKey),
    NestedValue,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum NestedParseState {
    ObjectFirstKeyOrEnd,
    ObjectKey,
    ObjectColon,
    ObjectValue,
    ObjectAfterValue,
    ArrayFirstValueOrEnd,
    ArrayValue,
    ArrayAfterValue,
}

struct TopLevelProvenanceScanner {
    state: RootParseState,
    nested_stack: Vec<NestedParseState>,
    current_key: ProvenanceKey,
    string_purpose: Option<StringPurpose>,
    string_token: Vec<u8>,
    string_token_overflowed: bool,
    escaped: bool,
    unicode_escape_remaining: u8,
    utf8_remaining: u8,
    utf8_next_min: u8,
    utf8_next_max: u8,
    unicode_escape_value: u16,
    pending_high_surrogate: bool,
    unicode_escape_is_low_surrogate: bool,
    primitive_token: Vec<u8>,
    primitive_purpose: Option<PrimitivePurpose>,
    provenance: BoundedRecordProvenance,
}

impl TopLevelProvenanceScanner {
    fn new() -> Self {
        Self {
            state: RootParseState::Start,
            nested_stack: Vec::with_capacity(16),
            current_key: ProvenanceKey::Other,
            string_purpose: None,
            string_token: Vec::with_capacity(64),
            string_token_overflowed: false,
            escaped: false,
            unicode_escape_remaining: 0,
            utf8_remaining: 0,
            utf8_next_min: 0x80,
            utf8_next_max: 0xbf,
            unicode_escape_value: 0,
            pending_high_surrogate: false,
            unicode_escape_is_low_surrogate: false,
            primitive_token: Vec::with_capacity(32),
            primitive_purpose: None,
            provenance: BoundedRecordProvenance {
                record_type: ScalarProvenance::Missing,
                role: ScalarProvenance::Missing,
                text: ScalarProvenance::Missing,
                content: ScalarProvenance::Missing,
                title: ScalarProvenance::Missing,
                ai_title: ScalarProvenance::Missing,
                timestamp: ScalarProvenance::Missing,
                session_id: ScalarProvenance::Missing,
                id: ScalarProvenance::Missing,
                cwd: ScalarProvenance::Missing,
            },
        }
    }

    fn feed(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.feed_byte(*byte);
        }
    }

    fn finish(mut self) -> Option<BoundedRecordProvenance> {
        if self.primitive_purpose.is_some() {
            self.finish_primitive();
        }
        (self.state == RootParseState::Complete
            && self.nested_stack.is_empty()
            && self.string_purpose.is_none()
            && self.primitive_purpose.is_none())
        .then_some(self.provenance)
    }

    fn feed_byte(&mut self, byte: u8) {
        if self.state == RootParseState::Invalid {
            return;
        }
        if self.string_purpose.is_some() {
            self.feed_string_byte(byte);
            return;
        }
        if self.primitive_purpose.is_some() {
            if is_json_whitespace(byte) || matches!(byte, b',' | b'}' | b']') {
                self.finish_primitive();
                if self.state != RootParseState::Invalid {
                    self.feed_byte(byte);
                }
            } else {
                self.push_primitive_byte(byte);
            }
            return;
        }
        if !self.nested_stack.is_empty() {
            self.feed_nested_byte(byte);
            return;
        }

        if is_json_whitespace(byte) && self.state != RootParseState::Value {
            return;
        }
        match self.state {
            RootParseState::Start => {
                if byte == b'{' {
                    self.state = RootParseState::FirstKeyOrEnd;
                } else {
                    self.invalidate();
                }
            }
            RootParseState::FirstKeyOrEnd => match byte {
                b'"' => self.begin_string(StringPurpose::RootKey),
                b'}' => self.state = RootParseState::Complete,
                _ => self.invalidate(),
            },
            RootParseState::Key => {
                if byte == b'"' {
                    self.begin_string(StringPurpose::RootKey);
                } else {
                    self.invalidate();
                }
            }
            RootParseState::Colon => {
                if byte == b':' {
                    self.state = RootParseState::Value;
                } else {
                    self.invalidate();
                }
            }
            RootParseState::Value => {
                if is_json_whitespace(byte) {
                    return;
                }
                match byte {
                    b'"' => self.begin_string(StringPurpose::RootValue(self.current_key)),
                    b'{' | b'[' => {
                        self.set_current_field(ScalarProvenance::Unsupported);
                        self.state = RootParseState::AfterValue;
                        self.push_nested(byte);
                    }
                    b',' | b'}' | b']' => self.invalidate(),
                    _ => self.begin_primitive(PrimitivePurpose::RootValue(self.current_key), byte),
                }
            }
            RootParseState::AfterValue => match byte {
                b',' => self.state = RootParseState::Key,
                b'}' => self.state = RootParseState::Complete,
                _ => self.invalidate(),
            },
            RootParseState::Complete => self.invalidate(),
            RootParseState::Invalid => {}
        }
    }

    fn begin_primitive(&mut self, purpose: PrimitivePurpose, first_byte: u8) {
        self.primitive_token.clear();
        self.primitive_purpose = Some(purpose);
        self.push_primitive_byte(first_byte);
    }

    fn feed_nested_byte(&mut self, byte: u8) {
        let state = *self.nested_stack.last().expect("nested state");
        if is_json_whitespace(byte)
            && !matches!(
                state,
                NestedParseState::ObjectValue
                    | NestedParseState::ArrayFirstValueOrEnd
                    | NestedParseState::ArrayValue
            )
        {
            return;
        }
        match state {
            NestedParseState::ObjectFirstKeyOrEnd => match byte {
                b'"' => self.begin_string(StringPurpose::NestedKey),
                b'}' => self.close_nested(b'}'),
                _ => self.invalidate(),
            },
            NestedParseState::ObjectKey => {
                if byte == b'"' {
                    self.begin_string(StringPurpose::NestedKey);
                } else {
                    self.invalidate();
                }
            }
            NestedParseState::ObjectColon => {
                if byte == b':' {
                    *self.nested_stack.last_mut().expect("nested state") =
                        NestedParseState::ObjectValue;
                } else {
                    self.invalidate();
                }
            }
            NestedParseState::ObjectValue | NestedParseState::ArrayValue => {
                if is_json_whitespace(byte) {
                    return;
                }
                self.begin_nested_value(byte);
            }
            NestedParseState::ObjectAfterValue => match byte {
                b',' => {
                    *self.nested_stack.last_mut().expect("nested state") =
                        NestedParseState::ObjectKey;
                }
                b'}' => self.close_nested(b'}'),
                _ => self.invalidate(),
            },
            NestedParseState::ArrayFirstValueOrEnd => {
                if is_json_whitespace(byte) {
                    return;
                }
                if byte == b']' {
                    self.close_nested(b']');
                } else {
                    self.begin_nested_value(byte);
                }
            }
            NestedParseState::ArrayAfterValue => match byte {
                b',' => {
                    *self.nested_stack.last_mut().expect("nested state") =
                        NestedParseState::ArrayValue;
                }
                b']' => self.close_nested(b']'),
                _ => self.invalidate(),
            },
        }
    }

    fn begin_nested_value(&mut self, byte: u8) {
        match byte {
            b'"' => self.begin_string(StringPurpose::NestedValue),
            b'{' | b'[' => {
                self.complete_nested_value();
                if self.state != RootParseState::Invalid {
                    self.push_nested(byte);
                }
            }
            b',' | b'}' | b']' => self.invalidate(),
            _ => self.begin_primitive(PrimitivePurpose::NestedValue, byte),
        }
    }

    fn complete_nested_value(&mut self) {
        let Some(state) = self.nested_stack.last_mut() else {
            self.invalidate();
            return;
        };
        *state = match *state {
            NestedParseState::ObjectValue => NestedParseState::ObjectAfterValue,
            NestedParseState::ArrayFirstValueOrEnd | NestedParseState::ArrayValue => {
                NestedParseState::ArrayAfterValue
            }
            _ => {
                self.invalidate();
                return;
            }
        };
    }

    fn close_nested(&mut self, closing: u8) {
        let Some(state) = self.nested_stack.last().copied() else {
            self.invalidate();
            return;
        };
        let valid = match closing {
            b'}' => matches!(
                state,
                NestedParseState::ObjectFirstKeyOrEnd | NestedParseState::ObjectAfterValue
            ),
            b']' => matches!(
                state,
                NestedParseState::ArrayFirstValueOrEnd | NestedParseState::ArrayAfterValue
            ),
            _ => false,
        };
        if valid {
            self.nested_stack.pop();
        } else {
            self.invalidate();
        }
    }

    fn begin_string(&mut self, purpose: StringPurpose) {
        self.string_purpose = Some(purpose);
        self.string_token.clear();
        self.string_token_overflowed = false;
        self.escaped = false;
        self.unicode_escape_remaining = 0;
        self.utf8_remaining = 0;
        self.utf8_next_min = 0x80;
        self.utf8_next_max = 0xbf;
        self.unicode_escape_value = 0;
        self.pending_high_surrogate = false;
        self.unicode_escape_is_low_surrogate = false;
        if !matches!(
            purpose,
            StringPurpose::NestedKey
                | StringPurpose::NestedValue
                | StringPurpose::RootValue(ProvenanceKey::Other)
        ) {
            self.push_string_token_byte(b'"');
        }
    }

    fn feed_string_byte(&mut self, byte: u8) {
        let purpose = self.string_purpose.expect("string purpose");
        let capture = !matches!(
            purpose,
            StringPurpose::NestedKey
                | StringPurpose::NestedValue
                | StringPurpose::RootValue(ProvenanceKey::Other)
        );
        if capture {
            self.push_string_token_byte(byte);
        }

        if self.unicode_escape_remaining > 0 {
            let Some(digit) = (byte as char).to_digit(16) else {
                self.invalidate();
                return;
            };
            self.unicode_escape_value = (self.unicode_escape_value << 4) | digit as u16;
            self.unicode_escape_remaining -= 1;
            if self.unicode_escape_remaining == 0 {
                let value = self.unicode_escape_value;
                if self.unicode_escape_is_low_surrogate {
                    if !(0xdc00..=0xdfff).contains(&value) {
                        self.invalidate();
                        return;
                    }
                    self.pending_high_surrogate = false;
                    self.unicode_escape_is_low_surrogate = false;
                } else if (0xd800..=0xdbff).contains(&value) {
                    self.pending_high_surrogate = true;
                } else if (0xdc00..=0xdfff).contains(&value) {
                    self.invalidate();
                }
            }
            return;
        }
        if self.escaped {
            self.escaped = false;
            if self.pending_high_surrogate {
                if byte != b'u' {
                    self.invalidate();
                    return;
                }
                self.unicode_escape_remaining = 4;
                self.unicode_escape_value = 0;
                self.unicode_escape_is_low_surrogate = true;
                return;
            }
            match byte {
                b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => {}
                b'u' => {
                    self.unicode_escape_remaining = 4;
                    self.unicode_escape_value = 0;
                }
                _ => self.invalidate(),
            }
            return;
        }
        if self.utf8_remaining > 0 {
            if !(self.utf8_next_min..=self.utf8_next_max).contains(&byte) {
                self.invalidate();
                return;
            }
            self.utf8_remaining -= 1;
            self.utf8_next_min = 0x80;
            self.utf8_next_max = 0xbf;
            return;
        }
        if self.pending_high_surrogate && byte != b'\\' {
            self.invalidate();
            return;
        }
        match byte {
            b'"' => self.finish_string(purpose),
            b'\\' => self.escaped = true,
            0x00..=0x1f => self.invalidate(),
            0x20..=0x7f => {}
            0xc2..=0xdf => self.start_utf8_sequence(1, 0x80, 0xbf),
            0xe0 => self.start_utf8_sequence(2, 0xa0, 0xbf),
            0xe1..=0xec | 0xee..=0xef => self.start_utf8_sequence(2, 0x80, 0xbf),
            0xed => self.start_utf8_sequence(2, 0x80, 0x9f),
            0xf0 => self.start_utf8_sequence(3, 0x90, 0xbf),
            0xf1..=0xf3 => self.start_utf8_sequence(3, 0x80, 0xbf),
            0xf4 => self.start_utf8_sequence(3, 0x80, 0x8f),
            _ => self.invalidate(),
        }
    }

    fn finish_string(&mut self, purpose: StringPurpose) {
        self.string_purpose = None;
        if self.string_token_overflowed {
            match purpose {
                StringPurpose::RootKey => {
                    self.current_key = ProvenanceKey::Other;
                    self.state = RootParseState::Colon;
                }
                StringPurpose::RootValue(key) => {
                    self.set_field(key, ScalarProvenance::Unsupported);
                    self.state = RootParseState::AfterValue;
                }
                StringPurpose::NestedKey | StringPurpose::NestedValue => unreachable!(),
            }
            return;
        }
        match purpose {
            StringPurpose::RootKey => {
                let Ok(key) = serde_json::from_slice::<String>(&self.string_token) else {
                    self.invalidate();
                    return;
                };
                self.current_key = match key.as_str() {
                    "type" => ProvenanceKey::RecordType,
                    "role" => ProvenanceKey::Role,
                    "text" => ProvenanceKey::Text,
                    "content" => ProvenanceKey::Content,
                    "title" => ProvenanceKey::Title,
                    "aiTitle" => ProvenanceKey::AiTitle,
                    "timestamp" => ProvenanceKey::Timestamp,
                    "sessionId" => ProvenanceKey::SessionId,
                    "id" => ProvenanceKey::Id,
                    "cwd" => ProvenanceKey::Cwd,
                    _ => ProvenanceKey::Other,
                };
                self.state = RootParseState::Colon;
            }
            StringPurpose::RootValue(key) => {
                if key != ProvenanceKey::Other {
                    let Ok(value) = serde_json::from_slice::<Value>(&self.string_token) else {
                        self.invalidate();
                        return;
                    };
                    self.set_field(key, ScalarProvenance::Scalar(value));
                }
                self.state = RootParseState::AfterValue;
            }
            StringPurpose::NestedKey => {
                let Some(state) = self.nested_stack.last_mut() else {
                    self.invalidate();
                    return;
                };
                if matches!(
                    *state,
                    NestedParseState::ObjectFirstKeyOrEnd | NestedParseState::ObjectKey
                ) {
                    *state = NestedParseState::ObjectColon;
                } else {
                    self.invalidate();
                }
            }
            StringPurpose::NestedValue => self.complete_nested_value(),
        }
    }

    fn start_utf8_sequence(&mut self, remaining: u8, next_min: u8, next_max: u8) {
        self.utf8_remaining = remaining;
        self.utf8_next_min = next_min;
        self.utf8_next_max = next_max;
    }

    fn push_nested(&mut self, byte: u8) {
        if self.nested_stack.len().saturating_add(1) >= MAX_PROVENANCE_NESTING {
            self.invalidate();
            return;
        }
        let state = match byte {
            b'{' => NestedParseState::ObjectFirstKeyOrEnd,
            b'[' => NestedParseState::ArrayFirstValueOrEnd,
            _ => {
                self.invalidate();
                return;
            }
        };
        self.nested_stack.push(state);
    }

    fn push_string_token_byte(&mut self, byte: u8) {
        if self.string_token.len() < MAX_PROVENANCE_TOKEN_BYTES {
            self.string_token.push(byte);
        } else {
            self.string_token_overflowed = true;
        }
    }

    fn push_primitive_byte(&mut self, byte: u8) {
        if self.primitive_token.len() < MAX_PROVENANCE_TOKEN_BYTES {
            self.primitive_token.push(byte);
        } else {
            self.invalidate();
        }
    }

    fn finish_primitive(&mut self) {
        let Some(purpose) = self.primitive_purpose.take() else {
            return;
        };
        let Ok(value) = serde_json::from_slice::<Value>(&self.primitive_token) else {
            self.invalidate();
            return;
        };
        if matches!(value, Value::Array(_) | Value::Object(_) | Value::String(_)) {
            self.invalidate();
            return;
        }
        match purpose {
            PrimitivePurpose::RootValue(key) => {
                self.set_field(key, ScalarProvenance::Scalar(value));
                self.state = RootParseState::AfterValue;
            }
            PrimitivePurpose::NestedValue => self.complete_nested_value(),
        }
    }

    fn set_current_field(&mut self, value: ScalarProvenance) {
        self.set_field(self.current_key, value);
    }

    fn set_field(&mut self, key: ProvenanceKey, value: ScalarProvenance) {
        match key {
            ProvenanceKey::RecordType => self.provenance.record_type = value,
            ProvenanceKey::Role => self.provenance.role = value,
            ProvenanceKey::Text => self.provenance.text = value,
            ProvenanceKey::Content => self.provenance.content = value,
            ProvenanceKey::Title => self.provenance.title = value,
            ProvenanceKey::AiTitle => self.provenance.ai_title = value,
            ProvenanceKey::Timestamp => self.provenance.timestamp = value,
            ProvenanceKey::SessionId => self.provenance.session_id = value,
            ProvenanceKey::Id => self.provenance.id = value,
            ProvenanceKey::Cwd => self.provenance.cwd = value,
            ProvenanceKey::Other => {}
        }
    }

    fn invalidate(&mut self) {
        self.state = RootParseState::Invalid;
        self.string_purpose = None;
        self.primitive_purpose = None;
    }
}

struct RetainedTailWindow<'a> {
    bytes: &'a [u8],
    start_offset: usize,
    starts_at_line_boundary: bool,
}

struct DecodedUtf8Window {
    text: String,
    start_offset: usize,
    end_offset: usize,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(crate) struct LocalSessionReadLimits {
    pub(crate) primary_head_bytes: usize,
    pub(crate) primary_tail_bytes: usize,
    pub(crate) max_line_fragment_bytes: usize,
    pub(crate) max_sidecar_file_bytes: usize,
    pub(crate) max_sidecar_session_bytes: usize,
    pub(crate) max_sidecar_files: usize,
    pub(crate) max_preview_read_bytes: usize,
    pub(crate) max_inventory_directories: usize,
    pub(crate) max_inventory_entries: usize,
}

impl Default for LocalSessionReadLimits {
    fn default() -> Self {
        Self {
            primary_head_bytes: 384 * 1024,
            primary_tail_bytes: 128 * 1024,
            max_line_fragment_bytes: 64 * 1024,
            max_sidecar_file_bytes: 64 * 1024,
            max_sidecar_session_bytes: 512 * 1024,
            max_sidecar_files: 240,
            max_preview_read_bytes: 64 * 1024 * 1024,
            max_inventory_directories: 20_000,
            max_inventory_entries: 100_000,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LocalSessionReadBudget {
    remaining_bytes: usize,
}

impl LocalSessionReadBudget {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            remaining_bytes: limit,
        }
    }

    fn claim(&mut self, requested: usize) -> usize {
        let granted = requested.min(self.remaining_bytes);
        self.remaining_bytes -= granted;
        granted
    }

    fn claim_exact(&mut self, requested: usize) -> bool {
        if requested > self.remaining_bytes {
            return false;
        }
        self.remaining_bytes -= requested;
        true
    }

    pub(crate) fn remaining_bytes(&self) -> usize {
        self.remaining_bytes
    }

    fn refund(&mut self, unused: usize) {
        self.remaining_bytes = self.remaining_bytes.saturating_add(unused);
    }
}

pub(crate) struct SessionSidecarBudget {
    remaining_files: usize,
    remaining_bytes: usize,
}

pub(crate) struct LocalSessionInventoryBudget {
    #[cfg(unix)]
    remaining_directories: usize,
    #[cfg(unix)]
    remaining_entries: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct LocalSessionFileCandidate {
    pub(crate) path: PathBuf,
    pub(crate) modified_at: i64,
}

#[derive(Debug, Default)]
pub(crate) struct LocalSessionInventory {
    pub(crate) candidates: Vec<LocalSessionFileCandidate>,
    pub(crate) total_candidate_count: usize,
    pub(crate) truncated: bool,
}

pub(crate) fn select_newest_candidates(
    mut candidates: Vec<LocalSessionFileCandidate>,
    max_files: usize,
) -> Vec<LocalSessionFileCandidate> {
    candidates.sort_by(|left, right| {
        right
            .modified_at
            .cmp(&left.modified_at)
            .then_with(|| left.path.cmp(&right.path))
    });
    candidates.truncate(max_files);
    candidates
}

#[cfg(unix)]
fn stat_modified_at_millis<Seconds, Nanoseconds>(seconds: Seconds, nanoseconds: Nanoseconds) -> i64
where
    Seconds: TryInto<i64>,
    Nanoseconds: TryInto<i64>,
{
    let seconds = seconds.try_into().unwrap_or(i64::MAX);
    let nanoseconds = nanoseconds.try_into().unwrap_or(0).clamp(0, 999_999_999);
    seconds
        .saturating_mul(1_000)
        .saturating_add(nanoseconds / 1_000_000)
}

impl LocalSessionInventoryBudget {
    fn new(remaining_directories: usize, remaining_entries: usize) -> Self {
        #[cfg(not(unix))]
        let _ = (remaining_directories, remaining_entries);
        Self {
            #[cfg(unix)]
            remaining_directories,
            #[cfg(unix)]
            remaining_entries,
        }
    }

    #[cfg(unix)]
    fn claim_directory(&mut self) -> bool {
        if self.remaining_directories == 0 {
            return false;
        }
        self.remaining_directories -= 1;
        true
    }

    #[cfg(unix)]
    fn claim_entry(&mut self) -> bool {
        if self.remaining_entries == 0 {
            return false;
        }
        self.remaining_entries -= 1;
        true
    }

    #[cfg(all(test, unix))]
    fn remaining_entries(&self) -> usize {
        self.remaining_entries
    }

    #[cfg(all(test, unix))]
    fn remaining_directories(&self) -> usize {
        self.remaining_directories
    }
}

impl SessionSidecarBudget {
    pub(crate) fn new(remaining_files: usize, remaining_bytes: usize) -> Self {
        Self {
            remaining_files,
            remaining_bytes,
        }
    }

    pub(crate) fn claim_file(&mut self) -> bool {
        if self.remaining_files == 0 || self.remaining_bytes == 0 {
            return false;
        }
        self.remaining_files -= 1;
        true
    }

    pub(crate) fn remaining_files(&self) -> usize {
        self.remaining_files
    }

    pub(crate) fn remaining_bytes(&self) -> usize {
        self.remaining_bytes
    }

    fn claim_bytes_exact(&mut self, requested: usize) -> bool {
        if requested > self.remaining_bytes {
            return false;
        }
        self.remaining_bytes -= requested;
        true
    }

    fn refund_bytes(&mut self, unused: usize) {
        self.remaining_bytes = self.remaining_bytes.saturating_add(unused);
    }
}

#[cfg(unix)]
fn open_guarded_relative_directory(
    root: &OwnedFd,
    relative: &Path,
    flags: rustix::fs::OFlags,
) -> io::Result<OwnedFd> {
    use rustix::fs::{openat, Mode};

    let mut directory = openat(root, ".", flags, Mode::empty()).map_err(io::Error::from)?;
    for component in relative.components() {
        match component {
            Component::Normal(name) => {
                directory =
                    openat(&directory, name, flags, Mode::empty()).map_err(io::Error::from)?;
            }
            Component::CurDir => {}
            Component::RootDir | Component::Prefix(_) | Component::ParentDir => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "guarded session directory contains an unsupported component",
                ));
            }
        }
    }
    Ok(directory)
}

pub(crate) struct LocalSessionIoContext {
    pub(crate) limits: LocalSessionReadLimits,
    pub(crate) budget: LocalSessionReadBudget,
    pub(crate) inventory_budget: LocalSessionInventoryBudget,
    pub(crate) cache: LocalSessionRequestCache,
    #[cfg(test)]
    pub(crate) primary_paths_read: Vec<PathBuf>,
}

#[derive(Debug, Default)]
pub(crate) struct LocalSessionRequestCache {
    pub(crate) codex_titles: HashMap<PathBuf, HashMap<String, String>>,
}

impl LocalSessionRequestCache {
    pub(crate) fn codex_titles_or_load<F>(
        &mut self,
        root: PathBuf,
        load: F,
    ) -> &HashMap<String, String>
    where
        F: FnOnce() -> HashMap<String, String>,
    {
        self.codex_titles.entry(root).or_insert_with(load)
    }
}

pub(crate) struct GuardedLocalSessionRoot {
    #[cfg(unix)]
    path: PathBuf,
    #[cfg(unix)]
    directory: OwnedFd,
}

pub(crate) struct GuardedLocalSessionInventory {
    pub(crate) files: Vec<PathBuf>,
    pub(crate) truncated: bool,
}

pub(crate) struct GuardedLocalSessionMetadataInventory {
    pub(crate) inventory: LocalSessionInventory,
    pub(crate) directory_errors: Vec<(PathBuf, io::Error)>,
}

impl GuardedLocalSessionRoot {
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        if !path.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "guarded session root must be absolute",
            ));
        }

        #[cfg(unix)]
        {
            use rustix::fs::{open, Mode, OFlags};

            let directory_flags =
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
            let directory = open(path, directory_flags, Mode::empty()).map_err(io::Error::from)?;
            let resolved_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            Ok(Self {
                path: resolved_path,
                directory,
            })
        }

        #[cfg(not(unix))]
        {
            let _ = path;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "guarded local session reads require descriptor-relative file access",
            ))
        }
    }

    pub(crate) fn open_beneath(anchor: &Path, path: &Path) -> io::Result<Self> {
        if !anchor.is_absolute() || !path.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "guarded session root and authorization anchor must be absolute",
            ));
        }
        let relative = path.strip_prefix(anchor).map_err(|_| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "session root is outside its authorization anchor",
            )
        })?;

        #[cfg(unix)]
        {
            use rustix::fs::{open, openat, Mode, OFlags};

            let directory_flags =
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
            let mut directory =
                open(anchor, directory_flags, Mode::empty()).map_err(io::Error::from)?;
            for component in relative.components() {
                match component {
                    Component::Normal(name) => {
                        directory = openat(&directory, name, directory_flags, Mode::empty())
                            .map_err(io::Error::from)?;
                    }
                    Component::CurDir => {}
                    Component::RootDir | Component::Prefix(_) | Component::ParentDir => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "guarded session root contains an unsupported component",
                        ));
                    }
                }
            }
            let resolved_anchor = anchor
                .canonicalize()
                .unwrap_or_else(|_| anchor.to_path_buf());
            Ok(Self {
                path: resolved_anchor.join(relative),
                directory,
            })
        }

        #[cfg(not(unix))]
        {
            let _ = relative;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "guarded local session reads require descriptor-relative file access",
            ))
        }
    }

    pub(crate) fn path(&self) -> &Path {
        #[cfg(unix)]
        {
            &self.path
        }

        #[cfg(not(unix))]
        {
            unreachable!("non-Unix guarded roots cannot be constructed")
        }
    }

    pub(crate) fn collect_regular_files(
        &self,
        budget: &mut LocalSessionInventoryBudget,
        mut is_candidate: impl FnMut(&Path) -> bool,
    ) -> io::Result<GuardedLocalSessionMetadataInventory> {
        #[cfg(unix)]
        {
            use rustix::fs::{statat, AtFlags, Dir, FileType, OFlags};

            let mut collected = GuardedLocalSessionMetadataInventory {
                inventory: LocalSessionInventory::default(),
                directory_errors: Vec::new(),
            };
            let directory_flags =
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
            let mut directories = vec![PathBuf::new()];

            while let Some(relative_directory) = directories.pop() {
                if !budget.claim_directory() {
                    collected.inventory.truncated = true;
                    return Ok(collected);
                }
                let directory = match open_guarded_relative_directory(
                    &self.directory,
                    &relative_directory,
                    directory_flags,
                ) {
                    Ok(directory) => directory,
                    Err(error) if relative_directory.as_os_str().is_empty() => return Err(error),
                    Err(error) => {
                        collected
                            .directory_errors
                            .push((self.path.join(&relative_directory), error));
                        continue;
                    }
                };
                let mut entries = match Dir::read_from(&directory) {
                    Ok(entries) => entries,
                    Err(error) => {
                        collected
                            .directory_errors
                            .push((self.path.join(&relative_directory), io::Error::from(error)));
                        continue;
                    }
                };
                let mut entry_names = Vec::new();
                for entry in &mut entries {
                    let entry = match entry {
                        Ok(entry) => entry,
                        Err(error) => {
                            collected.directory_errors.push((
                                self.path.join(&relative_directory),
                                io::Error::from(error),
                            ));
                            break;
                        }
                    };
                    let name_bytes = entry.file_name().to_bytes();
                    if matches!(name_bytes, b"." | b"..") {
                        continue;
                    }
                    entry_names.push(std::ffi::OsStr::from_bytes(name_bytes).to_owned());
                }
                entry_names.sort();
                let mut child_directories = Vec::new();
                for name in entry_names {
                    if !budget.claim_entry() {
                        collected.inventory.truncated = true;
                        return Ok(collected);
                    }
                    let relative_path = relative_directory.join(&name);
                    let metadata = match statat(&directory, &name, AtFlags::SYMLINK_NOFOLLOW) {
                        Ok(metadata) => metadata,
                        Err(_) => continue,
                    };
                    match FileType::from_raw_mode(metadata.st_mode) {
                        FileType::Directory => {
                            child_directories.push(relative_path);
                        }
                        FileType::RegularFile => {
                            let path = self.path.join(relative_path);
                            if is_candidate(&path) {
                                collected.inventory.total_candidate_count += 1;
                                collected
                                    .inventory
                                    .candidates
                                    .push(LocalSessionFileCandidate {
                                        path,
                                        modified_at: stat_modified_at_millis(
                                            metadata.st_mtime,
                                            metadata.st_mtime_nsec,
                                        ),
                                    });
                            }
                        }
                        _ => {}
                    }
                }
                directories.extend(child_directories.into_iter().rev());
            }

            Ok(collected)
        }

        #[cfg(not(unix))]
        {
            let _ = (budget, &mut is_candidate);
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "guarded local session inventory requires descriptor-relative directory access",
            ))
        }
    }

    pub(crate) fn collect_regular_files_in_directory(
        &self,
        directory_path: &Path,
        max_files: usize,
        budget: &mut LocalSessionInventoryBudget,
    ) -> io::Result<GuardedLocalSessionInventory> {
        #[cfg(unix)]
        {
            use rustix::fs::{openat, statat, AtFlags, Dir, FileType, Mode, OFlags};

            let mut inventory = GuardedLocalSessionInventory {
                files: Vec::new(),
                truncated: false,
            };
            if max_files == 0 || !budget.claim_directory() {
                inventory.truncated = true;
                return Ok(inventory);
            }

            let relative = directory_path.strip_prefix(&self.path).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "session sidecar directory is outside the guarded root",
                )
            })?;
            let directory_flags =
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
            let mut directory = openat(&self.directory, ".", directory_flags, Mode::empty())
                .map_err(io::Error::from)?;
            for component in relative.components() {
                match component {
                    Component::Normal(name) => {
                        directory = openat(&directory, name, directory_flags, Mode::empty())
                            .map_err(io::Error::from)?;
                    }
                    Component::CurDir => {}
                    Component::RootDir | Component::Prefix(_) | Component::ParentDir => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "session sidecar directory contains an unsupported component",
                        ));
                    }
                }
            }

            let mut selected = BinaryHeap::with_capacity(max_files);
            let mut entries = Dir::read_from(&directory).map_err(io::Error::from)?;
            for entry in &mut entries {
                let entry = entry.map_err(io::Error::from)?;
                let name_bytes = entry.file_name().to_bytes();
                if matches!(name_bytes, b"." | b"..") {
                    continue;
                }
                if !budget.claim_entry() {
                    inventory.truncated = true;
                    break;
                }
                let name = std::ffi::OsStr::from_bytes(name_bytes);
                let metadata = match statat(&directory, name, AtFlags::SYMLINK_NOFOLLOW) {
                    Ok(metadata) => metadata,
                    Err(_) => continue,
                };
                if FileType::from_raw_mode(metadata.st_mode) == FileType::RegularFile {
                    let path = directory_path.join(name);
                    if selected.len() < max_files {
                        selected.push(path);
                    } else {
                        inventory.truncated = true;
                        if selected.peek().is_some_and(|largest| path < *largest) {
                            selected.pop();
                            selected.push(path);
                        }
                    }
                }
            }
            inventory.files = selected.into_vec();
            inventory.files.sort();
            Ok(inventory)
        }

        #[cfg(not(unix))]
        {
            let _ = (directory_path, max_files, budget);
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "guarded local session sidecar reads require descriptor-relative access",
            ))
        }
    }

    #[cfg(unix)]
    pub(crate) fn open_regular_file(&self, path: &Path) -> io::Result<(fs::File, fs::Metadata)> {
        use rustix::fs::{openat, Mode, OFlags};

        let relative = path.strip_prefix(&self.path).map_err(|_| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "session candidate is outside the guarded root",
            )
        })?;
        let mut components = relative.components().peekable();
        if components.peek().is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "session candidate does not name a file",
            ));
        }

        let directory_flags =
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let file_flags =
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC;
        let mut parent: Option<OwnedFd> = None;
        while let Some(component) = components.next() {
            let Component::Normal(name) = component else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "session candidate contains an unsupported component",
                ));
            };
            let parent_fd = parent
                .as_ref()
                .map_or_else(|| self.directory.as_fd(), AsFd::as_fd);
            if components.peek().is_some() {
                parent = Some(
                    openat(parent_fd, name, directory_flags, Mode::empty())
                        .map_err(io::Error::from)?,
                );
                continue;
            }

            let descriptor =
                openat(parent_fd, name, file_flags, Mode::empty()).map_err(io::Error::from)?;
            let file = fs::File::from(descriptor);
            let metadata = file.metadata()?;
            if !metadata.is_file() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "session candidate is not a regular file",
                ));
            }
            return Ok((file, metadata));
        }
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "session candidate does not name a file",
        ))
    }

    #[cfg(not(unix))]
    pub(crate) fn open_regular_file(&self, _path: &Path) -> io::Result<(fs::File, fs::Metadata)> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "guarded local session reads require descriptor-relative file access",
        ))
    }
}

impl LocalSessionIoContext {
    pub(crate) fn new(limits: LocalSessionReadLimits) -> Self {
        Self {
            budget: LocalSessionReadBudget::new(limits.max_preview_read_bytes),
            inventory_budget: LocalSessionInventoryBudget::new(
                limits.max_inventory_directories,
                limits.max_inventory_entries,
            ),
            cache: LocalSessionRequestCache::default(),
            #[cfg(test)]
            primary_paths_read: Vec::new(),
            limits,
        }
    }
}

pub(crate) fn read_bounded_text(
    root: &GuardedLocalSessionRoot,
    path: &Path,
    spec: BoundedReadSpec,
    budget: &mut LocalSessionReadBudget,
) -> Result<BoundedText, ServiceError> {
    let (mut file, metadata) = root.open_regular_file(path)?;
    let mut bounded = read_bounded_from(&mut file, metadata.len(), spec, budget)?;
    bounded.modified_at_millis = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_millis()).ok());
    Ok(bounded)
}

pub(crate) fn read_bounded_sidecar_text(
    root: &GuardedLocalSessionRoot,
    path: &Path,
    spec: BoundedReadSpec,
    session_budget: &mut SessionSidecarBudget,
    request_budget: &mut LocalSessionReadBudget,
) -> Result<BoundedText, ServiceError> {
    let (mut file, metadata) = root.open_regular_file(path)?;
    let mut bounded = {
        #[cfg(all(test, unix))]
        {
            if let Some(bytes_before_error) = take_scheduled_sidecar_read_fault(path) {
                let mut fault = ScheduledReadFault {
                    inner: &mut file,
                    bytes_before_error,
                };
                read_bounded_sidecar_from(
                    &mut fault,
                    metadata.len(),
                    spec,
                    session_budget,
                    request_budget,
                )?
            } else {
                read_bounded_sidecar_from(
                    &mut file,
                    metadata.len(),
                    spec,
                    session_budget,
                    request_budget,
                )?
            }
        }
        #[cfg(not(all(test, unix)))]
        {
            read_bounded_sidecar_from(
                &mut file,
                metadata.len(),
                spec,
                session_budget,
                request_budget,
            )?
        }
    };
    bounded.modified_at_millis = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_millis()).ok());
    Ok(bounded)
}

fn read_bounded_sidecar_from<R: Read + Seek>(
    reader: &mut R,
    file_len: u64,
    spec: BoundedReadSpec,
    session_budget: &mut SessionSidecarBudget,
    request_budget: &mut LocalSessionReadBudget,
) -> io::Result<BoundedText> {
    let allowance = session_budget
        .remaining_bytes()
        .min(request_budget.remaining_bytes());
    if allowance == 0 {
        return Err(io::Error::other(
            "local session sidecar byte budget exhausted",
        ));
    }
    if !session_budget.claim_bytes_exact(allowance) {
        return Err(io::Error::other(
            "local session sidecar session budget reservation failed",
        ));
    }
    if !request_budget.claim_exact(allowance) {
        session_budget.refund_bytes(allowance);
        return Err(io::Error::other(
            "local session sidecar request budget reservation failed",
        ));
    }

    let mut reserved_budget = LocalSessionReadBudget::new(allowance);
    let result = read_bounded_from(reader, file_len, spec, &mut reserved_budget);
    if let Ok(bounded) = &result {
        let unused = allowance.saturating_sub(bounded.bytes_read);
        session_budget.refund_bytes(unused);
        request_budget.refund(unused);
    }
    result
}

fn read_bounded_from<R: Read + Seek>(
    reader: &mut R,
    file_len: u64,
    spec: BoundedReadSpec,
    budget: &mut LocalSessionReadBudget,
) -> io::Result<BoundedText> {
    reader.seek(SeekFrom::Start(0))?;
    let head_available = usize_from_u64(file_len).min(spec.head_bytes);
    let head_grant = budget.claim(head_available);
    let mut request_budget_exhausted = head_grant < head_available;
    let head_raw = read_window(reader, head_grant)?;
    let head_end = head_raw.len() as u64;

    let desired_tail_window = spec.tail_bytes.saturating_add(spec.line_fragment_bytes);
    let tail_available = usize_from_u64(file_len.saturating_sub(head_end));
    let tail_request = desired_tail_window.min(tail_available);
    let tail_grant = budget.claim(tail_request);
    request_budget_exhausted |= tail_grant < tail_request;
    let tail_start = file_len.saturating_sub(tail_grant as u64).max(head_end);
    reader.seek(SeekFrom::Start(tail_start))?;
    let tail_raw = read_window(reader, tail_grant)?;

    let decoded_head = decode_utf8_window(&head_raw, false);
    let retained_head_end = decoded_head.end_offset as u64;
    let head = decoded_head.text;
    let retained_tail = retain_tail_window(&tail_raw, spec.tail_bytes);
    let retained_tail_raw_start = tail_start.saturating_add(retained_tail.start_offset as u64);
    let decoded_tail = decode_utf8_window(
        retained_tail.bytes,
        retained_tail_raw_start > 0 && !retained_tail.starts_at_line_boundary,
    );
    let retained_tail_start =
        retained_tail_raw_start.saturating_add(decoded_tail.start_offset as u64);
    let retained_tail_end = retained_tail_raw_start.saturating_add(decoded_tail.end_offset as u64);
    let tail = decoded_tail.text;
    let retained_end = retained_head_end.max(retained_tail_end);
    let truncated = retained_head_end < retained_tail_start || retained_end < file_len;
    let tail_starts_at_line_boundary = retained_tail_start == 0
        || (retained_tail.starts_at_line_boundary && decoded_tail.start_offset == 0);
    let observed_tail_prefix_end = usize_from_u64(
        retained_tail_start
            .saturating_sub(tail_start)
            .min(tail_raw.len() as u64),
    );
    let observed_tail_prefix_has_line_break = tail_raw[..observed_tail_prefix_end].contains(&b'\n');
    let unread_gap_len = usize_from_u64(tail_start.saturating_sub(head_end));
    let head_fragment_start = head_raw
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |newline| newline + 1);
    let raw_head_fragment = &head_raw[head_fragment_start..];
    let head_fragment = if head_fragment_start == 0 {
        raw_head_fragment
            .strip_prefix(&[0xef, 0xbb, 0xbf])
            .unwrap_or(raw_head_fragment)
    } else {
        raw_head_fragment
    };
    let mut provenance_scanner = TopLevelProvenanceScanner::new();
    provenance_scanner.feed(head_fragment);
    let should_scan_gap = unread_gap_len <= spec.line_fragment_bytes;
    let gap_budget_granted = !should_scan_gap || budget.claim_exact(unread_gap_len);
    request_budget_exhausted |= should_scan_gap && !gap_budget_granted;
    let (unread_gap_has_line_break, provenance_bytes_read) =
        if should_scan_gap && gap_budget_granted {
            reader.seek(SeekFrom::Start(head_end))?;
            let scan = scan_window_for_line_break(reader, unread_gap_len, &mut provenance_scanner)?;
            (
                scan.has_line_break || scan.bytes_read != unread_gap_len,
                scan.bytes_read,
            )
        } else {
            (true, 0)
        };
    let gap_stays_on_same_line = !observed_tail_prefix_has_line_break
        && !unread_gap_has_line_break
        && !tail_starts_at_line_boundary;
    let record_provenance = if gap_stays_on_same_line {
        let record_end = tail_raw
            .iter()
            .position(|byte| *byte == b'\n')
            .unwrap_or(tail_raw.len());
        provenance_scanner.feed(&tail_raw[..record_end]);
        provenance_scanner.finish()
    } else {
        None
    };
    let bytes_read = head_raw
        .len()
        .saturating_add(tail_raw.len())
        .saturating_add(provenance_bytes_read);

    Ok(BoundedText {
        head,
        tail,
        retained_head_end,
        retained_tail_start,
        tail_starts_at_line_boundary,
        gap_stays_on_same_line,
        record_provenance,
        modified_at_millis: None,
        truncated,
        request_budget_exhausted,
        bytes_read,
    })
}

struct LineBreakScan {
    has_line_break: bool,
    bytes_read: usize,
}

fn scan_window_for_line_break<R: Read>(
    reader: &mut R,
    limit: usize,
    provenance: &mut TopLevelProvenanceScanner,
) -> io::Result<LineBreakScan> {
    let mut buffer = [0_u8; 8 * 1024];
    let mut remaining = limit;
    let mut has_line_break = false;
    let mut record_ended = false;
    while remaining > 0 {
        let requested = remaining.min(buffer.len());
        let count = reader.read(&mut buffer[..requested])?;
        if count == 0 {
            break;
        }
        if !record_ended {
            if let Some(newline) = buffer[..count].iter().position(|byte| *byte == b'\n') {
                provenance.feed(&buffer[..newline]);
                record_ended = true;
            } else {
                provenance.feed(&buffer[..count]);
            }
        }
        has_line_break |= buffer[..count].contains(&b'\n');
        remaining -= count;
    }
    Ok(LineBreakScan {
        has_line_break,
        bytes_read: limit - remaining,
    })
}

fn read_window<R: Read>(reader: &mut R, limit: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(limit);
    let mut buffer = vec![0_u8; limit.min(MAX_READ_CHUNK_BYTES)];
    let mut remaining = limit;
    while remaining > 0 {
        let requested = remaining.min(buffer.len());
        let count = reader.read(&mut buffer[..requested])?;
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..count]);
        remaining -= count;
    }
    Ok(bytes)
}

fn retain_tail_window(bytes: &[u8], tail_bytes: usize) -> RetainedTailWindow<'_> {
    if tail_bytes == 0 || bytes.is_empty() {
        return RetainedTailWindow {
            bytes: &[],
            start_offset: 0,
            starts_at_line_boundary: false,
        };
    }
    if bytes.len() <= tail_bytes {
        return RetainedTailWindow {
            bytes,
            start_offset: 0,
            starts_at_line_boundary: false,
        };
    }

    let minimum_start = bytes.len() - tail_bytes;
    if minimum_start > 0 && bytes[minimum_start - 1] == b'\n' {
        return RetainedTailWindow {
            bytes: &bytes[minimum_start..],
            start_offset: minimum_start,
            starts_at_line_boundary: true,
        };
    }
    if let Some(relative_newline) = bytes[minimum_start..bytes.len().saturating_sub(1)]
        .iter()
        .position(|byte| *byte == b'\n')
    {
        let newline = minimum_start + relative_newline;
        let start_offset = newline + 1;
        return RetainedTailWindow {
            bytes: &bytes[start_offset..],
            start_offset,
            starts_at_line_boundary: true,
        };
    }
    RetainedTailWindow {
        bytes: &bytes[minimum_start..],
        start_offset: minimum_start,
        starts_at_line_boundary: false,
    }
}

fn decode_utf8_window(bytes: &[u8], trim_leading_partial: bool) -> DecodedUtf8Window {
    let start_offset = if trim_leading_partial {
        bytes
            .iter()
            .take(3)
            .take_while(|byte| **byte & 0b1100_0000 == 0b1000_0000)
            .count()
    } else {
        0
    };
    let end_offset = start_offset + complete_utf8_end(&bytes[start_offset..]);
    let text = String::from_utf8_lossy(&bytes[start_offset..end_offset]).into_owned();
    DecodedUtf8Window {
        text,
        start_offset,
        end_offset,
    }
}

fn complete_utf8_end(bytes: &[u8]) -> usize {
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        match std::str::from_utf8(&bytes[cursor..]) {
            Ok(_) => return bytes.len(),
            Err(error) => {
                let invalid_start = cursor + error.valid_up_to();
                let Some(invalid_len) = error.error_len() else {
                    return invalid_start;
                };
                cursor = invalid_start.saturating_add(invalid_len);
            }
        }
    }
    bytes.len()
}

fn usize_from_u64(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::{
        cell::Cell,
        collections::HashMap,
        io::{self, Cursor, Read, Seek, SeekFrom},
    };

    fn candidate(path: &str, modified_at: i64) -> LocalSessionFileCandidate {
        LocalSessionFileCandidate {
            path: PathBuf::from(path),
            modified_at,
        }
    }

    fn candidate_paths(candidates: Vec<LocalSessionFileCandidate>) -> Vec<PathBuf> {
        candidates
            .into_iter()
            .map(|candidate| candidate.path)
            .collect()
    }

    #[test]
    fn inventory_selects_newest_candidates_independent_of_input_order() {
        let candidates = vec![
            candidate("old.jsonl", 100),
            candidate("new.jsonl", 300),
            candidate("middle.jsonl", 200),
        ];

        assert_eq!(
            candidate_paths(select_newest_candidates(candidates, 2)),
            vec![PathBuf::from("new.jsonl"), PathBuf::from("middle.jsonl")]
        );
    }

    #[test]
    fn inventory_uses_lexical_path_order_to_break_modified_time_ties() {
        let candidates = vec![
            candidate("zeta.jsonl", 300),
            candidate("alpha.jsonl", 300),
            candidate("middle.jsonl", 300),
        ];

        assert_eq!(
            candidate_paths(select_newest_candidates(candidates, 2)),
            vec![PathBuf::from("alpha.jsonl"), PathBuf::from("middle.jsonl")]
        );
    }

    #[cfg(unix)]
    #[test]
    fn stat_timestamp_accepts_unsigned_nanoseconds_and_saturates() {
        assert_eq!(stat_modified_at_millis(1_i64, 999_999_999_u64), 1_999);
        assert_eq!(stat_modified_at_millis(i64::MAX, 0_u64), i64::MAX);
    }

    #[cfg(not(unix))]
    #[test]
    fn guarded_session_roots_are_explicitly_unsupported_off_unix() {
        let error = match GuardedLocalSessionRoot::open(Path::new(r"C:\session-root")) {
            Ok(_) => panic!("non-Unix guarded roots must remain unavailable"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    }

    #[test]
    fn codex_index_cache_loads_once_per_store() {
        let loads = Cell::new(0usize);
        let root = PathBuf::from("/canonical/.codex");
        let mut cache = LocalSessionRequestCache::default();

        let first = cache.codex_titles_or_load(root.clone(), || {
            loads.set(loads.get() + 1);
            HashMap::from([("session-1".to_string(), "Cached title".to_string())])
        });
        assert_eq!(
            first.get("session-1").map(String::as_str),
            Some("Cached title")
        );

        let second = cache.codex_titles_or_load(root, || {
            loads.set(loads.get() + 1);
            HashMap::new()
        });
        assert_eq!(
            second.get("session-1").map(String::as_str),
            Some("Cached title")
        );
        assert_eq!(loads.get(), 1);
    }

    #[cfg(unix)]
    fn bounded_file_test_spec() -> BoundedReadSpec {
        BoundedReadSpec {
            head_bytes: 256,
            tail_bytes: 0,
            line_fragment_bytes: 0,
        }
    }

    #[cfg(unix)]
    fn guarded_reader_fixture_root(case: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};

        static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
        let unique = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        PathBuf::from("/tmp").join(format!("sc-guard-{case}-{}-{unique}", std::process::id()))
    }

    #[cfg(unix)]
    #[test]
    fn inventory_counts_all_candidates_before_newest_selection() {
        let fixture = guarded_reader_fixture_root("inventory-count");
        let root = fixture.join("authorized");
        fs::create_dir_all(&root).expect("create inventory root");
        for index in 0..5 {
            fs::write(root.join(format!("record-{index}.jsonl")), b"{}")
                .expect("write inventory candidate");
        }
        let guarded_root = GuardedLocalSessionRoot::open(&root).expect("open guarded root");
        let mut budget = LocalSessionInventoryBudget::new(1, 5);

        let collected = guarded_root
            .collect_regular_files(&mut budget, |_| true)
            .expect("collect inventory");

        assert_eq!(collected.inventory.total_candidate_count, 5);
        assert_eq!(collected.inventory.candidates.len(), 5);
        assert!(!collected.inventory.truncated);
        assert_eq!(budget.remaining_directories(), 0);
        fs::remove_dir_all(fixture).expect("remove fixture");
    }

    #[cfg(unix)]
    #[test]
    fn inventory_marks_entry_budget_exhaustion_as_truncated() {
        let fixture = guarded_reader_fixture_root("inventory-entry-budget");
        let root = fixture.join("authorized");
        fs::create_dir_all(&root).expect("create inventory root");
        for index in 0..5 {
            fs::write(root.join(format!("record-{index}.jsonl")), b"{}")
                .expect("write inventory candidate");
        }
        let guarded_root = GuardedLocalSessionRoot::open(&root).expect("open guarded root");
        let mut budget = LocalSessionInventoryBudget::new(1, 3);

        let collected = guarded_root
            .collect_regular_files(&mut budget, |_| true)
            .expect("collect bounded inventory");

        assert_eq!(collected.inventory.total_candidate_count, 3);
        assert!(collected.inventory.truncated);
        assert_eq!(budget.remaining_entries(), 0);
        fs::remove_dir_all(fixture).expect("remove fixture");
    }

    #[cfg(unix)]
    #[test]
    fn sidecar_directory_materialization_stops_at_request_entry_budget() {
        let fixture = guarded_reader_fixture_root("sidecar-entry-budget");
        let root = fixture.join("authorized");
        let sidecar_directory = root.join("message/session-1");
        fs::create_dir_all(&sidecar_directory).expect("create sidecar directory");
        for index in (0..16).rev() {
            fs::write(
                sidecar_directory.join(format!("message-{index:02}.json")),
                b"{}",
            )
            .expect("write sidecar fixture");
        }
        let guarded_root = GuardedLocalSessionRoot::open(&root).expect("open guarded root");
        let sidecar_directory = guarded_root.path().join("message/session-1");
        let limits = LocalSessionReadLimits {
            max_inventory_directories: 1,
            max_inventory_entries: 3,
            ..LocalSessionReadLimits::default()
        };
        let mut io = LocalSessionIoContext::new(limits);

        let inventory = guarded_root
            .collect_regular_files_in_directory(
                &sidecar_directory,
                limits.max_sidecar_files,
                &mut io.inventory_budget,
            )
            .expect("collect bounded sidecars");

        assert!(inventory.truncated);
        assert!(inventory.files.len() <= 3, "{:#?}", inventory.files);
        assert_eq!(io.inventory_budget.remaining_entries(), 0);

        fs::remove_dir_all(fixture).expect("remove fixture");
    }

    #[cfg(unix)]
    #[test]
    fn sidecar_directory_traversal_stops_at_request_directory_budget() {
        let fixture = guarded_reader_fixture_root("sidecar-directory-budget");
        let root = fixture.join("authorized");
        fs::create_dir_all(root.join("message/session-1")).expect("create first directory");
        fs::create_dir_all(root.join("message/session-2")).expect("create second directory");
        fs::write(root.join("message/session-2/message.json"), b"{}")
            .expect("write second-directory sidecar");
        let guarded_root = GuardedLocalSessionRoot::open(&root).expect("open guarded root");
        let limits = LocalSessionReadLimits {
            max_inventory_directories: 1,
            max_inventory_entries: 100,
            ..LocalSessionReadLimits::default()
        };
        let mut io = LocalSessionIoContext::new(limits);

        let first = guarded_root
            .collect_regular_files_in_directory(
                &guarded_root.path().join("message/session-1"),
                limits.max_sidecar_files,
                &mut io.inventory_budget,
            )
            .expect("collect first directory");
        let second = guarded_root
            .collect_regular_files_in_directory(
                &guarded_root.path().join("message/session-2"),
                limits.max_sidecar_files,
                &mut io.inventory_budget,
            )
            .expect("bound second directory");

        assert!(!first.truncated);
        assert!(second.truncated);
        assert!(second.files.is_empty());
        assert_eq!(io.inventory_budget.remaining_directories(), 0);

        fs::remove_dir_all(fixture).expect("remove fixture");
    }

    #[cfg(unix)]
    #[test]
    fn bounded_reader_rejects_a_checked_file_swapped_to_an_outside_symlink() {
        use std::os::unix::fs::symlink;

        let fixture = guarded_reader_fixture_root("final-symlink-swap");
        let root = fixture.join("authorized");
        let candidate = root.join("record.jsonl");
        let parked = root.join("record.parked");
        let outside = fixture.join("outside.jsonl");
        fs::create_dir_all(&root).expect("create authorized root");
        fs::write(&candidate, "user: SAFE_INSIDE\n").expect("write inside file");
        fs::write(&outside, "user: OUTSIDE_FINAL_SYMLINK_MUST_NOT_SURFACE\n")
            .expect("write outside file");
        let guarded_root =
            GuardedLocalSessionRoot::open(&root.canonicalize().expect("canonical authorized root"))
                .expect("open guarded root");
        let checked_candidate = candidate
            .canonicalize()
            .expect("canonical checked candidate");
        fs::rename(&candidate, parked).expect("park checked candidate");
        symlink(&outside, &candidate).expect("replace candidate with outside symlink");

        let mut budget = LocalSessionReadBudget::new(1_024);
        let result = read_bounded_text(
            &guarded_root,
            &checked_candidate,
            bounded_file_test_spec(),
            &mut budget,
        );
        let _ = fs::remove_dir_all(&fixture);

        let error = result.expect_err("swapped final symlink must be rejected");
        let message = error.to_string();
        assert!(!message.contains("OUTSIDE_FINAL_SYMLINK_MUST_NOT_SURFACE"));
        assert!(message.len() <= 160, "unbounded error: {message}");
    }

    #[cfg(unix)]
    #[test]
    fn bounded_reader_carries_modified_time_from_the_opened_descriptor() {
        use std::fs::{FileTimes, OpenOptions};
        use std::os::unix::fs::symlink;
        use std::time::{Duration, UNIX_EPOCH};

        let fixture = guarded_reader_fixture_root("descriptor-mtime");
        let root = fixture.join("authorized");
        fs::create_dir_all(&root).expect("create authorized root");
        let root = root.canonicalize().expect("canonical authorized root");
        let candidate = root.join("record.jsonl");
        let parked = root.join("record.parked");
        let outside = fixture.join("outside.jsonl");
        fs::write(&candidate, "user: SAFE_DESCRIPTOR_MTIME\n").expect("write checked file");
        fs::write(&outside, "user: OUTSIDE_MTIME\n").expect("write outside file");
        let safe_modified = UNIX_EPOCH + Duration::from_secs(1_600_000_000);
        let outside_modified = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        OpenOptions::new()
            .write(true)
            .open(&candidate)
            .expect("open checked file for timestamp")
            .set_times(FileTimes::new().set_modified(safe_modified))
            .expect("set checked timestamp");
        OpenOptions::new()
            .write(true)
            .open(&outside)
            .expect("open outside file for timestamp")
            .set_times(FileTimes::new().set_modified(outside_modified))
            .expect("set outside timestamp");
        let guarded_root = GuardedLocalSessionRoot::open(&root).expect("open guarded root");
        let mut budget = LocalSessionReadBudget::new(1_024);
        let bounded = read_bounded_text(
            &guarded_root,
            &candidate,
            bounded_file_test_spec(),
            &mut budget,
        )
        .expect("read checked descriptor");
        fs::rename(&candidate, parked).expect("park checked file");
        symlink(&outside, &candidate).expect("replace path with outside symlink");
        let _ = fs::remove_dir_all(&fixture);

        assert_eq!(bounded.modified_at_millis, Some(1_600_000_000_000));
    }

    #[cfg(unix)]
    #[test]
    fn bounded_reader_rejects_a_checked_parent_swapped_to_an_outside_symlink() {
        use std::os::unix::fs::symlink;

        let fixture = guarded_reader_fixture_root("parent-symlink-swap");
        let root = fixture.join("authorized");
        let live_parent = root.join("nested");
        let parked_parent = root.join("nested-parked");
        let outside_parent = fixture.join("outside");
        let candidate = live_parent.join("record.jsonl");
        fs::create_dir_all(&live_parent).expect("create live parent");
        fs::create_dir_all(&outside_parent).expect("create outside parent");
        fs::write(&candidate, "user: SAFE_INSIDE\n").expect("write inside file");
        fs::write(
            outside_parent.join("record.jsonl"),
            "user: OUTSIDE_PARENT_SYMLINK_MUST_NOT_SURFACE\n",
        )
        .expect("write outside file");
        let guarded_root =
            GuardedLocalSessionRoot::open(&root.canonicalize().expect("canonical authorized root"))
                .expect("open guarded root");
        let checked_candidate = candidate
            .canonicalize()
            .expect("canonical checked candidate");
        fs::rename(&live_parent, &parked_parent).expect("park checked parent");
        symlink(&outside_parent, &live_parent).expect("replace parent with outside symlink");

        let mut budget = LocalSessionReadBudget::new(1_024);
        let result = read_bounded_text(
            &guarded_root,
            &checked_candidate,
            bounded_file_test_spec(),
            &mut budget,
        );
        let _ = fs::remove_dir_all(&fixture);

        let error = result.expect_err("swapped parent symlink must be rejected");
        let message = error.to_string();
        assert!(!message.contains("OUTSIDE_PARENT_SYMLINK_MUST_NOT_SURFACE"));
        assert!(message.len() <= 160, "unbounded error: {message}");
    }

    #[cfg(unix)]
    #[test]
    fn bounded_reader_rejects_static_file_and_directory_symlinks() {
        use std::os::unix::fs::symlink;

        let fixture = guarded_reader_fixture_root("static-symlinks");
        let root = fixture.join("authorized");
        let outside_parent = fixture.join("outside");
        fs::create_dir_all(&root).expect("create authorized root");
        fs::create_dir_all(&outside_parent).expect("create outside parent");
        let root = root.canonicalize().expect("canonical authorized root");
        let outside = outside_parent.join("record.jsonl");
        fs::write(&outside, "user: STATIC_SYMLINK_MUST_NOT_SURFACE\n").expect("write outside file");
        let file_link = root.join("file-link.jsonl");
        let directory_link = root.join("directory-link");
        symlink(&outside, &file_link).expect("create file symlink");
        symlink(&outside_parent, &directory_link).expect("create directory symlink");
        let guarded_root = GuardedLocalSessionRoot::open(&root).expect("open guarded root");

        for candidate in [file_link, directory_link.join("record.jsonl")] {
            let mut budget = LocalSessionReadBudget::new(1_024);
            let result = read_bounded_text(
                &guarded_root,
                &candidate,
                bounded_file_test_spec(),
                &mut budget,
            );
            assert!(
                result.is_err(),
                "static symlink was followed: {candidate:?}"
            );
        }
        let _ = fs::remove_dir_all(&fixture);
    }

    #[cfg(unix)]
    #[test]
    fn bounded_reader_accepts_regular_files_and_rejects_non_regular_nodes() {
        use std::fs::OpenOptions;
        use std::os::unix::net::UnixListener;
        use std::process::Command;

        let fixture = guarded_reader_fixture_root("node-types");
        let root = fixture.join("authorized");
        fs::create_dir_all(&root).expect("create authorized root");
        let root = root.canonicalize().expect("canonical authorized root");
        let guarded_root = GuardedLocalSessionRoot::open(&root).expect("open guarded root");
        let regular = root.join("regular.jsonl");
        fs::write(&regular, "user: SAFE_REGULAR_FILE\n").expect("write regular file");
        let mut budget = LocalSessionReadBudget::new(1_024);
        let text = read_bounded_text(
            &guarded_root,
            &regular,
            bounded_file_test_spec(),
            &mut budget,
        )
        .expect("regular file accepted");
        assert!(text.head.contains("SAFE_REGULAR_FILE"));

        let socket = root.join("session.socket");
        let _listener = UnixListener::bind(&socket).expect("bind unix socket");
        let fifo = root.join("session.fifo");
        let status = Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("run mkfifo");
        assert!(status.success(), "mkfifo failed: {status}");
        let _fifo_keeper = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&fifo)
            .expect("open fifo keeper without blocking");

        for candidate in [root.clone(), socket, fifo] {
            let mut budget = LocalSessionReadBudget::new(1_024);
            let result = read_bounded_text(
                &guarded_root,
                &candidate,
                bounded_file_test_spec(),
                &mut budget,
            );
            assert!(
                result.is_err(),
                "non-regular node was accepted: {candidate:?}"
            );
        }
        let device_root =
            GuardedLocalSessionRoot::open(Path::new("/dev")).expect("open guarded device root");
        let mut budget = LocalSessionReadBudget::new(1_024);
        assert!(
            read_bounded_text(
                &device_root,
                Path::new("/dev/null"),
                bounded_file_test_spec(),
                &mut budget,
            )
            .is_err(),
            "device node was accepted"
        );
        let _ = fs::remove_dir_all(&fixture);
    }

    #[test]
    fn bounded_reader_reads_disjoint_head_and_tail_windows() {
        let mut input = b"head\n".to_vec();
        input.extend(std::iter::repeat_n(b'm', 128));
        input.extend_from_slice(b"\ntail\n");
        let mut reader = Cursor::new(input.clone());
        let mut budget = LocalSessionReadBudget::new(128);

        let text = read_bounded_from(
            &mut reader,
            input.len() as u64,
            BoundedReadSpec {
                head_bytes: 5,
                tail_bytes: 6,
                line_fragment_bytes: 8,
            },
            &mut budget,
        )
        .expect("bounded read");

        assert_eq!(text.head, "head\n");
        assert_eq!(text.tail, "tail\n");
        assert!(text.truncated);
        assert!(text.bytes_read <= 19, "read {} bytes", text.bytes_read);
    }

    #[test]
    fn bounded_reader_verifies_a_small_unread_gap_stays_on_one_line() {
        let input = b"HEAD?abcdefghijklTAIL\n".to_vec();
        let mut reader = Cursor::new(input.clone());
        let mut budget = LocalSessionReadBudget::new(128);

        let text = read_bounded_from(
            &mut reader,
            input.len() as u64,
            BoundedReadSpec {
                head_bytes: 5,
                tail_bytes: 5,
                line_fragment_bytes: 8,
            },
            &mut budget,
        )
        .expect("bounded read");

        assert_eq!(text.head, "HEAD?");
        assert_eq!(text.tail, "TAIL\n");
        assert!(text.gap_stays_on_same_line);
        assert!(text.bytes_read <= 22, "read {} bytes", text.bytes_read);
    }

    #[test]
    fn bounded_reader_provenance_uses_last_root_classification_scalars() {
        let input = format!(
            concat!(
                "{{\"type\":\"user\",\"role\":\"assistant\",",
                "\"timestamp\":\"old\",\"data\":\"{}\",",
                "\"type\":\"file-history-snapshot\",\"role\":\"user\",",
                "\"timestamp\":\"new\"}}\n"
            ),
            "x".repeat(96)
        )
        .into_bytes();
        let mut reader = Cursor::new(input.clone());
        let mut budget = LocalSessionReadBudget::new(input.len());

        let text = read_bounded_from(
            &mut reader,
            input.len() as u64,
            BoundedReadSpec {
                head_bytes: 72,
                tail_bytes: 48,
                line_fragment_bytes: 128,
            },
            &mut budget,
        )
        .expect("bounded read");
        let mut fields = serde_json::Map::new();
        fields.insert("type".to_string(), Value::String("user".to_string()));
        fields.insert("role".to_string(), Value::String("assistant".to_string()));
        fields.insert("timestamp".to_string(), Value::String("old".to_string()));

        text.record_provenance
            .as_ref()
            .expect("complete spanning record provenance")
            .merge_into(&mut fields);

        assert_eq!(
            fields.get("type").and_then(Value::as_str),
            Some("file-history-snapshot")
        );
        assert_eq!(fields.get("role").and_then(Value::as_str), Some("user"));
        assert_eq!(fields.get("timestamp").and_then(Value::as_str), Some("new"));
    }

    #[test]
    fn provenance_requires_complete_valid_json_syntax() {
        for invalid in [
            br#"{"type":"user","data":{xxxxx}}"#.as_slice(),
            br#"{"type":"user","data":[xxxxx]}"#.as_slice(),
            br#"{"type":"user","data":{"value":1,}}"#.as_slice(),
            br#"{"type":"user",}"#.as_slice(),
            br#"{"type":"user","data":{"value" 1}}"#.as_slice(),
            br#"{"type":"user","data":{"value":}}"#.as_slice(),
            br#"{"type":"user","data":[1 2]}"#.as_slice(),
            br#"{"type":"user","data":[,]}"#.as_slice(),
            br#"{"type":"user","data":01}"#.as_slice(),
            br#"{"type":"user","data":+1}"#.as_slice(),
            br#"{"type":"user","data":"\q"}"#.as_slice(),
            br#"{"type":"user","data":"\uD800"}"#.as_slice(),
            br#"{"type":"user"} trailing"#.as_slice(),
            b"{\"type\"\x0b:\"user\"}".as_slice(),
            b"{\"type\":\"user\"\x0c}".as_slice(),
        ] {
            let mut scanner = TopLevelProvenanceScanner::new();
            scanner.feed(invalid);

            assert!(
                scanner.finish().is_none(),
                "invalid JSON must not establish provenance: {}",
                String::from_utf8_lossy(invalid)
            );
        }

        for valid in [
            br#"{"type":"user","data":{"value":1}}"#.as_slice(),
            br#"{"type":"user","data":[true,false,null,-1.5e2]}"#.as_slice(),
            br#"{"t\u0079pe":"user","data":{"nested":[{},[]]},"text":"\uD834\uDD1E"}"#.as_slice(),
        ] {
            let mut scanner = TopLevelProvenanceScanner::new();
            scanner.feed(valid);

            assert!(
                scanner.finish().is_some(),
                "valid JSON should establish provenance: {}",
                String::from_utf8_lossy(valid)
            );
        }
    }

    #[test]
    fn provenance_applies_last_supported_scalar_including_null() {
        let mut scanner = TopLevelProvenanceScanner::new();
        scanner
            .feed(br#"{"type":"user","role":"user","text":"stale","data":"ignored","text":null}"#);
        let provenance = scanner.finish().expect("complete object provenance");
        let mut fields = serde_json::Map::new();
        fields.insert("text".to_string(), Value::String("stale".to_string()));

        provenance.merge_into(&mut fields);

        assert_eq!(fields.get("text"), Some(&Value::Null));
    }

    #[test]
    fn bounded_reader_rejects_a_line_break_inside_the_small_unread_gap() {
        let input = b"HEAD?ab\ndefghijklTAIL\n".to_vec();
        let mut reader = Cursor::new(input.clone());
        let mut budget = LocalSessionReadBudget::new(128);

        let text = read_bounded_from(
            &mut reader,
            input.len() as u64,
            BoundedReadSpec {
                head_bytes: 5,
                tail_bytes: 5,
                line_fragment_bytes: 8,
            },
            &mut budget,
        )
        .expect("bounded read");

        assert!(!text.gap_stays_on_same_line);
        assert!(text.bytes_read <= 23, "read {} bytes", text.bytes_read);
    }

    #[test]
    fn bounded_reader_does_not_scan_an_unread_gap_larger_than_the_fragment_limit() {
        let input = b"HEAD?abcdefghijklmnopqrstTAIL\n".to_vec();
        let mut reader = Cursor::new(input.clone());
        let mut budget = LocalSessionReadBudget::new(128);

        let text = read_bounded_from(
            &mut reader,
            input.len() as u64,
            BoundedReadSpec {
                head_bytes: 5,
                tail_bytes: 5,
                line_fragment_bytes: 8,
            },
            &mut budget,
        )
        .expect("bounded read");

        assert!(!text.gap_stays_on_same_line);
        assert_eq!(text.bytes_read, 18);
    }

    #[test]
    fn bounded_reader_uses_retained_ranges_after_tail_alignment() {
        let input = b"HEAD?drop\nTAIL\n".to_vec();
        let mut reader = Cursor::new(input.clone());
        let mut budget = LocalSessionReadBudget::new(64);

        let text = read_bounded_from(
            &mut reader,
            input.len() as u64,
            BoundedReadSpec {
                head_bytes: 5,
                tail_bytes: 6,
                line_fragment_bytes: 8,
            },
            &mut budget,
        )
        .expect("bounded read");

        assert_eq!(text.head, "HEAD?");
        assert_eq!(text.tail, "TAIL\n");
        assert_eq!(text.retained_head_end, 5);
        assert_eq!(text.retained_tail_start, 10);
        assert!(text.tail_starts_at_line_boundary);
        assert!(text.truncated);
    }

    #[test]
    fn bounded_reader_confirms_boundary_immediately_before_tail_cap() {
        for (input, expected_tail_start) in [
            (b"HEAD?drop\nTAIL\n".as_slice(), 10),
            (b"HEAD?drop\r\nTAIL\n".as_slice(), 11),
        ] {
            let mut reader = Cursor::new(input.to_vec());
            let mut budget = LocalSessionReadBudget::new(64);

            let text = read_bounded_from(
                &mut reader,
                input.len() as u64,
                BoundedReadSpec {
                    head_bytes: 5,
                    tail_bytes: 5,
                    line_fragment_bytes: 8,
                },
                &mut budget,
            )
            .expect("bounded read");

            assert_eq!(text.tail, "TAIL\n");
            assert_eq!(text.retained_tail_start, expected_tail_start);
            assert!(text.tail_starts_at_line_boundary);
            assert!(text.truncated);
        }
    }

    #[test]
    fn bounded_reader_never_exceeds_request_byte_budget() {
        let mut reader = RecordingReadSeek::new(1024 * 1024);
        let mut budget = LocalSessionReadBudget::new(80);

        let text = read_bounded_from(
            &mut reader,
            1024 * 1024,
            BoundedReadSpec {
                head_bytes: 64,
                tail_bytes: 32,
                line_fragment_bytes: 16,
            },
            &mut budget,
        )
        .expect("bounded read");

        assert!(text.bytes_read <= 80, "read {} bytes", text.bytes_read);
        assert_eq!(budget.remaining_bytes, 0);
        assert!(text.head.len() <= 64);
        assert!(text.tail.len() <= 32);
    }

    #[test]
    fn bounded_reader_does_not_duplicate_overlapping_windows() {
        let input = b"short\n".to_vec();
        let mut reader = Cursor::new(input.clone());
        let mut budget = LocalSessionReadBudget::new(64);

        let text = read_bounded_from(
            &mut reader,
            input.len() as u64,
            BoundedReadSpec {
                head_bytes: 4,
                tail_bytes: 4,
                line_fragment_bytes: 4,
            },
            &mut budget,
        )
        .expect("bounded read");

        assert_eq!(format!("{}{}", text.head, text.tail), "short\n");
        assert!(!text.truncated);
        assert_eq!(text.bytes_read, input.len());
    }

    #[test]
    fn bounded_reader_shares_budget_across_files() {
        let spec = BoundedReadSpec {
            head_bytes: 32,
            tail_bytes: 0,
            line_fragment_bytes: 0,
        };
        let mut budget = LocalSessionReadBudget::new(48);
        let mut first = RecordingReadSeek::new(100);
        let mut second = RecordingReadSeek::new(100);

        let first_text =
            read_bounded_from(&mut first, 100, spec, &mut budget).expect("first bounded read");
        let second_text =
            read_bounded_from(&mut second, 100, spec, &mut budget).expect("second bounded read");

        assert_eq!(first_text.bytes_read, 32);
        assert_eq!(second_text.bytes_read, 16);
        assert_eq!(budget.remaining_bytes, 0);
    }

    #[test]
    fn failed_sidecar_read_does_not_release_session_or_request_allowance() {
        let spec = BoundedReadSpec {
            head_bytes: 8,
            tail_bytes: 0,
            line_fragment_bytes: 0,
        };
        let mut session_budget = SessionSidecarBudget::new(2, 8);
        let mut request_budget = LocalSessionReadBudget::new(8);
        let mut fault = BytesThenErrorReadSeek::new(3);

        let error = read_bounded_sidecar_from(
            &mut fault,
            8,
            spec,
            &mut session_budget,
            &mut request_budget,
        )
        .expect_err("the injected read fault must propagate");

        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(fault.bytes_returned, 3);
        assert_eq!(session_budget.remaining_bytes(), 0);
        assert_eq!(request_budget.remaining_bytes(), 0);

        let mut later = RecordingReadSeek::new(8);
        assert!(read_bounded_sidecar_from(
            &mut later,
            8,
            spec,
            &mut session_budget,
            &mut request_budget,
        )
        .is_err());
        assert_eq!(later.max_requested, 0);
    }

    #[test]
    fn bounded_reader_keeps_utf8_boundaries_valid() {
        let input = format!("开始{}结束", "x".repeat(128)).into_bytes();
        let mut reader = Cursor::new(input.clone());
        let mut budget = LocalSessionReadBudget::new(16);

        let text = read_bounded_from(
            &mut reader,
            input.len() as u64,
            BoundedReadSpec {
                head_bytes: 4,
                tail_bytes: 4,
                line_fragment_bytes: 0,
            },
            &mut budget,
        )
        .expect("bounded read");

        assert_eq!(text.head, "开");
        assert_eq!(text.tail, "束");
        assert!(!text.head.contains('\u{fffd}'));
        assert!(!text.tail.contains('\u{fffd}'));
    }

    #[test]
    fn bounded_reader_replaces_interior_invalid_utf8_without_changing_raw_ranges() {
        let input = b"before\n\xff\nafter\n".to_vec();
        let mut reader = Cursor::new(input.clone());
        let mut budget = LocalSessionReadBudget::new(input.len());

        let text = read_bounded_from(
            &mut reader,
            input.len() as u64,
            BoundedReadSpec {
                head_bytes: input.len(),
                tail_bytes: 0,
                line_fragment_bytes: 0,
            },
            &mut budget,
        )
        .expect("bounded read");

        assert!(text.head.contains("before"), "{}", text.head);
        assert!(text.head.contains("after"), "{}", text.head);
        assert!(text.head.contains('\u{fffd}'), "{}", text.head);
        assert_eq!(text.retained_head_end, input.len() as u64);
        assert!(!text.truncated);
    }

    #[test]
    fn bounded_reader_replaces_interior_invalid_utf8_in_aligned_tail() {
        let input = b"HEAD?drop\nBEFORE\n\xff\nAFTER\n".to_vec();
        let mut reader = Cursor::new(input.clone());
        let mut budget = LocalSessionReadBudget::new(input.len());

        let text = read_bounded_from(
            &mut reader,
            input.len() as u64,
            BoundedReadSpec {
                head_bytes: 5,
                tail_bytes: 15,
                line_fragment_bytes: 8,
            },
            &mut budget,
        )
        .expect("bounded read");

        assert!(text.tail.contains("BEFORE"), "{}", text.tail);
        assert!(text.tail.contains("AFTER"), "{}", text.tail);
        assert!(text.tail.contains('\u{fffd}'), "{}", text.tail);
        assert_eq!(text.retained_tail_start, 10);
        assert!(text.tail_starts_at_line_boundary);
    }

    #[test]
    fn bounded_reader_handles_one_line_larger_than_both_windows() {
        let len = 8 * 1024 * 1024;
        let mut reader = RecordingReadSeek::new(len);
        let mut budget = LocalSessionReadBudget::new(1_024);

        let text = read_bounded_from(
            &mut reader,
            len,
            BoundedReadSpec {
                head_bytes: 64,
                tail_bytes: 32,
                line_fragment_bytes: 16,
            },
            &mut budget,
        )
        .expect("bounded read");

        assert!(text.truncated);
        assert!(text.head.len() + text.tail.len() <= 96);
        assert!(reader.max_requested <= 64 * 1024);
    }

    struct RecordingReadSeek {
        len: u64,
        position: u64,
        max_requested: usize,
    }

    struct BytesThenErrorReadSeek {
        bytes_before_error: usize,
        bytes_returned: usize,
        position: u64,
    }

    impl BytesThenErrorReadSeek {
        fn new(bytes_before_error: usize) -> Self {
            Self {
                bytes_before_error,
                bytes_returned: 0,
                position: 0,
            }
        }
    }

    impl Read for BytesThenErrorReadSeek {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if self.bytes_returned >= self.bytes_before_error {
                return Err(io::Error::other("injected sidecar read fault"));
            }
            let count = (self.bytes_before_error - self.bytes_returned).min(buffer.len());
            buffer[..count].fill(b'x');
            self.bytes_returned += count;
            self.position += count as u64;
            Ok(count)
        }
    }

    impl Seek for BytesThenErrorReadSeek {
        fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
            let next = match position {
                SeekFrom::Start(offset) => i128::from(offset),
                SeekFrom::End(offset) => 8_i128 + i128::from(offset),
                SeekFrom::Current(offset) => i128::from(self.position) + i128::from(offset),
            };
            if !(0..=8).contains(&next) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "seek outside injected reader",
                ));
            }
            self.position = next as u64;
            Ok(self.position)
        }
    }

    impl RecordingReadSeek {
        fn new(len: u64) -> Self {
            Self {
                len,
                position: 0,
                max_requested: 0,
            }
        }
    }

    impl Read for RecordingReadSeek {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.max_requested = self.max_requested.max(buf.len());
            let remaining = self.len.saturating_sub(self.position) as usize;
            let count = remaining.min(buf.len());
            buf[..count].fill(b'x');
            self.position += count as u64;
            Ok(count)
        }
    }

    impl Seek for RecordingReadSeek {
        fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
            let next = match position {
                SeekFrom::Start(offset) => i128::from(offset),
                SeekFrom::End(offset) => i128::from(self.len) + i128::from(offset),
                SeekFrom::Current(offset) => i128::from(self.position) + i128::from(offset),
            };
            if !(0..=i128::from(self.len)).contains(&next) {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid seek"));
            }
            self.position = next as u64;
            Ok(self.position)
        }
    }
}
