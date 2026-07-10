#![no_main]

use libfuzzer_sys::fuzz_target;
use skills_copilot_adapters::ClaudeCodeAdapter;
use skills_copilot_core::AgentAdapter;

fuzz_target!(|data: &[u8]| {
    let Ok(content) = std::str::from_utf8(data) else {
        return;
    };

    let Ok(temp_dir) = tempfile::Builder::new()
        .prefix("skills-copilot-frontmatter-fuzz-")
        .tempdir()
    else {
        return;
    };
    let skill_path = temp_dir.path().join("SKILL.md");
    if std::fs::write(&skill_path, content).is_err() {
        return;
    }

    let _ = ClaudeCodeAdapter.parse(&skill_path);
    if let Ok(content) = String::from_utf8(data.to_vec()) {
        let _ = ClaudeCodeAdapter.parse_content(&skill_path, content);
    }
});
