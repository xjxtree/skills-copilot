pub(crate) mod shared;

pub mod claude_code;
pub mod codex;
pub mod hermes;
pub mod openclaw;
pub mod opencode;
pub mod pi;

pub use claude_code::ClaudeCodeAdapter;
pub use codex::{
    codex_home_dir, codex_plugin_cache_id, parse_codex_enabled_plugin_ids,
    parse_codex_skill_config_entries, CodexAdapter, CodexSkillConfigEntry,
};
pub use hermes::{hermes_disabled_skill_names, HermesAdapter};
pub use openclaw::{
    openclaw_config_key_from_frontmatter, openclaw_disabled_skill_keys, OpenclawAdapter,
};
pub use opencode::OpencodeAdapter;
pub use pi::{pi_disabled_skill_names, PiAdapter};

#[cfg(test)]
pub(crate) fn assert_parse_equivalent(
    adapter: &dyn skills_copilot_core::AgentAdapter,
    fixture: &std::path::Path,
) {
    let content = std::fs::read_to_string(fixture).expect("read fixture");
    let from_path = adapter.parse(fixture).expect("path parse");
    let from_content = adapter
        .parse_content(fixture, content)
        .expect("content parse");

    assert_eq!(from_content.name, from_path.name);
    assert_eq!(from_content.description, from_path.description);
    assert_eq!(from_content.frontmatter_raw, from_path.frontmatter_raw);
    assert_eq!(from_content.body, from_path.body);
    assert_eq!(from_content.state, from_path.state);
    assert_eq!(from_content.enabled, from_path.enabled);
    assert_eq!(from_content.permissions, from_path.permissions);
}
