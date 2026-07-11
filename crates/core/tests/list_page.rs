use skills_copilot_core::{ListIncompleteReason, ListPageMetadata, ListSourceCompleteness};

#[test]
fn page_metadata_requires_returned_count_to_match_rows() {
    let page = ListPageMetadata::enumerable(2, Some(5), Some("v1:next".into()));
    assert_eq!(page.validate(1), Err("returned_count does not match rows"));
}

#[test]
fn enumerable_more_page_requires_cursor() {
    let page = ListPageMetadata {
        returned_count: 2,
        total_count: Some(5),
        has_more: true,
        next_cursor: None,
        source_completeness: ListSourceCompleteness::Enumerable,
        incomplete_reason: None,
    };
    assert_eq!(
        page.validate(2),
        Err("enumerable page with more rows requires next_cursor")
    );
}

#[test]
fn source_limited_page_is_honest_without_cursor() {
    let page = ListPageMetadata::incomplete(8, None, ListIncompleteReason::SourceLimited);
    assert_eq!(page.validate(8), Ok(()));
    assert_eq!(page.source_completeness, ListSourceCompleteness::Limited);
}
