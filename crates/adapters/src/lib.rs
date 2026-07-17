pub(crate) mod shared;

pub mod claude_code;
pub mod codex;
mod environment;
pub mod hermes;
pub mod openclaw;
pub mod opencode;
pub mod pi;

pub use claude_code::{claude_config_dir, ClaudeCodeAdapter};
pub use codex::{
    codex_home_dir, codex_plugin_cache_id, parse_codex_enabled_plugin_ids,
    parse_codex_skill_config_entries, CodexAdapter, CodexSkillConfigEntry,
};
pub use hermes::{hermes_disabled_skill_names, hermes_home_dir, HermesAdapter};
pub use openclaw::{
    openclaw_config_key_from_frontmatter, openclaw_config_path, openclaw_disabled_skill_keys,
    openclaw_state_dir, OpenclawAdapter,
};
pub use opencode::{
    opencode_data_dir, opencode_user_config_path, opencode_user_skills_dir, OpencodeAdapter,
};
pub use pi::{pi_agent_dir, pi_skill_enabled_by_settings, PiAdapter};

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
