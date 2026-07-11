use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ListSourceCompleteness {
    Enumerable,
    Limited,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ListIncompleteReason {
    SafetyBudget,
    SourceChanged,
    SourceLimited,
    UnreadableSource,
    PageFailed,
    UnsupportedProtocol,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListPageMetadata {
    pub returned_count: usize,
    pub total_count: Option<usize>,
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub source_completeness: ListSourceCompleteness,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incomplete_reason: Option<ListIncompleteReason>,
}

impl ListPageMetadata {
    pub fn enumerable(
        returned_count: usize,
        total_count: Option<usize>,
        next_cursor: Option<String>,
    ) -> Self {
        Self {
            returned_count,
            total_count,
            has_more: next_cursor.is_some(),
            next_cursor,
            source_completeness: ListSourceCompleteness::Enumerable,
            incomplete_reason: None,
        }
    }

    pub fn incomplete(
        returned_count: usize,
        total_count: Option<usize>,
        reason: ListIncompleteReason,
    ) -> Self {
        Self {
            returned_count,
            total_count,
            has_more: false,
            next_cursor: None,
            source_completeness: ListSourceCompleteness::Limited,
            incomplete_reason: Some(reason),
        }
    }

    pub fn validate(&self, returned_len: usize) -> Result<(), &'static str> {
        if self.returned_count != returned_len {
            return Err("returned_count does not match rows");
        }
        if self.has_more
            && self.source_completeness == ListSourceCompleteness::Enumerable
            && self.next_cursor.is_none()
        {
            return Err("enumerable page with more rows requires next_cursor");
        }
        if !self.has_more && self.next_cursor.is_some() {
            return Err("terminal page cannot expose next_cursor");
        }
        Ok(())
    }
}
