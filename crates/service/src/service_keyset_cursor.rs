use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct KeysetCursor {
    pub(crate) version: u8,
    pub(crate) method: String,
    pub(crate) query_digest: String,
    pub(crate) source_revision: String,
    pub(crate) sort_value: i64,
    pub(crate) stable_id: String,
    pub(crate) tie_breaker_digest: Option<String>,
}

pub(crate) fn encode_cursor(value: &KeysetCursor) -> Result<String, ServiceError> {
    let bytes = serde_json::to_vec(value)?;
    Ok(format!("v1:{}", lowercase_hex(&bytes)))
}

pub(crate) fn decode_cursor(
    text: &str,
    method: &str,
    query_digest: &str,
) -> Result<KeysetCursor, ServiceError> {
    let encoded = text.strip_prefix("v1:").ok_or_else(|| {
        ServiceError::InvalidRequest("cursor must use the v1 encoding".to_string())
    })?;
    let bytes = decode_lowercase_hex(encoded)?;
    let cursor: KeysetCursor = serde_json::from_slice(&bytes)
        .map_err(|_| ServiceError::InvalidRequest("cursor payload is invalid".to_string()))?;
    if cursor.version != 1 || cursor.method != method || cursor.query_digest != query_digest {
        return Err(ServiceError::InvalidRequest(
            "cursor does not match this list query".to_string(),
        ));
    }
    Ok(cursor)
}

fn lowercase_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

fn decode_lowercase_hex(text: &str) -> Result<Vec<u8>, ServiceError> {
    if !text.len().is_multiple_of(2)
        || text
            .bytes()
            .any(|byte| !matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(ServiceError::InvalidRequest(
            "cursor must contain lowercase hexadecimal".to_string(),
        ));
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| Ok((hex_value(pair[0]) << 4) | hex_value(pair[1])))
        .collect()
}

fn hex_value(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!("validated lowercase hexadecimal"),
    }
}
