use super::ServiceError;
use serde_json::{Map, Value};
use std::{
    fs,
    io::{self, Read, Seek, SeekFrom},
    path::Path,
};

const MAX_READ_CHUNK_BYTES: usize = 64 * 1024;

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
    pub(crate) truncated: bool,
    pub(crate) bytes_read: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct BoundedRecordProvenance {
    record_type: ScalarProvenance,
    role: ScalarProvenance,
    timestamp: ScalarProvenance,
}

impl BoundedRecordProvenance {
    pub(crate) fn merge_into(&self, fields: &mut Map<String, Value>) {
        self.record_type.merge_into(fields, "type");
        self.role.merge_into(fields, "role");
        self.timestamp.merge_into(fields, "timestamp");
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
}

const MAX_PROVENANCE_TOKEN_BYTES: usize = 4 * 1024;
const MAX_PROVENANCE_NESTING: usize = 128;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ProvenanceKey {
    RecordType,
    Role,
    Timestamp,
    Other,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum RootParseState {
    Start,
    KeyOrEnd,
    Colon,
    Value,
    Primitive,
    AfterValue,
    Complete,
    Invalid,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum StringPurpose {
    RootKey,
    RootValue(ProvenanceKey),
    Nested,
}

struct TopLevelProvenanceScanner {
    state: RootParseState,
    stack: Vec<u8>,
    current_key: ProvenanceKey,
    string_purpose: Option<StringPurpose>,
    string_token: Vec<u8>,
    string_token_overflowed: bool,
    escaped: bool,
    unicode_escape_remaining: u8,
    utf8_remaining: u8,
    utf8_next_min: u8,
    utf8_next_max: u8,
    primitive_token: Vec<u8>,
    primitive_key: ProvenanceKey,
    provenance: BoundedRecordProvenance,
}

impl TopLevelProvenanceScanner {
    fn new() -> Self {
        Self {
            state: RootParseState::Start,
            stack: Vec::with_capacity(16),
            current_key: ProvenanceKey::Other,
            string_purpose: None,
            string_token: Vec::with_capacity(64),
            string_token_overflowed: false,
            escaped: false,
            unicode_escape_remaining: 0,
            utf8_remaining: 0,
            utf8_next_min: 0x80,
            utf8_next_max: 0xbf,
            primitive_token: Vec::with_capacity(32),
            primitive_key: ProvenanceKey::Other,
            provenance: BoundedRecordProvenance {
                record_type: ScalarProvenance::Missing,
                role: ScalarProvenance::Missing,
                timestamp: ScalarProvenance::Missing,
            },
        }
    }

    fn feed(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.feed_byte(*byte);
        }
    }

    fn finish(mut self) -> Option<BoundedRecordProvenance> {
        if self.state == RootParseState::Primitive {
            self.finish_primitive();
        }
        (self.state == RootParseState::Complete
            && self.stack.is_empty()
            && self.string_purpose.is_none())
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
        if self.stack.len() > 1 {
            self.feed_nested_byte(byte);
            return;
        }

        let mut pending = Some(byte);
        while let Some(byte) = pending.take() {
            match self.state {
                RootParseState::Start => {
                    if byte.is_ascii_whitespace() {
                        continue;
                    }
                    if byte == b'{' {
                        self.stack.push(byte);
                        self.state = RootParseState::KeyOrEnd;
                    } else {
                        self.invalidate();
                    }
                }
                RootParseState::KeyOrEnd => {
                    if byte.is_ascii_whitespace() {
                        continue;
                    }
                    match byte {
                        b'"' => self.begin_string(StringPurpose::RootKey),
                        b'}' => {
                            self.stack.pop();
                            self.state = RootParseState::Complete;
                        }
                        _ => self.invalidate(),
                    }
                }
                RootParseState::Colon => {
                    if byte.is_ascii_whitespace() {
                        continue;
                    }
                    if byte == b':' {
                        self.state = RootParseState::Value;
                    } else {
                        self.invalidate();
                    }
                }
                RootParseState::Value => {
                    if byte.is_ascii_whitespace() {
                        continue;
                    }
                    match byte {
                        b'"' => self.begin_string(StringPurpose::RootValue(self.current_key)),
                        b'{' | b'[' => {
                            self.set_current_field(ScalarProvenance::Unsupported);
                            self.push_nested(byte);
                        }
                        b',' | b'}' | b']' => self.invalidate(),
                        _ => {
                            self.primitive_token.clear();
                            self.primitive_key = self.current_key;
                            self.push_primitive_byte(byte);
                            self.state = RootParseState::Primitive;
                        }
                    }
                }
                RootParseState::Primitive => {
                    if byte.is_ascii_whitespace() || matches!(byte, b',' | b'}') {
                        self.finish_primitive();
                        pending = Some(byte);
                    } else {
                        self.push_primitive_byte(byte);
                    }
                }
                RootParseState::AfterValue => {
                    if byte.is_ascii_whitespace() {
                        continue;
                    }
                    match byte {
                        b',' => self.state = RootParseState::KeyOrEnd,
                        b'}' => {
                            self.stack.pop();
                            self.state = RootParseState::Complete;
                        }
                        _ => self.invalidate(),
                    }
                }
                RootParseState::Complete => {
                    if !byte.is_ascii_whitespace() {
                        self.invalidate();
                    }
                }
                RootParseState::Invalid => {}
            }
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
        if !matches!(
            purpose,
            StringPurpose::Nested | StringPurpose::RootValue(ProvenanceKey::Other)
        ) {
            self.push_string_token_byte(b'"');
        }
    }

    fn feed_string_byte(&mut self, byte: u8) {
        let purpose = self.string_purpose.expect("string purpose");
        let capture = !matches!(
            purpose,
            StringPurpose::Nested | StringPurpose::RootValue(ProvenanceKey::Other)
        );
        if capture {
            self.push_string_token_byte(byte);
        }

        if self.unicode_escape_remaining > 0 {
            if !byte.is_ascii_hexdigit() {
                self.invalidate();
                return;
            }
            self.unicode_escape_remaining -= 1;
            return;
        }
        if self.escaped {
            self.escaped = false;
            match byte {
                b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => {}
                b'u' => self.unicode_escape_remaining = 4,
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
                StringPurpose::RootKey => self.invalidate(),
                StringPurpose::RootValue(key) => {
                    self.set_field(key, ScalarProvenance::Unsupported);
                    self.state = RootParseState::AfterValue;
                }
                StringPurpose::Nested => {}
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
                    "timestamp" => ProvenanceKey::Timestamp,
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
            StringPurpose::Nested => {}
        }
    }

    fn start_utf8_sequence(&mut self, remaining: u8, next_min: u8, next_max: u8) {
        self.utf8_remaining = remaining;
        self.utf8_next_min = next_min;
        self.utf8_next_max = next_max;
    }

    fn feed_nested_byte(&mut self, byte: u8) {
        match byte {
            b'"' => self.begin_string(StringPurpose::Nested),
            b'{' | b'[' => self.push_nested(byte),
            b'}' | b']' => {
                let expected = if byte == b'}' { b'{' } else { b'[' };
                if self.stack.last() != Some(&expected) {
                    self.invalidate();
                    return;
                }
                self.stack.pop();
                if self.stack.len() == 1 {
                    self.state = RootParseState::AfterValue;
                }
            }
            _ => {}
        }
    }

    fn push_nested(&mut self, byte: u8) {
        if self.stack.len() >= MAX_PROVENANCE_NESTING {
            self.invalidate();
            return;
        }
        self.stack.push(byte);
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
        if self.state == RootParseState::Invalid {
            return;
        }
        let Ok(value) = serde_json::from_slice::<Value>(&self.primitive_token) else {
            self.invalidate();
            return;
        };
        if matches!(value, Value::Array(_) | Value::Object(_) | Value::String(_)) {
            self.invalidate();
            return;
        }
        self.set_field(self.primitive_key, ScalarProvenance::Scalar(value));
        self.state = RootParseState::AfterValue;
    }

    fn set_current_field(&mut self, value: ScalarProvenance) {
        self.set_field(self.current_key, value);
    }

    fn set_field(&mut self, key: ProvenanceKey, value: ScalarProvenance) {
        match key {
            ProvenanceKey::RecordType => self.provenance.record_type = value,
            ProvenanceKey::Role => self.provenance.role = value,
            ProvenanceKey::Timestamp => self.provenance.timestamp = value,
            ProvenanceKey::Other => {}
        }
    }

    fn invalidate(&mut self) {
        self.state = RootParseState::Invalid;
        self.string_purpose = None;
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
}

pub(crate) struct LocalSessionIoContext {
    pub(crate) limits: LocalSessionReadLimits,
    pub(crate) budget: LocalSessionReadBudget,
}

impl LocalSessionIoContext {
    pub(crate) fn new(limits: LocalSessionReadLimits) -> Self {
        Self {
            budget: LocalSessionReadBudget::new(limits.max_preview_read_bytes),
            limits,
        }
    }
}

pub(crate) fn read_bounded_text(
    path: &Path,
    spec: BoundedReadSpec,
    budget: &mut LocalSessionReadBudget,
) -> Result<BoundedText, ServiceError> {
    let mut file = fs::File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(
            io::Error::new(io::ErrorKind::InvalidInput, "session path is not a file").into(),
        );
    }
    Ok(read_bounded_from(&mut file, metadata.len(), spec, budget)?)
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
    let head_raw = read_window(reader, head_grant)?;
    let head_end = head_raw.len() as u64;

    let desired_tail_window = spec.tail_bytes.saturating_add(spec.line_fragment_bytes);
    let tail_available = usize_from_u64(file_len.saturating_sub(head_end));
    let tail_request = desired_tail_window.min(tail_available);
    let tail_grant = budget.claim(tail_request);
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
    let head_fragment = head_raw[head_fragment_start..]
        .strip_prefix(&[0xef, 0xbb, 0xbf])
        .unwrap_or(&head_raw[head_fragment_start..]);
    let mut provenance_scanner = TopLevelProvenanceScanner::new();
    provenance_scanner.feed(head_fragment);
    let (unread_gap_has_line_break, provenance_bytes_read) =
        if unread_gap_len <= spec.line_fragment_bytes && budget.claim_exact(unread_gap_len) {
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
        truncated,
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
    use std::io::{self, Cursor, Read, Seek, SeekFrom};

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
