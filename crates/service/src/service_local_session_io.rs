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
    pub(crate) retained_head_end: u64,
    pub(crate) retained_tail_start: u64,
    pub(crate) tail_starts_at_line_boundary: bool,
    pub(crate) truncated: bool,
    pub(crate) bytes_read: usize,
}

struct RetainedTailWindow<'a> {
    bytes: &'a [u8],
    start_offset: usize,
    starts_at_line_boundary: bool,
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
    let head = valid_utf8_prefix(&head_raw).to_string();
    let retained_head_end = head.len() as u64;
    let retained_tail = retain_tail_window(&tail_raw, spec.tail_bytes);
    let (tail_utf8_offset, tail_text) = valid_utf8_window(retained_tail.bytes);
    let retained_tail_start = tail_start
        .saturating_add(retained_tail.start_offset as u64)
        .saturating_add(tail_utf8_offset as u64);
    let tail = tail_text.to_string();
    let retained_tail_end = retained_tail_start.saturating_add(tail.len() as u64);
    let retained_end = retained_head_end.max(retained_tail_end);
    let truncated = retained_head_end < retained_tail_start || retained_end < file_len;
    let tail_starts_at_line_boundary = retained_tail_start == 0
        || (retained_tail.starts_at_line_boundary && tail_utf8_offset == 0);

    Ok(BoundedText {
        head,
        tail,
        retained_head_end,
        retained_tail_start,
        tail_starts_at_line_boundary,
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

fn valid_utf8_prefix(bytes: &[u8]) -> &str {
    match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => std::str::from_utf8(&bytes[..error.valid_up_to()]).unwrap_or_default(),
    }
}

fn valid_utf8_window(bytes: &[u8]) -> (usize, &str) {
    for start in 0..=bytes.len().min(3) {
        let candidate = &bytes[start..];
        match std::str::from_utf8(candidate) {
            Ok(text) => return (start, text),
            Err(error) if error.error_len().is_none() => {
                return (
                    start,
                    std::str::from_utf8(&candidate[..error.valid_up_to()]).unwrap_or_default(),
                );
            }
            Err(_) => {}
        }
    }
    (bytes.len(), "")
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
