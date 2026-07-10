use super::ServiceError;
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
    pub(crate) truncated: bool,
    pub(crate) bytes_read: usize,
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

    let bytes_read = head_raw.len().saturating_add(tail_raw.len());
    let covered_end = tail_start.saturating_add(tail_raw.len() as u64);
    let truncated = head_end < tail_start || covered_end < file_len;
    let head = valid_utf8_prefix(&head_raw).to_string();
    let retained_tail = retain_tail_window(&tail_raw, spec.tail_bytes);
    let tail = valid_utf8_window(retained_tail).to_string();

    Ok(BoundedText {
        head,
        tail,
        truncated,
        bytes_read,
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

fn retain_tail_window(bytes: &[u8], tail_bytes: usize) -> &[u8] {
    if tail_bytes == 0 || bytes.is_empty() {
        return &[];
    }
    if bytes.len() <= tail_bytes {
        return bytes;
    }

    let minimum_start = bytes.len() - tail_bytes;
    if let Some(relative_newline) = bytes[minimum_start..bytes.len().saturating_sub(1)]
        .iter()
        .position(|byte| *byte == b'\n')
    {
        let newline = minimum_start + relative_newline;
        let candidate = &bytes[newline + 1..];
        return candidate;
    }
    &bytes[minimum_start..]
}

fn valid_utf8_prefix(bytes: &[u8]) -> &str {
    match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => std::str::from_utf8(&bytes[..error.valid_up_to()]).unwrap_or_default(),
    }
}

fn valid_utf8_window(bytes: &[u8]) -> &str {
    for start in 0..=bytes.len().min(3) {
        let candidate = &bytes[start..];
        match std::str::from_utf8(candidate) {
            Ok(text) => return text,
            Err(error) if error.error_len().is_none() => {
                return std::str::from_utf8(&candidate[..error.valid_up_to()]).unwrap_or_default();
            }
            Err(_) => {}
        }
    }
    ""
}

fn usize_from_u64(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
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
