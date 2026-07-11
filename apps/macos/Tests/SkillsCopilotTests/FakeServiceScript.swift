import Darwin
import Foundation
@testable import SkillsCopilot

final class FakeServiceScript: ServiceProcessRunning {
    private static let processGate = FakeServiceProcessGate()

    private let directory: URL
    let executableURL: URL
    private let stateURL: URL
    private let callRecorder = FakeServiceCallRecorder()
    private let delayedConfigSaveReleaseURL: URL
    private let delayedProviderSaveAReleaseURL: URL
    private let delayedProviderSaveBReleaseURL: URL
    private let responseReleaseURL: URL
    private let scenarioLock = NSLock()
    private var currentScenario = "normal"

    init() throws {
        directory = URL(fileURLWithPath: NSTemporaryDirectory())
            .appendingPathComponent("skills-copilot-fake-service-\(UUID().uuidString)", isDirectory: true)
        executableURL = directory.appendingPathComponent("fake-service.sh")
        stateURL = directory.appendingPathComponent("method-state.log")
        delayedConfigSaveReleaseURL = directory.appendingPathComponent("release-config-save-a")
        delayedProviderSaveAReleaseURL = directory.appendingPathComponent("release-provider-save-a")
        delayedProviderSaveBReleaseURL = directory.appendingPathComponent("release-provider-save-b")
        responseReleaseURL = directory.appendingPathComponent("release-response")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        FileManager.default.createFile(atPath: stateURL.path, contents: nil)
        try script.write(to: executableURL, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes(
            [.posixPermissions: NSNumber(value: Int16(0o755))],
            ofItemAtPath: executableURL.path
        )
    }

    func activate(scenario: String) {
        setScenario(scenario)
    }

    func setScenario(_ scenario: String) {
        scenarioLock.lock()
        currentScenario = scenario
        scenarioLock.unlock()
    }

    func cleanup() {
        try? FileManager.default.removeItem(at: directory)
    }

    @discardableResult
    func releaseDelayedConfigSave() -> Bool {
        FileManager.default.createFile(atPath: delayedConfigSaveReleaseURL.path, contents: Data())
    }

    @discardableResult
    func releaseDelayedProviderSaveA() -> Bool {
        FileManager.default.createFile(atPath: delayedProviderSaveAReleaseURL.path, contents: Data())
    }

    @discardableResult
    func releaseDelayedProviderSaveB() -> Bool {
        FileManager.default.createFile(atPath: delayedProviderSaveBReleaseURL.path, contents: Data())
    }

    func releaseBlockedResponse() {
        FileManager.default.createFile(atPath: responseReleaseURL.path, contents: nil)
    }

    func serviceClient() -> ServiceClient {
        ServiceClient(processRunner: self, serviceURL: executableURL)
    }

    func run(executableURL: URL, input: Data, timeoutNanoseconds: UInt64?) async throws -> Data {
        callRecorder.record(input, methodStateURL: stateURL)
        return try await Self.processGate.run(
            executableURL: self.executableURL,
            input: input,
            timeoutNanoseconds: timeoutNanoseconds,
            environmentOverrides: [
                "SKILLS_COPILOT_FAKE_SERVICE_SCENARIO": scenario,
                "SKILLS_COPILOT_FAKE_SERVICE_CALLS": stateURL.path,
                "SKILLS_COPILOT_FAKE_CONFIG_RELEASE": delayedConfigSaveReleaseURL.path,
                "SKILLS_COPILOT_FAKE_PROVIDER_A_RELEASE": delayedProviderSaveAReleaseURL.path,
                "SKILLS_COPILOT_FAKE_PROVIDER_B_RELEASE": delayedProviderSaveBReleaseURL.path,
                "SKILLS_COPILOT_FAKE_SERVICE_RESPONSE_RELEASE": responseReleaseURL.path
            ]
        )
    }

    func calls() -> String {
        callRecorder.calls()
    }

    private var scenario: String {
        scenarioLock.lock()
        let value = currentScenario
        scenarioLock.unlock()
        return value
    }

    private var script: String {
        """
        #!/bin/sh
        input=$(cat)
        scenario=${SKILLS_COPILOT_FAKE_SERVICE_SCENARIO:-normal}

        protocol_version=2
        if [ "$scenario" = "protocol-v1-bindings" ]; then
          protocol_version=1
        fi

        if [ -n "$SKILLS_COPILOT_FAKE_SERVICE_CALLS" ]; then
          printf '%s\\n' "$input" >> "$SKILLS_COPILOT_FAKE_SERVICE_CALLS"
        fi
        respond() {
          printf '%s' "$1"
          exit 0
        }

        service_error() {
          respond '{"id":"test","ok":false,"result":null,"error":{"code":"test.error","message":"boom"}}'
        }

        wait_for_release() {
          release_path=$1
          attempts=0
          while [ ! -f "$release_path" ] && [ "$attempts" -lt 500 ]; do
            sleep 0.01
            attempts=$((attempts + 1))
          done
          [ -f "$release_path" ]
        }

        wait_for_response_release() {
          while [ ! -f "$SKILLS_COPILOT_FAKE_SERVICE_RESPONSE_RELEASE" ]; do
            sleep 0.01
          done
        }

        status_response() {
          respond '{"id":"test","ok":true,"result":{"protocol_version":'"$protocol_version"', "version":"test","app_data_dir":"/tmp/skills-copilot","catalog_path":"/tmp/skills-copilot/catalog.sqlite","user_home":"/tmp/home","supported_methods":["app.stateSnapshot","service.status","catalog.listSkills","catalog.scanAll","catalog.getSkill","catalog.listFindings","catalog.listConflicts","skill.listEvents","snapshot.list","snapshot.listAgentConfig","snapshot.previewRollback","snapshot.rollback","config.toggleSkill","config.readAgentConfig","config.readClaudeSettings","config.saveClaudeSettings","batch.previewSkillToggles","batch.applySkillToggles","project.getContext","project.setContext","project.clearContext","project.validateContext"],"adapter_capabilities":'"$adapter_capabilities"'}}'
        }

        adapter_capabilities='[{"agent":"claude-code","display_name":"Claude Code","status":"verified","scan":{"supported":true,"status":"verified","reason":null},"project_scan":{"supported":true,"status":"verified","reason":null},"config_toggle":{"supported":true,"status":"verified","reason":null},"config_snapshot":{"supported":true,"status":"verified","reason":null},"install":{"supported":true,"status":"verified","reason":null},"writable":{"supported":true,"status":"verified","reason":null},"blockers":[]},{"agent":"codex","display_name":"Codex","status":"verified","scan":{"supported":true,"status":"verified","reason":null},"project_scan":{"supported":true,"status":"verified","reason":null},"config_toggle":{"supported":true,"status":"verified","reason":null},"config_snapshot":{"supported":true,"status":"verified","reason":null},"install":{"supported":false,"status":"planned","reason":"Install is not part of this slice."},"writable":{"supported":true,"status":"verified","reason":null},"blockers":[]},{"agent":"opencode","display_name":"opencode","status":"verified","scan":{"supported":true,"status":"verified","reason":null},"project_scan":{"supported":true,"status":"verified","reason":null},"config_toggle":{"supported":true,"status":"verified","reason":null},"config_snapshot":{"supported":true,"status":"verified","reason":null},"install":{"supported":true,"status":"verified","reason":null},"writable":{"supported":true,"status":"verified","reason":null},"blockers":[]},{"agent":"pi","display_name":"Pi","status":"read-only","scan":{"supported":true,"status":"verified","reason":null},"project_scan":{"supported":true,"status":"verified","reason":null},"config_toggle":{"supported":false,"status":"read-only","reason":"Pi writable support is blocked pending evidence."},"config_snapshot":{"supported":false,"status":"read-only","reason":"Pi is read-only."},"install":{"supported":false,"status":"read-only","reason":"Pi is read-only."},"writable":{"supported":false,"status":"read-only","reason":"Pi is read-only."},"blockers":["Pi writable support is blocked pending evidence."]},{"agent":"hermes","display_name":"Hermes","status":"read-only","scan":{"supported":true,"status":"verified","reason":null},"project_scan":{"supported":false,"status":"read-only","reason":"Hermes project skills are not confirmed."},"config_toggle":{"supported":false,"status":"read-only","reason":"Hermes is read-only."},"config_snapshot":{"supported":false,"status":"read-only","reason":"Hermes is read-only."},"install":{"supported":false,"status":"read-only","reason":"Hermes is read-only."},"writable":{"supported":false,"status":"read-only","reason":"Hermes is read-only."},"blockers":["Hermes is read-only."]},{"agent":"openclaw","display_name":"OpenClaw","status":"read-only","scan":{"supported":true,"status":"verified","reason":null},"project_scan":{"supported":true,"status":"verified","reason":null},"config_toggle":{"supported":false,"status":"read-only","reason":"OpenClaw is read-only."},"config_snapshot":{"supported":false,"status":"read-only","reason":"OpenClaw is read-only."},"install":{"supported":false,"status":"read-only","reason":"OpenClaw is read-only."},"writable":{"supported":false,"status":"read-only","reason":"OpenClaw is read-only."},"blockers":["OpenClaw is read-only."]}]'
        project_active='{"id":"project-1","name":"Fixture Project","root_path":"/tmp/project","current_cwd":"/tmp/project","last_used_at":"2026-06-08T00:00:00Z","is_active":true,"validation_error":null}'
        project_recent='[{"id":"project-1","name":"Fixture Project","root_path":"/tmp/project","current_cwd":"/tmp/project","last_used_at":"2026-06-08T00:00:00Z","is_active":true,"validation_error":null},{"id":"project-2","name":"Other Project","root_path":"/tmp/other","current_cwd":"/tmp/other","last_used_at":"2026-06-07T00:00:00Z","is_active":false,"validation_error":null}]'
        project_invalid='{"id":"project-missing","name":"Missing Project","root_path":"/tmp/missing","current_cwd":"/tmp/missing","last_used_at":"2026-06-08T00:00:00Z","is_active":true,"validation_error":"Project root does not exist."}'

        skills_normal='[{"id":"alpha","agent":"claude-code","scope":"agent-global","path":"/tmp/global/alpha/SKILL.md","display_path":"/tmp/global/alpha/SKILL.md","definition_id":"def.alpha","name":"Alpha","state":"loaded","enabled":true},{"id":"beta","agent":"claude-code","scope":"agent-project","path":"/tmp/project/beta/SKILL.md","display_path":"/tmp/project/beta/SKILL.md","definition_id":"def.beta","name":"Beta","state":"loaded","enabled":true},{"id":"gamma","agent":"codex","scope":"agent-global","path":"/tmp/codex/skills/gamma/SKILL.md","display_path":"~/.codex/skills/gamma/SKILL.md","definition_id":"codex:gamma","name":"Gamma","state":"loaded","enabled":true}]'
        skills_toggled='[{"id":"alpha","agent":"claude-code","scope":"agent-global","path":"/tmp/global/alpha/SKILL.md","display_path":"/tmp/global/alpha/SKILL.md","definition_id":"def.alpha","name":"Alpha","state":"loaded","enabled":true},{"id":"beta","agent":"claude-code","scope":"agent-project","path":"/tmp/project/beta/SKILL.md","display_path":"/tmp/project/beta/SKILL.md","definition_id":"def.beta","name":"Beta","state":"loaded","enabled":false},{"id":"gamma","agent":"codex","scope":"agent-global","path":"/tmp/codex/skills/gamma/SKILL.md","display_path":"~/.codex/skills/gamma/SKILL.md","definition_id":"codex:gamma","name":"Gamma","state":"loaded","enabled":true}]'
        skills_codex_toggled='[{"id":"alpha","agent":"claude-code","scope":"agent-global","path":"/tmp/global/alpha/SKILL.md","display_path":"/tmp/global/alpha/SKILL.md","definition_id":"def.alpha","name":"Alpha","state":"loaded","enabled":true},{"id":"beta","agent":"claude-code","scope":"agent-project","path":"/tmp/project/beta/SKILL.md","display_path":"/tmp/project/beta/SKILL.md","definition_id":"def.beta","name":"Beta","state":"loaded","enabled":true},{"id":"gamma","agent":"codex","scope":"agent-global","path":"/tmp/codex/skills/gamma/SKILL.md","display_path":"~/.codex/skills/gamma/SKILL.md","definition_id":"codex:gamma","name":"Gamma","state":"loaded","enabled":false}]'
        skills_opencode='[{"id":"omega","agent":"opencode","scope":"agent-global","path":"/tmp/opencode/skills/omega/SKILL.md","display_path":"~/.config/opencode/skills/omega/SKILL.md","definition_id":"opencode:omega","name":"Omega","state":"loaded","enabled":true}]'
        skills_opencode_toggled='[{"id":"omega","agent":"opencode","scope":"agent-global","path":"/tmp/opencode/skills/omega/SKILL.md","display_path":"~/.config/opencode/skills/omega/SKILL.md","definition_id":"opencode:omega","name":"Omega","state":"disabled","enabled":false}]'
        skills_toolglobal='[{"id":"tool-alpha","agent":"tool-global","scope":"tool-global","path":"/tmp/skills-copilot/staging/tool-alpha/SKILL.md","display_path":"Tool Pool/tool-alpha/SKILL.md","definition_id":"tool:alpha","name":"Tool Alpha","state":"loaded","enabled":true}]'
        skills_batch_mixed='[{"id":"alpha","agent":"claude-code","scope":"agent-global","path":"/tmp/global/alpha/SKILL.md","display_path":"/tmp/global/alpha/SKILL.md","definition_id":"def.alpha","name":"Alpha","state":"loaded","enabled":true},{"id":"beta","agent":"claude-code","scope":"agent-project","path":"/tmp/project/beta/SKILL.md","display_path":"/tmp/project/beta/SKILL.md","definition_id":"def.beta","name":"Beta","state":"loaded","enabled":false},{"id":"gamma","agent":"codex","scope":"agent-global","path":"/tmp/codex/skills/gamma/SKILL.md","display_path":"~/.codex/skills/gamma/SKILL.md","definition_id":"codex:gamma","name":"Gamma","state":"loaded","enabled":true},{"id":"pi-one","agent":"pi","scope":"agent-global","path":"/tmp/pi/skills/pi-one/SKILL.md","display_path":"~/.pi/skills/pi-one/SKILL.md","definition_id":"pi:one","name":"Pi One","state":"loaded","enabled":true}]'
        skills_batch_applied='[{"id":"alpha","agent":"claude-code","scope":"agent-global","path":"/tmp/global/alpha/SKILL.md","display_path":"/tmp/global/alpha/SKILL.md","definition_id":"def.alpha","name":"Alpha","state":"loaded","enabled":false},{"id":"beta","agent":"claude-code","scope":"agent-project","path":"/tmp/project/beta/SKILL.md","display_path":"/tmp/project/beta/SKILL.md","definition_id":"def.beta","name":"Beta","state":"loaded","enabled":false},{"id":"gamma","agent":"codex","scope":"agent-global","path":"/tmp/codex/skills/gamma/SKILL.md","display_path":"~/.codex/skills/gamma/SKILL.md","definition_id":"codex:gamma","name":"Gamma","state":"loaded","enabled":false},{"id":"pi-one","agent":"pi","scope":"agent-global","path":"/tmp/pi/skills/pi-one/SKILL.md","display_path":"~/.pi/skills/pi-one/SKILL.md","definition_id":"pi:one","name":"Pi One","state":"loaded","enabled":true}]'
        findings_stale_before='[{"id":"finding-stale-before","instance_id":"beta","definition_id":"def.beta","rule_id":"frontmatter.required-fields","severity":"error","message":"before","suggestion":"Add missing metadata.","created_at":1}]'
        findings_stale_after_scan='[{"id":"finding-fresh-scan","instance_id":"beta","definition_id":"def.beta","rule_id":"fingerprint.changed","severity":"info","message":"scan","suggestion":"Review changed content.","created_at":2},{"id":"finding-fresh-codex","instance_id":"gamma","definition_id":"codex:gamma","rule_id":"path.outside-workspace","severity":"error","message":"codex","suggestion":"Move the skill under the project root.","created_at":3}]'
        findings_stale_after_project='[{"id":"finding-project","instance_id":"gamma","definition_id":"codex:gamma","rule_id":"name.collision","severity":"warning","message":"project","suggestion":"Review duplicate names.","created_at":4}]'
        findings_stale_after_toggle='[{"id":"finding-toggle","instance_id":"beta","definition_id":"def.beta","rule_id":"path.outside-workspace","severity":"error","message":"toggle","suggestion":"Move the skill under the project root.","created_at":5}]'
        findings_detail_scope='[{"id":"finding-beta-instance","instance_id":"beta","definition_id":"def.beta","rule_id":"fingerprint.changed","severity":"info","message":"beta instance","suggestion":"Review beta.","created_at":6},{"id":"finding-beta-definition-only","instance_id":"alpha","definition_id":"def.beta","rule_id":"name.collision","severity":"warning","message":"shared definition, wrong skill","suggestion":"Do not show on beta detail.","created_at":7},{"id":"finding-gamma-instance","instance_id":"gamma","definition_id":"codex:gamma","rule_id":"path.outside-workspace","severity":"error","message":"gamma instance","suggestion":"Review gamma.","created_at":8}]'
        conflicts_detail_scope='[{"id":"conflict-beta-alpha","definition_id":"def.beta","reason":"content-drift","winner_id":null,"instance_ids":["beta","alpha"]},{"id":"conflict-beta-gamma-cross-agent","definition_id":"def.beta","reason":"source-overlap","winner_id":null,"instance_ids":["beta","gamma"]},{"id":"conflict-alpha-gamma-no-selected","definition_id":"def.shared","reason":"source-overlap","winner_id":null,"instance_ids":["alpha","gamma"]}]'
        events_beta='[{"id":1001,"instance_id":"beta","kind":"toggle","payload":{"enabled":false,"agent":"claude-code","skill_name":"Beta"},"occurred_at":10},{"id":1000,"instance_id":"beta","kind":"scan","payload":{"summary":"rescan"},"occurred_at":9}]'
        events_gamma='[{"id":2001,"instance_id":"gamma","kind":"toggle","payload":{"enabled":true,"agent":"codex","skill_name":"Gamma"},"occurred_at":11}]'
        snapshots_claude='[{"id":"snap-claude-new","agent":"claude-code","scope":"agent-global","target":"/tmp/home/.claude/settings.json","content":"{}\\n","reason":"pre-toggle","created_at":30},{"id":"snap-claude-old","agent":"claude-code","scope":"agent-project","target":"/tmp/project/.claude/settings.local.json","content":"{}\\n","reason":"pre-config-edit","created_at":20}]'
        snapshots_codex='[{"id":"snap-codex","agent":"codex","scope":"agent-global","target":"/tmp/home/.codex/config.toml","content":"disable_response_storage = true\\n","reason":"pre-toggle","created_at":40}]'
        snapshots_opencode='[{"id":"snap-opencode","agent":"opencode","scope":"agent-global","target":"/tmp/home/.config/opencode/opencode.json","content":"{}\\n","reason":"pre-toggle","created_at":50}]'
        agent_config_claude='[{"agent":"claude-code","scope":"agent-global","target":"/tmp/home/.claude/settings.json","format":"json","content":"{\\"skillOverrides\\":{}}\\n","exists":true,"revision":"sha256:claude-global"},{"agent":"claude-code","scope":"agent-project","target":"/tmp/project/.claude/settings.local.json","format":"json","content":"{\\"permissions\\":{\\"allow\\":[\\"Bash(grep *)\\"]}}\\n","exists":true,"revision":"sha256:claude-project"}]'
        agent_config_codex='[{"agent":"codex","scope":"agent-global","target":"/tmp/home/.codex/config.toml","format":"toml","content":"model = \\"gpt-5\\"\\n","exists":true,"revision":"sha256:codex-global"},{"agent":"codex","scope":"agent-project","target":"/tmp/project/.codex/config.toml","format":"toml","content":"approval_policy = \\"never\\"\\n","exists":true,"revision":"sha256:codex-project"}]'
        agent_config_opencode='[{"agent":"opencode","scope":"agent-global","target":"/tmp/home/.config/opencode/opencode.json","format":"json","content":"{\\"permission\\":{\\"skill\\":{}}}\\n","exists":true,"revision":"sha256:opencode-global"},{"agent":"opencode","scope":"agent-project","target":"/tmp/project/opencode.json","format":"json","content":"{\\"permission\\":{\\"skill\\":{\\"local-review\\":\\"deny\\"}}}\\n","exists":true,"revision":"sha256:opencode-project"}]'
        agent_config_pi='[{"agent":"pi","scope":"agent-global","target":"/tmp/home/.pi/agent/settings.json","format":"json","content":"{\\"skills\\":{\\"disabled\\":[\\"alibabacloud-agentbay-aio-skills\\"]},\\"apiToken\\":\\"fixture-token\\"}\\n","exists":true,"revision":"sha256:pi-global"},{"agent":"pi","scope":"agent-project","target":"/tmp/project/.pi/settings.json","format":"json","content":"{\\"skills\\":{\\"disabled\\":[]}}\\n","exists":false,"revision":"sha256:pi-project"}]'
        agent_config_hermes='[{"agent":"hermes","scope":"agent-global","target":"/tmp/home/.hermes/config.yaml","format":"yaml","content":"skills:\\n  disabled: []\\n","exists":true,"revision":"sha256:hermes-global"}]'
        agent_config_openclaw='[{"agent":"openclaw","scope":"agent-global","target":"/tmp/home/.openclaw/openclaw.json","format":"json","content":"{\\"skills\\":{\\"entries\\":{}}}\\n","exists":true,"revision":"sha256:openclaw-global"}]'

        state_snapshot_response() {
          if [ "$scenario" = "error" ]; then service_error; fi
          if [ "$scenario" = "empty" ]; then
            state_skills='[]'
            state_findings='[]'
            state_conflicts='[]'
          elif [ "$scenario" = "toggle-disabled" ]; then
            state_skills=$skills_toggled
            state_findings='[]'
            state_conflicts='[]'
          elif [ "$scenario" = "toggle-codex-disabled" ]; then
            state_skills=$skills_codex_toggled
            state_findings='[]'
            state_conflicts='[]'
          elif [ "$scenario" = "opencode" ]; then
            if grep -q '"method":"config.toggleSkill"' "$SKILLS_COPILOT_FAKE_SERVICE_CALLS"; then
              state_skills=$skills_opencode_toggled
            else
              state_skills=$skills_opencode
            fi
            state_findings='[]'
            state_conflicts='[]'
          elif [ "$scenario" = "tool-global" ]; then
            state_skills=$skills_toolglobal
            state_findings='[]'
            state_conflicts='[]'
          elif [ "$scenario" = "stale-before" ]; then
            state_skills=$skills_normal
            state_findings=$findings_stale_before
            state_conflicts='[]'
          elif [ "$scenario" = "stale-after-scan" ]; then
            state_skills=$skills_normal
            state_findings=$findings_stale_after_scan
            state_conflicts='[]'
          elif [ "$scenario" = "stale-after-project" ]; then
            state_skills=$skills_normal
            state_findings=$findings_stale_after_project
            state_conflicts='[]'
          elif [ "$scenario" = "stale-after-toggle" ]; then
            state_skills=$skills_toggled
            state_findings=$findings_stale_after_toggle
            state_conflicts='[]'
          elif [ "$scenario" = "detail-scope" ]; then
            state_skills=$skills_normal
            state_findings=$findings_detail_scope
            state_conflicts=$conflicts_detail_scope
          elif [ "$scenario" = "batch-mixed" ]; then
            if grep -q '"method":"batch.applySkillToggles"' "$SKILLS_COPILOT_FAKE_SERVICE_CALLS"; then
              state_skills=$skills_batch_applied
            else
              state_skills=$skills_batch_mixed
            fi
            state_findings='[]'
            state_conflicts='[]'
          else
            state_skills=$skills_normal
            state_findings='[]'
            state_conflicts='[]'
          fi
          respond '{"id":"test","ok":true,"result":{"status":{"protocol_version":'"$protocol_version"', "version":"test","app_data_dir":"/tmp/skills-copilot","catalog_path":"/tmp/skills-copilot/catalog.sqlite","user_home":"/tmp/home","supported_methods":["app.stateSnapshot","service.status","catalog.listSkills","catalog.scanAll","catalog.getSkill","catalog.listFindings","catalog.listConflicts","skill.listEvents","snapshot.list","snapshot.listAgentConfig","snapshot.previewRollback","snapshot.rollback","config.toggleSkill","config.readAgentConfig","config.readClaudeSettings","config.saveClaudeSettings","batch.previewSkillToggles","batch.applySkillToggles","project.getContext","project.setContext","project.clearContext","project.validateContext"],"adapter_capabilities":'"$adapter_capabilities"'},"skills":'"$state_skills"',"findings":'"$state_findings"',"conflicts":'"$state_conflicts"',"snapshots":[]}}'
        }

        detail_alpha='{"id":"alpha","agent":"claude-code","scope":"agent-global","path":"/tmp/global/alpha/SKILL.md","display_path":"/tmp/global/alpha/SKILL.md","definition_id":"def.alpha","name":"Alpha","description":"Alpha skill","state":"loaded","enabled":true,"frontmatter_raw":"name: Alpha","body":"Alpha body","permissions":{"marker":"alpha"},"fingerprint":"fp-alpha"}'
        detail_beta_enabled='{"id":"beta","agent":"claude-code","scope":"agent-project","path":"/tmp/project/beta/SKILL.md","display_path":"/tmp/project/beta/SKILL.md","definition_id":"def.beta","name":"Beta","description":"Beta skill","state":"loaded","enabled":true,"frontmatter_raw":"name: Beta","body":"Beta body","permissions":{"marker":"default"},"fingerprint":"fp-beta"}'
        detail_beta_disabled='{"id":"beta","agent":"claude-code","scope":"agent-project","path":"/tmp/project/beta/SKILL.md","display_path":"/tmp/project/beta/SKILL.md","definition_id":"def.beta","name":"Beta","description":"Beta skill","state":"loaded","enabled":false,"frontmatter_raw":"name: Beta","body":"Beta body","permissions":{"marker":"toggle-disabled"},"fingerprint":"fp-beta"}'
        detail_gamma='{"id":"gamma","agent":"codex","scope":"agent-global","path":"/tmp/codex/skills/gamma/SKILL.md","display_path":"~/.codex/skills/gamma/SKILL.md","definition_id":"codex:gamma","name":"Gamma","description":"Gamma skill","state":"loaded","enabled":true,"frontmatter_raw":"name: Gamma","body":"Gamma body","permissions":{"marker":"codex"},"fingerprint":"fp-gamma"}'
        detail_gamma_disabled='{"id":"gamma","agent":"codex","scope":"agent-global","path":"/tmp/codex/skills/gamma/SKILL.md","display_path":"~/.codex/skills/gamma/SKILL.md","definition_id":"codex:gamma","name":"Gamma","description":"Gamma skill","state":"loaded","enabled":false,"frontmatter_raw":"name: Gamma","body":"Gamma body","permissions":{"marker":"codex-disabled"},"fingerprint":"fp-gamma"}'
        detail_beta_before='{"id":"beta","agent":"claude-code","scope":"agent-project","path":"/tmp/project/beta/SKILL.md","display_path":"/tmp/project/beta/SKILL.md","definition_id":"def.beta","name":"Beta","description":"Beta skill","state":"loaded","enabled":true,"frontmatter_raw":"name: Beta","body":"Beta body","permissions":{"marker":"before"},"fingerprint":"fp-beta-before"}'
        detail_beta_scan='{"id":"beta","agent":"claude-code","scope":"agent-project","path":"/tmp/project/beta/SKILL.md","display_path":"/tmp/project/beta/SKILL.md","definition_id":"def.beta","name":"Beta","description":"Beta skill","state":"loaded","enabled":true,"frontmatter_raw":"name: Beta","body":"Beta body","permissions":{"marker":"scan"},"fingerprint":"fp-beta-scan"}'
        detail_beta_toggle='{"id":"beta","agent":"claude-code","scope":"agent-project","path":"/tmp/project/beta/SKILL.md","display_path":"/tmp/project/beta/SKILL.md","definition_id":"def.beta","name":"Beta","description":"Beta skill","state":"loaded","enabled":false,"frontmatter_raw":"name: Beta","body":"Beta body","permissions":{"marker":"toggle"},"fingerprint":"fp-beta-toggle"}'
        detail_gamma_scan='{"id":"gamma","agent":"codex","scope":"agent-global","path":"/tmp/codex/skills/gamma/SKILL.md","display_path":"~/.codex/skills/gamma/SKILL.md","definition_id":"codex:gamma","name":"Gamma","description":"Gamma skill","state":"loaded","enabled":true,"frontmatter_raw":"name: Gamma","body":"Gamma body","permissions":{"marker":"codex-scan"},"fingerprint":"fp-gamma-scan"}'
        detail_gamma_project='{"id":"gamma","agent":"codex","scope":"agent-global","path":"/tmp/codex/skills/gamma/SKILL.md","display_path":"~/.codex/skills/gamma/SKILL.md","definition_id":"codex:gamma","name":"Gamma","description":"Gamma skill","state":"loaded","enabled":true,"frontmatter_raw":"name: Gamma","body":"Gamma body","permissions":{"marker":"project"},"fingerprint":"fp-gamma-project"}'
        detail_omega='{"id":"omega","agent":"opencode","scope":"agent-global","path":"/tmp/opencode/skills/omega/SKILL.md","display_path":"~/.config/opencode/skills/omega/SKILL.md","definition_id":"opencode:omega","name":"Omega","description":"Omega skill","state":"loaded","enabled":true,"frontmatter_raw":"name: Omega","body":"Omega body","permissions":{},"fingerprint":"fp-omega"}'
        detail_omega_disabled='{"id":"omega","agent":"opencode","scope":"agent-global","path":"/tmp/opencode/skills/omega/SKILL.md","display_path":"~/.config/opencode/skills/omega/SKILL.md","definition_id":"opencode:omega","name":"Omega","description":"Omega skill","state":"disabled","enabled":false,"frontmatter_raw":"name: Omega","body":"Omega body","permissions":{},"fingerprint":"fp-omega"}'
        detail_toolglobal='{"id":"tool-alpha","agent":"tool-global","scope":"tool-global","path":"/tmp/skills-copilot/staging/tool-alpha/SKILL.md","display_path":"Tool Pool/tool-alpha/SKILL.md","definition_id":"tool:alpha","name":"Tool Alpha","description":"Tool-global staged skill","state":"loaded","enabled":true,"frontmatter_raw":"name: Tool Alpha","body":"Tool Alpha body","permissions":{},"fingerprint":"fp-tool-alpha"}'

        case "$input" in
          *\\"app.stateSnapshot\\"*)
            state_snapshot_response
            ;;
          *\\"service.status\\"*)
            if [ "$scenario" = "error" ]; then service_error; fi
            status_response
            ;;
          *\\"rules.listTuning\\"*)
            respond '{"id":"test","ok":true,"result":[]}'
            ;;
          *\\"llm.status\\"*)
            if [ "$scenario" = "old-service" ]; then
              respond '{"id":"test","ok":false,"result":null,"error":{"code":"unknown_method","message":"unknown method: llm.status"}}'
            elif [ "$scenario" = "llm-ready" ] || [ "$scenario" = "prompt-ready" ]; then
              respond '{"id":"test","ok":true,"result":{"enabled":true,"provider":"openai","model":"gpt-5","disabled_reason":null,"supported_actions":["analyze","recommend","explain_conflict","draft_frontmatter"]}}'
            else
              respond '{"id":"test","ok":true,"result":{"enabled":false,"provider":null,"model":null,"disabled_reason":"LLM is disabled.","supported_actions":["analyze","recommend","explain_conflict","draft_frontmatter"]}}'
            fi
            ;;
          *\\"llm.listProviderProfiles\\"*)
            if [ "$scenario" = "autosave-delayed-provider" ]; then
              respond '{"id":"test","ok":true,"result":{"service_available":true,"enabled":true,"configured":true,"active_profile_id":"openai-compatible","credential_storage":"keychain","credential_persistence_allowed":true,"profiles":[{"id":"openai-compatible","kind":"openai-compatible","endpoint":"https://provider-b.example.com/v1","model":"model-b","enabled":true,"configured":true,"has_api_key":true}]}}'
            elif [ "$scenario" = "prompt-ready" ]; then
              respond '{"id":"test","ok":true,"result":{"service_available":true,"enabled":true,"configured":true,"active_profile_id":"openai-compatible","credential_storage":"keychain","credential_persistence_allowed":true,"profiles":[{"id":"openai-compatible","kind":"openai-compatible","endpoint":"https://llm.example.com/v1","model":"gpt-5","enabled":true,"configured":true,"has_api_key":true}]}}'
            fi
            respond '{"id":"test","ok":false,"result":null,"error":{"code":"unknown_method","message":"unknown method: llm.listProviderProfiles"}}'
            ;;
          *\\"llm.saveProviderProfile\\"*)
            if [ "$scenario" = "autosave-delayed-provider" ]; then
              if printf '%s' "$input" | grep -q '\\"api_key\\":\\"A\\"'; then
                wait_for_release "$SKILLS_COPILOT_FAKE_PROVIDER_A_RELEASE" || service_error
              elif printf '%s' "$input" | grep -q '\\"api_key\\":\\"B\\"'; then
                wait_for_release "$SKILLS_COPILOT_FAKE_PROVIDER_B_RELEASE" || service_error
              fi
              respond '{"id":"test","ok":true,"result":{"profile":null}}'
            fi
            respond '{"id":"test","ok":false,"result":null,"error":{"code":"unknown_method","message":"unknown method: llm.saveProviderProfile"}}'
            ;;
          *\\"session.previewLocalSessions\\"*)
            if [ "$scenario" = "sessions-mixed" ]; then
              case "$input" in
                *\\"scope\\":\\"all\\"*)
                  respond '{"id":"test","ok":true,"result":{"generated_by":"local-v2.98","authorized":true,"count":3,"total_candidate_count":3,"total_matched_count":3,"offset":0,"limit":50,"has_more":false,"session_rows":[{"id":"session-alpha","title":"Analyze repository CI","source_kind":"authorized-local-session","agent":"claude-code","scope":"project","project_root":"/tmp/project","redacted_path":"$HOME/.codex/sessions/alpha.jsonl","excerpt":"Audit the current repository CI pipeline.","user_message_count":1,"total_message_count":2,"tool_call_count":1,"skill_call_count":0,"content_hash":"alpha"},{"id":"session-develop","title":"Switch to develop branch","source_kind":"authorized-local-session","agent":"claude-code","scope":"project","project_root":"/tmp/project","redacted_path":"$HOME/.codex/sessions/develop.jsonl","excerpt":"Switch branch to develop and inspect status.","user_message_count":1,"total_message_count":2,"tool_call_count":1,"skill_call_count":0,"content_hash":"develop"},{"id":"session-global","title":"Review global setup","source_kind":"authorized-local-session","agent":"claude-code","scope":"agent-global","redacted_path":"$HOME/.codex/sessions/global.jsonl","excerpt":"Review global agent setup.","user_message_count":2,"total_message_count":4,"tool_call_count":0,"skill_call_count":1,"content_hash":"global"}],"skill_usage_rows":[{"skill_name":"release-audit","call_count":1,"session_count":1,"agent":"claude-code"}]}}'
                  ;;
                *\\"sort\\":\\"title\\"*\\"direction\\":\\"descending\\"*)
                  respond '{"id":"test","ok":true,"result":{"generated_by":"local-v2.98","authorized":true,"count":2,"total_candidate_count":3,"total_matched_count":2,"offset":0,"limit":50,"has_more":false,"session_rows":[{"id":"session-develop","title":"Switch to develop branch","source_kind":"authorized-local-session","agent":"claude-code","scope":"project","project_root":"/tmp/project","redacted_path":"$HOME/.codex/sessions/develop.jsonl","excerpt":"Switch branch to develop and inspect status.","user_message_count":1,"total_message_count":2,"tool_call_count":1,"skill_call_count":0,"content_hash":"develop"},{"id":"session-alpha","title":"Analyze repository CI","source_kind":"authorized-local-session","agent":"claude-code","scope":"project","project_root":"/tmp/project","redacted_path":"$HOME/.codex/sessions/alpha.jsonl","excerpt":"Audit the current repository CI pipeline.","user_message_count":1,"total_message_count":2,"tool_call_count":1,"skill_call_count":0,"content_hash":"alpha"}]}}'
                  ;;
              esac
              respond '{"id":"test","ok":true,"result":{"generated_by":"local-v2.98","authorized":true,"count":2,"total_candidate_count":3,"total_matched_count":2,"offset":0,"limit":50,"has_more":false,"session_rows":[{"id":"session-alpha","title":"Analyze repository CI","source_kind":"authorized-local-session","agent":"claude-code","scope":"project","project_root":"/tmp/project","redacted_path":"$HOME/.codex/sessions/alpha.jsonl","excerpt":"Audit the current repository CI pipeline.","user_message_count":1,"total_message_count":2,"tool_call_count":1,"skill_call_count":0,"content_hash":"alpha"},{"id":"session-develop","title":"Switch to develop branch","source_kind":"authorized-local-session","agent":"claude-code","scope":"project","project_root":"/tmp/project","redacted_path":"$HOME/.codex/sessions/develop.jsonl","excerpt":"Switch branch to develop and inspect status.","user_message_count":1,"total_message_count":2,"tool_call_count":1,"skill_call_count":0,"content_hash":"develop"}]}}'
            fi
            if [ "$scenario" = "sessions-all-scope-project-root" ]; then
              case "$input" in
                *\\"scope\\":\\"all\\"*)
                  respond '{"id":"test","ok":true,"result":{"generated_by":"local-v2.98","authorized":true,"count":2,"total_candidate_count":2,"total_matched_count":2,"offset":0,"limit":50,"has_more":false,"session_rows":[{"id":"session-project-from-all","title":"Open latest app","source_kind":"authorized-local-session","agent":"claude-code","scope":"all","project_root":"<project-root>","redacted_path":"$HOME/.claude/projects/project/session.jsonl","excerpt":"Open latest app.","user_message_count":2,"total_message_count":24,"tool_call_count":24,"skill_call_count":1,"content_hash":"project-all"},{"id":"session-global","title":"Review global setup","source_kind":"authorized-local-session","agent":"claude-code","scope":"all","redacted_path":"$HOME/.claude.jsonl","excerpt":"Review global setup.","user_message_count":1,"total_message_count":2,"tool_call_count":0,"skill_call_count":0,"content_hash":"global"}]}}'
                  ;;
              esac
              respond '{"id":"test","ok":true,"result":{"generated_by":"local-v2.98","authorized":true,"count":1,"total_candidate_count":2,"total_matched_count":1,"offset":0,"limit":50,"has_more":false,"session_rows":[{"id":"session-project-from-all","title":"Open latest app","source_kind":"authorized-local-session","agent":"claude-code","scope":"all","project_root":"<project-root>","redacted_path":"$HOME/.claude/projects/project/session.jsonl","excerpt":"Open latest app.","user_message_count":2,"total_message_count":24,"tool_call_count":24,"skill_call_count":1,"content_hash":"project-all"}]}}'
            fi
            if [ "$scenario" = "sessions" ]; then
              case "$input" in
                *\\"search\\":\\"develop\\"*)
                  respond '{"id":"test","ok":true,"result":{"generated_by":"local-v2.98","authorized":true,"count":1,"total_candidate_count":2,"total_matched_count":1,"offset":0,"limit":50,"has_more":false,"session_rows":[{"id":"session-develop","title":"Switch to develop branch","source_kind":"authorized-local-session","agent":"claude-code","scope":"project","project_root":"/tmp/project","redacted_path":"$HOME/.codex/sessions/develop.jsonl","excerpt":"Switch branch to develop and inspect status.","user_message_count":1,"total_message_count":2,"tool_call_count":1,"skill_call_count":0,"content_hash":"develop"}]}}'
                  ;;
                *\\"search\\":\\"missing\\"*)
                  respond '{"id":"test","ok":true,"result":{"generated_by":"local-v2.98","authorized":true,"count":0,"total_candidate_count":2,"total_matched_count":0,"offset":0,"limit":50,"has_more":false,"session_rows":[]}}'
                  ;;
              esac
              respond '{"id":"test","ok":true,"result":{"generated_by":"local-v2.98","authorized":true,"count":2,"total_candidate_count":2,"total_matched_count":2,"offset":0,"limit":50,"has_more":false,"session_rows":[{"id":"session-alpha","title":"Analyze repository CI","source_kind":"authorized-local-session","agent":"claude-code","scope":"project","project_root":"/tmp/project","redacted_path":"$HOME/.codex/sessions/alpha.jsonl","excerpt":"Audit the current repository CI pipeline.","user_message_count":1,"total_message_count":2,"tool_call_count":1,"skill_call_count":0,"content_hash":"alpha"},{"id":"session-develop","title":"Switch to develop branch","source_kind":"authorized-local-session","agent":"claude-code","scope":"project","project_root":"/tmp/project","redacted_path":"$HOME/.codex/sessions/develop.jsonl","excerpt":"Switch branch to develop and inspect status.","user_message_count":1,"total_message_count":2,"tool_call_count":1,"skill_call_count":0,"content_hash":"develop"}]}}'
            fi
            respond '{"id":"test","ok":false,"result":null,"error":{"code":"unknown_method","message":"unknown method: session.previewLocalSessions"}}'
            ;;
          *\\"llm.providerObservability\\"*)
            if [ "$scenario" = "prompt-ready" ]; then
              respond '{"id":"test","ok":true,"result":{"generated_by":"local-v2.64","app_local_only":true,"metadata_redacted":true,"filters":{"window_days":30,"limit":30,"include_history":true,"include_budget_hints":true,"include_retention_recommendations":true,"include_evidence":true},"summary":{"call_count":3,"success_count":1,"failure_count":1,"blocked_count":1,"provider_count":1,"model_count":2,"destination_count":1,"error_count":1,"estimated_input_tokens":980,"estimated_output_tokens":320,"estimated_total_tokens":1300,"estimated_cost_usd":0.041,"total_duration_ms":1800,"average_duration_ms":600,"budget_hint_count":1,"retention_recommendation_count":2,"summary":"Three redacted provider-call metadata rows were reviewed locally."},"call_rows":[{"id":"call-1","preview_id":"preview-1","confirmation_id":"confirm-1","request_kind":"task_cockpit","action":"task_cockpit","provider":"openai-compatible","model":"gpt-5","destination_host":"llm.example.com","status":"succeeded","duration_ms":720,"input_tokens":420,"output_tokens":120,"total_tokens":540,"estimated_cost_usd":0.014,"completed_at":1781260000000,"draft_copy_only":true,"provider_request_sent":true,"credential_accessed":false,"raw_prompt_persisted":false,"raw_response_persisted":false,"raw_secret_returned":false,"evidence_refs":["prompt-run:preview-1"],"safety_flags":["copy-only","raw prompt not stored"],"detail":"Provider response metadata was stored without raw prompt or response."},{"id":"call-2","request_kind":"analyze","provider":"openai-compatible","model":"gpt-5-mini","destination_host":"llm.example.com","status":"failed","error_code":"timeout","error_message":"Provider request timed out.","duration_ms":1080,"input_tokens":560,"output_tokens":0,"total_tokens":560,"estimated_cost_usd":0.027,"draft_copy_only":true,"provider_request_sent":true,"credential_accessed":false,"raw_prompt_persisted":false,"raw_response_persisted":false,"raw_secret_returned":false,"evidence_refs":["prompt-run:timeout"],"safety_flags":["raw response not stored"]}],"provider_rows":[{"kind":"provider","label":"OpenAI-compatible","provider":"openai-compatible","call_count":3,"success_count":1,"failure_count":1,"blocked_count":1,"estimated_tokens":1300,"estimated_cost_usd":0.041,"average_duration_ms":600,"status":"partial","notes":["One timeout and one blocked local preview."],"evidence_refs":["provider:openai-compatible"]}],"model_rows":[{"kind":"model","label":"gpt-5","model":"gpt-5","call_count":1,"success_count":1,"estimated_tokens":540,"status":"ok"},{"kind":"model","label":"gpt-5-mini","model":"gpt-5-mini","call_count":1,"failure_count":1,"estimated_tokens":560,"status":"warning"}],"destination_rows":[{"kind":"destination","label":"llm.example.com","destination_host":"llm.example.com","call_count":2,"status":"partial"}],"model_task_history_rows":[{"id":"model-task:fixture","source":"model-task-matches.json","source_kind":"manual","title":"Release audit model fit","task":"Review local release audit evidence.","task_kind":"task_cockpit","agent":"codex","provider":"openai-compatible","model":"gpt-5","destination_host":"llm.example.com","match_status":"fit","confidence_score":88,"status":"fit","latency_ms":720,"estimated_total_tokens":540,"estimated_cost_usd":0.014,"gap_notes":[],"blocker_notes":[],"outcome_notes":["The model was recorded as a fit for release audit work."],"evidence_refs":["prompt-run:preview-1"],"redaction_status":"redacted-local-only","safety_flags":{"provider_request_sent":false,"write_back_allowed":false,"write_actions_available":false,"script_execution_allowed":false,"execution_actions_available":false,"config_mutation_allowed":false,"snapshot_created":false,"triage_mutation_allowed":false,"credential_accessed":false,"raw_prompt_persisted":false,"raw_response_persisted":false,"raw_trace_persisted":false,"cloud_sync_enabled":false,"telemetry_enabled":false,"raw_secret_returned":false}}],"status_rows":[{"severity":"info","status":"succeeded","title":"Succeeded","detail":"One call completed.","count":1},{"severity":"warning","status":"blocked","title":"Blocked locally","detail":"One preview never sent a provider request.","count":1}],"error_rows":[{"severity":"warning","status":"failed","title":"Timeout","detail":"Provider request timed out.","count":1,"provider":"openai-compatible","model":"gpt-5-mini","evidence_refs":["prompt-run:timeout"]}],"budget_hints":[{"severity":"info","title":"Monthly budget healthy","detail":"Estimated spend is below the configured budget.","value":"0.041","threshold":"25.00","recommendation":"Keep monitoring prompt-run history."}],"usage_hints":[{"severity":"info","title":"Token usage available","detail":"Estimated token totals are derived from redacted metadata.","value":"1300"}],"retention_rows":[{"severity":"info","title":"Retain metadata only","detail":"Keep redacted prompt-run metadata; do not retain raw prompts.","recommendation":"Review old metadata periodically."}],"cleanup_recommendations":[{"severity":"info","title":"No cleanup required","detail":"No unsafe raw prompt or response payloads were observed."}],"gap_notes":["No raw response bodies are available for observability by design."],"blocker_notes":[],"evidence_references":[{"title":"Prompt run history","detail":"Read from app-local prompt-runs metadata.","source":"llm.providerObservability"}],"prompt_request":{"enabled":false,"request_kind":"provider_observability","summary":"No provider request is prepared or sent by observability.","draft_copy_only":true,"redacted":true},"safety_flags":{"provider_request_sent":false,"write_back_allowed":false,"write_actions_available":false,"script_execution_allowed":false,"execution_actions_available":false,"config_mutation_allowed":false,"snapshot_created":false,"triage_mutation_allowed":false,"credential_accessed":false,"raw_prompt_persisted":false,"raw_response_persisted":false,"raw_trace_persisted":false,"cloud_sync_enabled":false,"telemetry_enabled":false,"raw_secret_returned":false,"notes":["observability did not send a provider request"]}}}'
            fi
            respond '{"id":"test","ok":false,"result":null,"error":{"code":"unknown_method","message":"unknown method: llm.providerObservability"}}'
            ;;
          *\\"llm.prepareAction\\"*)
            if [ "$scenario" = "old-service" ]; then
              respond '{"id":"test","ok":false,"result":null,"error":{"code":"unknown_method","message":"unknown method: llm.prepareAction"}}'
            elif [ "$scenario" = "llm-ready" ] || [ "$scenario" = "prompt-ready" ]; then
              case "$input" in
                *\\"kind\\":\\"draft_frontmatter\\"*)
                  respond '{"id":"test","ok":true,"result":{"action":"draft_frontmatter","enabled":true,"disabled_reason":null,"provider":"openai","model":"gpt-5","estimate":{"input_tokens":240,"output_tokens":180,"total_tokens":420,"estimated_cost_usd":0.0042},"confirmation_required":true}}'
                  ;;
                *)
                  respond '{"id":"test","ok":true,"result":{"action":"analyze","enabled":true,"disabled_reason":null,"provider":"openai","model":"gpt-5","estimate":{"input_tokens":240,"output_tokens":120,"total_tokens":360,"estimated_cost_usd":0.0042},"confirmation_required":true}}'
                  ;;
              esac
            else
              respond '{"id":"test","ok":true,"result":{"action":"analyze","enabled":false,"disabled_reason":"LLM is disabled.","provider":null,"model":null,"estimate":null,"confirmation_required":true}}'
            fi
            ;;
          *\\"llm.previewPrompt\\"*)
            if [ "$scenario" = "prompt-ready" ] || [ "$scenario" = "slow-task-cockpit" ]; then
              if printf '%s' "$input" | grep -q '\\"request_kind\\":\\"task_cockpit\\"'; then
                respond '{"id":"test","ok":true,"result":{"preview_id":"task-cockpit-preview","request_kind":"task_cockpit","action":"task_cockpit","scope":"agents","prompt_scope":"Task preflight for selected agents","enabled":true,"provider":"openai-compatible","model":"gpt-5","destination_host":"llm.example.com","included_fields":["task.text","agents","effective_skills"],"excluded_fields":[{"name":"skill.body","reason":"raw body omitted"},{"name":"agent.config","reason":"config contents omitted"},{"name":"api_key","reason":"credential redacted"}],"redaction":{"status":"redacted","summary":"Secrets, raw bodies, config contents, and local paths removed.","redacted_fields":["api_key","path","skill.body","agent.config"],"placeholders":["<project-root>"]},"estimate":{"input_tokens":520,"output_tokens":240,"total_tokens":760,"estimated_cost_usd":0.008},"confirmation_required":true,"raw_prompt_persisted":false,"raw_response_persisted":false,"draft_copy_only":true,"redacted_prompt_preview":"Task preflight from selected agent and effective skill metadata."}}'
              fi
            fi
            if [ "$scenario" = "prompt-ready" ]; then
              respond '{"id":"test","ok":true,"result":{"preview_id":"prompt-preview-beta","request_kind":"action","action":"analyze","scope":"selected","prompt_scope":"Selected skill analysis for Beta","enabled":true,"provider":"openai-compatible","model":"gpt-5","destination_host":"llm.example.com","included_fields":["skill.name","findings.summary"],"excluded_fields":[{"name":"api_key","reason":"credential redacted"}],"redaction":{"status":"redacted","summary":"Secrets and local paths removed.","redacted_fields":["api_key"],"placeholders":["<project-root>"]},"estimate":{"input_tokens":240,"output_tokens":120,"total_tokens":360,"estimated_cost_usd":0.0042},"confirmation_required":true,"raw_prompt_persisted":false,"raw_response_persisted":false,"draft_copy_only":true,"redacted_prompt_preview":"Analyze Beta using catalog metadata and finding summaries only."}}'
            fi
            respond '{"id":"test","ok":false,"result":null,"error":{"code":"unknown_method","message":"unknown method: llm.previewPrompt"}}'
            ;;
          *\\"llm.confirmPromptAndSend\\"*)
            if [ "$scenario" = "slow-task-cockpit" ]; then
              if printf '%s' "$input" | grep -q '\\"request_kind\\":\\"task_cockpit\\"'; then
                sleep 1
                respond '{"id":"test","ok":true,"result":{"preview_id":"task-cockpit-preview","status":"succeeded","message":"Provider response received.","output_text":"{\\"generated_by\\":\\"provider-task-cockpit\\",\\"catalog_available\\":true,\\"filters\\":{\\"task_text\\":\\"Prepare local release audit work.\\",\\"agents\\":[\\"claude-code\\"]},\\"summary\\":{\\"task_text\\":\\"Prepare local release audit work.\\",\\"summary\\":\\"Late slow result that should be ignored after timeout or cancel.\\",\\"recommended_agent\\":\\"claude-code\\",\\"recommended_skill_name\\":\\"Slow Beta\\",\\"readiness_score\\":61,\\"routing_score\\":62,\\"agent_candidate_count\\":1,\\"skill_candidate_count\\":1,\\"gap_count\\":0,\\"blocker_count\\":0},\\"agent_candidates\\":[{\\"id\\":\\"agent-claude\\",\\"rank\\":1,\\"title\\":\\"Claude Code\\",\\"agent\\":\\"claude-code\\",\\"score\\":62}],\\"skill_candidates\\":[{\\"id\\":\\"skill:beta\\",\\"rank\\":1,\\"title\\":\\"Slow Beta\\",\\"agent\\":\\"claude-code\\",\\"skill\\":{\\"instance_id\\":\\"beta\\",\\"name\\":\\"Slow Beta\\",\\"agent\\":\\"claude-code\\",\\"definition_id\\":\\"def.beta\\"},\\"routing_score\\":62,\\"readiness_score\\":61}],\\"safety_flags\\":{\\"provider_request_sent\\":true,\\"write_back_allowed\\":false,\\"script_execution_allowed\\":false,\\"raw_prompt_persisted\\":false,\\"raw_response_persisted\\":false}}","draft_copy_only":true,"raw_prompt_persisted":false,"raw_response_persisted":false,"write_back_allowed":false,"script_execution_allowed":false,"audit_metadata":{"request_id":"audit-task-cockpit-slow","status":"succeeded","provider":"openai-compatible","model":"gpt-5","destination_host":"llm.example.com","redaction_applied":true,"raw_prompt_persisted":false,"raw_response_persisted":false,"input_tokens":520,"output_tokens":180}}}'
              fi
            fi
            if [ "$scenario" = "prompt-ready" ]; then
              if printf '%s' "$input" | grep -q '\\"request_kind\\":\\"task_cockpit\\"'; then
                respond '{"id":"test","ok":true,"result":{"preview_id":"task-cockpit-preview","status":"succeeded","message":"Provider response received.","output_text":"{\\"generated_by\\":\\"provider-task-cockpit\\",\\"catalog_available\\":true,\\"filters\\":{\\"task_text\\":\\"Prepare local release audit work.\\",\\"agents\\":[\\"claude-code\\"]},\\"summary\\":{\\"task_text\\":\\"Prepare local release audit work.\\",\\"summary\\":\\"Beta is the strongest provider-ranked route; confirm handoff boundaries.\\",\\"recommended_agent\\":\\"claude-code\\",\\"recommended_skill_name\\":\\"Beta\\",\\"readiness_score\\":78,\\"routing_score\\":88,\\"agent_candidate_count\\":1,\\"skill_candidate_count\\":2,\\"gap_count\\":1,\\"blocker_count\\":0},\\"agent_candidates\\":[{\\"id\\":\\"agent-claude\\",\\"rank\\":1,\\"title\\":\\"Claude Code\\",\\"agent\\":\\"claude-code\\",\\"score\\":82,\\"summary\\":\\"Selected agent has enabled matching skills.\\",\\"reasons\\":[\\"Effective skills include Beta.\\"]}],\\"skill_candidates\\":[{\\"id\\":\\"skill:beta\\",\\"rank\\":1,\\"title\\":\\"Beta\\",\\"agent\\":\\"claude-code\\",\\"skill\\":{\\"instance_id\\":\\"beta\\",\\"name\\":\\"Beta\\",\\"agent\\":\\"claude-code\\",\\"definition_id\\":\\"def.beta\\"},\\"readiness_score\\":78,\\"routing_score\\":88,\\"summary\\":\\"Best match for release audit.\\",\\"reasons\\":[\\"Description matches audit work.\\"]},{\\"id\\":\\"skill:alpha\\",\\"rank\\":2,\\"title\\":\\"Alpha\\",\\"agent\\":\\"claude-code\\",\\"skill\\":{\\"instance_id\\":\\"alpha\\",\\"name\\":\\"Alpha\\",\\"agent\\":\\"claude-code\\",\\"definition_id\\":\\"def.alpha\\"},\\"routing_score\\":64,\\"summary\\":\\"Similar audit wording.\\"}],\\"readiness_signals\\":[{\\"id\\":\\"readiness-beta\\",\\"title\\":\\"Provider readiness\\",\\"detail\\":\\"Ready for local audit, confirm handoff boundary.\\",\\"status\\":\\"review\\",\\"agent\\":\\"claude-code\\"}],\\"gap_rows\\":[{\\"id\\":\\"gap-codex\\",\\"title\\":\\"Codex coverage not selected\\",\\"detail\\":\\"Selected scope only includes Claude Code.\\",\\"severity\\":\\"info\\",\\"agent\\":\\"codex\\"}],\\"blocker_rows\\":[],\\"safety_flags\\":{\\"provider_request_sent\\":true,\\"write_back_allowed\\":false,\\"write_actions_available\\":false,\\"script_execution_allowed\\":false,\\"execution_actions_available\\":false,\\"config_mutation_allowed\\":false,\\"snapshot_created\\":false,\\"triage_mutation_allowed\\":false,\\"credential_accessed\\":false,\\"raw_prompt_persisted\\":false,\\"raw_response_persisted\\":false,\\"raw_trace_persisted\\":false,\\"cloud_sync_enabled\\":false,\\"telemetry_enabled\\":false,\\"raw_secret_returned\\":false,\\"notes\\":[\\"copy-only recommendation\\"]}}","draft_copy_only":true,"raw_prompt_persisted":false,"raw_response_persisted":false,"write_back_allowed":false,"script_execution_allowed":false,"audit_metadata":{"request_id":"audit-task-cockpit-1","status":"succeeded","provider":"openai-compatible","model":"gpt-5","destination_host":"llm.example.com","redaction_applied":true,"raw_prompt_persisted":false,"raw_response_persisted":false,"input_tokens":520,"output_tokens":180}}}'
              fi
              respond '{"id":"test","ok":true,"result":{"preview_id":"prompt-preview-beta","status":"succeeded","message":"Provider response received.","output_text":"Read-only analysis for Beta.","draft_copy_only":true,"raw_prompt_persisted":false,"raw_response_persisted":false,"write_back_allowed":false,"script_execution_allowed":false,"audit_metadata":{"request_id":"audit-prompt-1","status":"succeeded","provider":"openai-compatible","model":"gpt-5","destination_host":"llm.example.com","redaction_applied":true,"raw_prompt_persisted":false,"raw_response_persisted":false,"input_tokens":240,"output_tokens":80}}}'
            fi
            respond '{"id":"test","ok":false,"result":null,"error":{"code":"unknown_method","message":"unknown method: llm.confirmPromptAndSend"}}'
            ;;
          *\\"script.previewExecution\\"*)
            if [ "$scenario" = "script-preview" ]; then
              respond '{"id":"test","ok":true,"result":{"instance_id":"beta","script_name":"setup","command_preview":["bash","scripts/setup.sh"],"scope":{"current_cwd":"/tmp/project","env":{"SKILLS_SAFE_MODE":"1"},"network":"none","files":["/tmp/project/scripts/setup.sh"]},"risks":["Writes are blocked by default."],"requires_confirmation":true,"execution_allowed":false,"audit_status":"blocked","audit_id":"audit-1","summary":"Blocked until confirmed.","reason":"Native UI is preview-only."}}'
            else
              respond '{"id":"test","ok":false,"result":null,"error":{"code":"unknown_method","message":"unknown method: script.previewExecution"}}'
            fi
            ;;
          *\\"batch.previewSkillToggles\\"*)
            if [ "$scenario" = "batch-mixed" ]; then
              respond '{"id":"test","ok":true,"result":{"preview_id":"batch-preview-1","action":"disable","target_enabled":false,"selected_count":4,"writable_count":2,"skipped_count":2,"affected_skills":[{"instance_id":"alpha","name":"Alpha","agent":"claude-code","scope":"agent-global","display_path":"/tmp/global/alpha/SKILL.md","current_enabled":true,"target_enabled":false},{"instance_id":"gamma","name":"Gamma","agent":"codex","scope":"agent-global","display_path":"~/.codex/skills/gamma/SKILL.md","current_enabled":true,"target_enabled":false}],"skipped_items":[{"instance_id":"beta","name":"Beta","agent":"claude-code","scope":"agent-project","display_path":"/tmp/project/beta/SKILL.md","current_enabled":false,"target_enabled":false,"reason":"Already disabled"},{"instance_id":"pi-one","name":"Pi One","agent":"pi","scope":"agent-global","display_path":"~/.pi/skills/pi-one/SKILL.md","current_enabled":true,"target_enabled":false,"reason":"Pi is read-only."}],"snapshot_plan":{"summary":"Create config snapshots for Claude Code and Codex before applying; rollback uses existing agent-config timeline.","rollback_supported":true,"targets":["/tmp/home/.claude/settings.json","/tmp/home/.codex/config.toml"]},"apply_supported":true}}'
            fi
            respond '{"id":"test","ok":false,"result":null,"error":{"code":"unknown_method","message":"unknown method: batch.previewSkillToggles"}}'
            ;;
          *\\"batch.applySkillToggles\\"*)
            if [ "$scenario" = "batch-mixed" ]; then
              respond '{"id":"test","ok":true,"result":{"updated_count":2,"skipped_count":2,"snapshot_ids":["snap-claude-new","snap-codex"]}}'
            fi
            respond '{"id":"test","ok":false,"result":null,"error":{"code":"unknown_method","message":"unknown method: batch.applySkillToggles"}}'
            ;;
          *\\"catalog.listSkills\\"*)
            if [ "$scenario" = "empty" ]; then
              respond '{"id":"test","ok":true,"result":[]}'
            elif [ "$scenario" = "toggle-disabled" ]; then
              respond '{"id":"test","ok":true,"result":'"$skills_toggled"'}'
            elif [ "$scenario" = "toggle-codex-disabled" ]; then
              respond '{"id":"test","ok":true,"result":'"$skills_codex_toggled"'}'
            elif [ "$scenario" = "opencode" ]; then
              if grep -q '"method":"config.toggleSkill"' "$SKILLS_COPILOT_FAKE_SERVICE_CALLS"; then
                respond '{"id":"test","ok":true,"result":'"$skills_opencode_toggled"'}'
              else
                respond '{"id":"test","ok":true,"result":'"$skills_opencode"'}'
              fi
            elif [ "$scenario" = "tool-global" ]; then
              respond '{"id":"test","ok":true,"result":'"$skills_toolglobal"'}'
            elif [ "$scenario" = "batch-mixed" ]; then
              respond '{"id":"test","ok":true,"result":'"$skills_batch_mixed"'}'
            else
              respond '{"id":"test","ok":true,"result":'"$skills_normal"'}'
            fi
            ;;
          *\\"catalog.scanAll\\"*)
            if [ "$scenario" = "scan-slow" ]; then sleep 1; fi
            if [ "$scenario" = "stale-after-toggle" ]; then
              scan_skills=$skills_toggled
              scan_finding_count=1
            else
              scan_skills=$skills_normal
              scan_finding_count=0
            fi
            respond '{"id":"test","ok":true,"result":{"scanned_count":3,"skills":'"$scan_skills"',"activity":{"operation":"catalog.scanAll","status":"completed-partial","started_at":1,"finished_at":2,"scanned_count":3,"skill_count":3,"finding_count":'"$scan_finding_count"',"conflict_count":0,"snapshot_count":0,"roots":["$HOME/.claude/skills","$HOME/.agents/skills","<adapter-root>/missing-opencode"],"log_entries":[{"level":"warning","message":"Claude Code discovered 2 skill(s); catalog now has 2 skill(s), 0 broken, across 0 complete root(s), 1 partial root(s), and 0 skipped root(s); first scan issue entry_unreadable at <adapter-root>/dangling-link: A directory entry could not be inspected or resolved."},{"level":"info","message":"Codex discovered 1 skill(s); catalog now has 1 skill(s), 0 broken, across 1 complete root(s), 0 partial root(s), and 0 skipped root(s)."},{"level":"warning","message":"opencode discovered 0 skill(s); catalog now has 0 skill(s), 0 broken, across 0 complete root(s), 0 partial root(s), and 1 skipped root(s); root-error skipped-root path(s): <adapter-root>/missing-opencode."}],"recovery_actions":["Review partial-root diagnostics; unseen rows under partial roots were preserved."],"agent_summaries":[{"agent":"claude-code","display_label":"Claude Code","status":"completed-partial","scanned_count":2,"catalog_count":2,"broken_count":0,"roots_considered":["$HOME/.claude/skills"],"roots_scanned":[],"roots_partial":["<adapter-root>"],"roots_skipped":[],"scan_issues":[{"kind":"entry_unreadable","path":"<adapter-root>/dangling-link","detail":"A directory entry could not be inspected or resolved."}],"recovery_actions":["Review partial scan diagnostics."]},{"agent":"codex","display_label":"Codex","status":"completed","scanned_count":1,"catalog_count":1,"broken_count":0,"roots_considered":["$HOME/.agents/skills"],"roots_scanned":["$HOME/.agents/skills"],"roots_partial":[],"roots_skipped":[],"scan_issues":[],"recovery_actions":[]},{"agent":"opencode","display_label":"opencode","status":"completed-with-skipped-roots","scanned_count":0,"catalog_count":0,"broken_count":0,"roots_considered":["<adapter-root>/missing-opencode"],"roots_scanned":[],"roots_partial":[],"roots_skipped":["<adapter-root>/missing-opencode"],"scan_issues":[{"kind":"root_unavailable","path":"<adapter-root>/missing-opencode","detail":"A declared scan root was unavailable or not a directory."}],"recovery_actions":["Review opencode skipped-root diagnostics, then retry Scan."]}]}}}'
            ;;
          *\\"project.getContext\\"*)
            if [ "$scenario" = "project-clear" ] || [ "$scenario" = "empty" ]; then
              respond '{"id":"test","ok":true,"result":{"active":null,"recent":'"$project_recent"'}}'
            elif [ "$scenario" = "project-validation-error" ]; then
              respond '{"id":"test","ok":true,"result":{"active":'"$project_invalid"',"recent":'"$project_recent"'}}'
            else
              respond '{"id":"test","ok":true,"result":{"active":'"$project_active"',"recent":'"$project_recent"'}}'
            fi
            ;;
          *\\"project.setContext\\"*)
            if [ "$scenario" = "project-validation-error" ]; then
              respond '{"id":"test","ok":true,"result":{"active":'"$project_invalid"',"recent":['"$project_invalid"']}}'
            else
              respond '{"id":"test","ok":true,"result":{"active":'"$project_active"',"recent":'"$project_recent"'}}'
            fi
            ;;
          *\\"project.clearContext\\"*)
            respond '{"id":"test","ok":true,"result":{"active":null,"recent":'"$project_recent"'}}'
            ;;
          *\\"project.validateContext\\"*)
            if [ "$scenario" = "project-validation-error" ]; then
              respond '{"id":"test","ok":true,"result":'"$project_invalid"'}'
            else
              respond '{"id":"test","ok":true,"result":'"$project_active"'}'
            fi
            ;;
          *\\"catalog.getSkill\\"*)
            case "$input" in
              *\\"instance_id\\":\\"beta\\"*)
                if [ "$scenario" = "stale-before" ]; then
                  respond '{"id":"test","ok":true,"result":'"$detail_beta_before"'}'
                elif [ "$scenario" = "stale-after-scan" ]; then
                  respond '{"id":"test","ok":true,"result":'"$detail_beta_scan"'}'
                elif [ "$scenario" = "stale-after-toggle" ]; then
                  respond '{"id":"test","ok":true,"result":'"$detail_beta_toggle"'}'
                elif [ "$scenario" = "toggle-disabled" ]; then
                  respond '{"id":"test","ok":true,"result":'"$detail_beta_disabled"'}'
                else
                  respond '{"id":"test","ok":true,"result":'"$detail_beta_enabled"'}'
                fi
                ;;
              *\\"instance_id\\":\\"gamma\\"*)
                if [ "$scenario" = "stale-after-scan" ]; then
                  respond '{"id":"test","ok":true,"result":'"$detail_gamma_scan"'}'
                elif [ "$scenario" = "stale-after-project" ]; then
                  respond '{"id":"test","ok":true,"result":'"$detail_gamma_project"'}'
                elif [ "$scenario" = "toggle-codex-disabled" ]; then
                  respond '{"id":"test","ok":true,"result":'"$detail_gamma_disabled"'}'
                else
                  respond '{"id":"test","ok":true,"result":'"$detail_gamma"'}'
                fi
                ;;
              *\\"instance_id\\":\\"omega\\"*)
                if grep -q '"method":"config.toggleSkill"' "$SKILLS_COPILOT_FAKE_SERVICE_CALLS"; then
                  respond '{"id":"test","ok":true,"result":'"$detail_omega_disabled"'}'
                else
                  respond '{"id":"test","ok":true,"result":'"$detail_omega"'}'
                fi
                ;;
              *\\"instance_id\\":\\"tool-alpha\\"*)
                respond '{"id":"test","ok":true,"result":'"$detail_toolglobal"'}'
                ;;
              *)
                respond '{"id":"test","ok":true,"result":'"$detail_alpha"'}'
                ;;
            esac
            ;;
          *\\"catalog.listFindings\\"*)
            respond '{"id":"test","ok":true,"result":[]}'
            ;;
          *\\"catalog.listConflicts\\"*)
            respond '{"id":"test","ok":true,"result":[]}'
            ;;
          *\\"skill.listEvents\\"*)
            if [ "$scenario" = "detail-scope" ]; then
              case "$input" in
                *\\"instance_id\\":\\"beta\\"*)
                  respond '{"id":"test","ok":true,"result":'"$events_beta"'}'
                  ;;
                *\\"instance_id\\":\\"gamma\\"*)
                  respond '{"id":"test","ok":true,"result":'"$events_gamma"'}'
                  ;;
              esac
            fi
            respond '{"id":"test","ok":true,"result":[]}'
            ;;
          *\\"config.readClaudeSettings\\"*)
            if [ "$scenario" = "config-legacy" ] || [ "$scenario" = "protocol-v2-missing-bindings" ]; then
              respond '{"id":"test","ok":true,"result":{"agent":"claude-code","scope":"agent-global","target":"/tmp/home/.claude/settings.json","format":"json","content":"{}\\n","exists":true}}'
            elif [ "$scenario" = "protocol-v1-bindings" ]; then
              respond '{"id":"test","ok":true,"result":{"agent":"claude-code","scope":"agent-global","target":"/tmp/home/.claude/settings.json","format":"json","content":"{\\"theme\\":\\"legacy\\"}\\n","exists":true,"revision":"sha256:malicious-v1-revision"}}'
            elif [ "$scenario" = "config-conflict" ]; then
              read_count=$(grep -c '"method":"config.readClaudeSettings"' "$SKILLS_COPILOT_FAKE_SERVICE_CALLS")
              if [ "$read_count" -gt 1 ]; then
                respond '{"id":"test","ok":true,"result":{"agent":"claude-code","scope":"agent-global","target":"/tmp/home/.claude/settings.json","format":"json","content":"{\\"theme\\":\\"external\\"}\\n","exists":true,"revision":"sha256:external-revision"}}'
              fi
              respond '{"id":"test","ok":true,"result":{"agent":"claude-code","scope":"agent-global","target":"/tmp/home/.claude/settings.json","format":"json","content":"{\\"theme\\":\\"light\\"}\\n","exists":true,"revision":"sha256:settings-revision"}}'
            elif [ "$scenario" = "config-cas" ]; then
              respond '{"id":"test","ok":true,"result":{"agent":"claude-code","scope":"agent-global","target":"/tmp/home/.claude/settings.json","format":"json","content":"{\\"theme\\":\\"light\\"}\\n","exists":true,"revision":"sha256:settings-revision"}}'
            fi
            respond '{"id":"test","ok":false,"result":null,"error":{"code":"test.missing","message":"missing Claude settings"}}'
            ;;
          *\\"config.saveClaudeSettings\\"*)
            if [ "$scenario" = "config-cas" ]; then
              respond '{"id":"test","ok":true,"result":{"agent":"claude-code","scope":"agent-global","target":"/tmp/home/.claude/settings.json","format":"json","content":"{\\"theme\\":\\"dark\\"}\\n","exists":true,"revision":"sha256:saved-revision"}}'
            elif [ "$scenario" = "config-conflict" ]; then
              respond '{"id":"test","ok":false,"result":null,"error":{"code":"config_conflict","message":"config changed since it was read"}}'
            elif [ "$scenario" = "protocol-v1-bindings" ] || [ "$scenario" = "protocol-v2-missing-bindings" ]; then
              respond '{"id":"test","ok":true,"result":{"agent":"claude-code","scope":"agent-global","target":"/tmp/home/.claude/settings.json","format":"json","content":"{\\"theme\\":\\"unsafe-write\\"}\\n","exists":true,"revision":"sha256:unsafe-write"}}'
            fi
            respond '{"id":"test","ok":false,"result":null,"error":{"code":"test.missing","message":"missing Claude settings save"}}'
            ;;
          *\\"snapshot.previewRollback\\"*)
            if [ "$scenario" = "timeline" ]; then
              respond '{"id":"test","ok":true,"result":{"snapshot":{"id":"snap-claude-new","agent":"claude-code","scope":"agent-global","target":"/tmp/home/.claude/settings.json","content":"{}\\n","reason":"pre-toggle","created_at":30},"current_content":"{\\"skillOverrides\\":{\\"beta\\":false}}\\n","current_read_error":null,"current_revision":"sha256:timeline-current","preview_token":"sha256:timeline-preview","changed":true,"redacted":false,"rollback_supported":true}}'
            elif [ "$scenario" = "config-cas" ]; then
              case "$input" in
                *\\"snapshot_id\\":\\"snap-claude-old\\"*)
                  respond '{"id":"test","ok":true,"result":{"snapshot":{"id":"snap-claude-old","agent":"claude-code","scope":"agent-project","target":"/tmp/project/.claude/settings.local.json","content":"{}\\n","reason":"pre-config-edit","created_at":20},"current_content":"{\\"permissions\\":{}}\\n","current_read_error":null,"current_revision":"sha256:rollback-current-2","preview_token":"sha256:rollback-preview-2","changed":true,"redacted":false,"rollback_supported":true}}'
                  ;;
                *)
                  respond '{"id":"test","ok":true,"result":{"snapshot":{"id":"snap-claude-new","agent":"claude-code","scope":"agent-global","target":"/tmp/home/.claude/settings.json","content":"{}\\n","reason":"pre-toggle","created_at":30},"current_content":"{\\"theme\\":\\"dark\\"}\\n","current_read_error":null,"current_revision":"sha256:rollback-current","preview_token":"sha256:rollback-preview","changed":true,"redacted":false,"rollback_supported":true}}'
                  ;;
              esac
            elif [ "$scenario" = "rollback-stale" ]; then
              respond '{"id":"test","ok":true,"result":{"snapshot":{"id":"snap-claude-new","agent":"claude-code","scope":"agent-global","target":"/tmp/home/.claude/settings.json","content":"{}\\n","reason":"pre-toggle","created_at":30},"current_content":"{\\"theme\\":\\"external\\"}\\n","current_read_error":null,"current_revision":"sha256:rollback-stale-current","preview_token":"sha256:rollback-stale-preview","changed":true,"redacted":false,"rollback_supported":true}}'
            elif [ "$scenario" = "rollback-preview-delay" ]; then
              sleep 1
              respond '{"id":"test","ok":true,"result":{"snapshot":{"id":"snap-claude-new","agent":"claude-code","scope":"agent-global","target":"/tmp/home/.claude/settings.json","content":"{}\\n","reason":"pre-toggle","created_at":30},"current_content":"{\\"theme\\":\\"dark\\"}\\n","current_read_error":null,"current_revision":"sha256:rollback-delay-current","preview_token":"sha256:rollback-delay-preview","changed":true,"redacted":false,"rollback_supported":true}}'
            elif [ "$scenario" = "rollback-preview-mismatch-blocked" ]; then
              wait_for_response_release
              respond '{"id":"test","ok":true,"result":{"snapshot":{"id":"snap-claude-old","agent":"claude-code","scope":"agent-project","target":"/tmp/project/.claude/settings.local.json","content":"{}\\n","reason":"pre-config-edit","created_at":20},"current_content":"{\\"theme\\":\\"dark\\"}\\n","current_read_error":null,"current_revision":"sha256:rollback-mismatch-current","preview_token":"sha256:rollback-mismatch-preview","changed":true,"redacted":false,"rollback_supported":true}}'
            elif [ "$scenario" = "protocol-v1-bindings" ]; then
              respond '{"id":"test","ok":true,"result":{"snapshot":{"id":"snap-claude-new","agent":"claude-code","scope":"agent-global","target":"/tmp/home/.claude/settings.json","content":"{}\\n","reason":"pre-toggle","created_at":30},"current_content":"{\\"theme\\":\\"legacy\\"}\\n","current_read_error":null,"current_revision":"sha256:malicious-v1-current","preview_token":"sha256:malicious-v1-token","changed":true,"redacted":false,"rollback_supported":true}}'
            elif [ "$scenario" = "protocol-v2-missing-bindings" ]; then
              respond '{"id":"test","ok":true,"result":{"snapshot":{"id":"snap-claude-new","agent":"claude-code","scope":"agent-global","target":"/tmp/home/.claude/settings.json","content":"{}\\n","reason":"pre-toggle","created_at":30},"current_content":"{}\\n","current_read_error":null,"changed":false,"redacted":false,"rollback_supported":true}}'
            fi
            respond '{"id":"test","ok":false,"result":null,"error":{"code":"test.missing","message":"missing snapshot preview"}}'
            ;;
          *\\"snapshot.rollback\\"*)
            if [ "$scenario" = "timeline" ] || [ "$scenario" = "config-cas" ] || [ "$scenario" = "protocol-v1-bindings" ] || [ "$scenario" = "protocol-v2-missing-bindings" ]; then
              respond '{"id":"test","ok":true,"result":3}'
            elif [ "$scenario" = "rollback-stale" ]; then
              respond '{"id":"test","ok":false,"result":null,"error":{"code":"stale_preview_token","message":"preview no longer matches current state"}}'
            elif [ "$scenario" = "rollback-stale-blocked" ]; then
              wait_for_response_release
              respond '{"id":"test","ok":false,"result":null,"error":{"code":"stale_preview_token","message":"preview no longer matches current state"}}'
            elif [ "$scenario" = "rollback-error-blocked" ]; then
              wait_for_response_release
              respond '{"id":"test","ok":false,"result":null,"error":{"code":"rollback_failed","message":"rollback service failed"}}'
            fi
            respond '{"id":"test","ok":false,"result":null,"error":{"code":"test.missing","message":"missing snapshot rollback"}}'
            ;;
          *\\"snapshot.listAgentConfig\\"*)
            if [ "$scenario" = "timeline" ] || [ "$scenario" = "config-cas" ] || [ "$scenario" = "rollback-stale" ] || [ "$scenario" = "rollback-preview-delay" ] || [ "$scenario" = "protocol-v1-bindings" ] || [ "$scenario" = "protocol-v2-missing-bindings" ]; then
              case "$input" in
                *\\"agent\\":\\"claude-code\\"*)
                  respond '{"id":"test","ok":true,"result":'"$snapshots_claude"'}'
                  ;;
                *\\"agent\\":\\"codex\\"*)
                  respond '{"id":"test","ok":true,"result":'"$snapshots_codex"'}'
                  ;;
                *\\"agent\\":\\"opencode\\"*)
                  respond '{"id":"test","ok":true,"result":'"$snapshots_opencode"'}'
                  ;;
              esac
            fi
            respond '{"id":"test","ok":true,"result":[]}'
            ;;
          *\\"snapshot.list\\"*)
            respond '{"id":"test","ok":true,"result":[]}'
            ;;
          *\\"config.readAgentConfig\\"*)
            if [ "$scenario" = "agent-config" ]; then
              case "$input" in
                *\\"agent\\":\\"claude-code\\"*) respond '{"id":"test","ok":true,"result":'"$agent_config_claude"'}' ;;
                *\\"agent\\":\\"codex\\"*) respond '{"id":"test","ok":true,"result":'"$agent_config_codex"'}' ;;
                *\\"agent\\":\\"opencode\\"*) respond '{"id":"test","ok":true,"result":'"$agent_config_opencode"'}' ;;
                *\\"agent\\":\\"pi\\"*) respond '{"id":"test","ok":true,"result":'"$agent_config_pi"'}' ;;
                *\\"agent\\":\\"hermes\\"*) respond '{"id":"test","ok":true,"result":'"$agent_config_hermes"'}' ;;
                *\\"agent\\":\\"openclaw\\"*) respond '{"id":"test","ok":true,"result":'"$agent_config_openclaw"'}' ;;
              esac
            fi
            respond '{"id":"test","ok":true,"result":[]}'
            ;;
          *\\"config.saveClaudeSettings\\"*)
            if [ "$scenario" = "autosave-delayed-config" ]; then
              if printf '%s' "$input" | grep -q 'config-a'; then
                wait_for_release "$SKILLS_COPILOT_FAKE_CONFIG_RELEASE" || service_error
                respond '{"id":"test","ok":true,"result":{"agent":"claude-code","scope":"agent-global","target":"/tmp/home/.claude/settings.json","format":"json","content":"config-a","exists":true}}'
              fi
              respond '{"id":"test","ok":true,"result":{"agent":"claude-code","scope":"agent-global","target":"/tmp/home/.claude/settings.json","format":"json","content":"config-b","exists":true}}'
            fi
            respond '{"id":"test","ok":false,"result":null,"error":{"code":"test.unknown","message":"unknown method"}}'
            ;;
          *\\"config.toggleSkill\\"*)
            if [ "$scenario" = "stale-after-toggle" ]; then
              respond '{"id":"test","ok":true,"result":'"$detail_beta_toggle"'}'
            elif [ "$scenario" = "toggle-disabled" ]; then
              sleep 1
              respond '{"id":"test","ok":true,"result":'"$detail_beta_disabled"'}'
            elif [ "$scenario" = "toggle-codex-disabled" ]; then
              respond '{"id":"test","ok":true,"result":'"$detail_gamma_disabled"'}'
            elif [ "$scenario" = "opencode" ]; then
              respond '{"id":"test","ok":true,"result":'"$detail_omega_disabled"'}'
            else
              respond '{"id":"test","ok":true,"result":'"$detail_beta_disabled"'}'
            fi
            ;;
          *)
            respond '{"id":"test","ok":false,"result":null,"error":{"code":"test.unknown","message":"unknown method"}}'
            ;;
        esac
        """
    }
}

private final class FakeServiceCallRecorder {
    private let lock = NSLock()
    private var records: [String] = []

    func record(_ input: Data, methodStateURL: URL) {
        guard let request = try? JSONSerialization.jsonObject(with: input),
              let dictionary = request as? [String: Any],
              let method = dictionary["method"] as? String else {
            return
        }
        let redactedRequest = Self.redactedJSONValue(request)
        let redactedData = try? JSONSerialization.data(
            withJSONObject: redactedRequest,
            options: [.sortedKeys]
        )
        let redactedRecord = redactedData.flatMap { String(data: $0, encoding: .utf8) }
            ?? #"{"method":"invalid"}"#
        let methodState = Data(#"{"method":"\#(method)"}"#.utf8)

        lock.lock()
        records.append(redactedRecord)
        var existingState = (try? Data(contentsOf: methodStateURL)) ?? Data()
        existingState.append(methodState)
        existingState.append(0x0A)
        try? existingState.write(to: methodStateURL, options: .atomic)
        lock.unlock()
    }

    func calls() -> String {
        lock.lock()
        let value = records.joined(separator: "\n")
        lock.unlock()
        return value
    }

    private static func redactedJSONValue(_ value: Any, key: String? = nil) -> Any {
        if let key, ConfigContentRedactor.containsSensitiveKey(key) {
            return ConfigContentRedactor.redactedValue
        }
        if let dictionary = value as? [String: Any] {
            var redacted: [String: Any] = [:]
            for (childKey, child) in dictionary {
                redacted[childKey] = redactedJSONValue(child, key: childKey)
            }
            return redacted
        }
        if let array = value as? [Any] {
            return array.map { redactedJSONValue($0) }
        }
        return value
    }
}

private actor FakeServiceProcessGate {
    func run(
        executableURL: URL,
        input: Data,
        timeoutNanoseconds: UInt64?,
        environmentOverrides: [String: String]
    ) async throws -> Data {
        let runner = PosixFakeServiceProcessRunner(environmentOverrides: environmentOverrides)
        return try runner.run(
            executableURL: executableURL,
            input: input,
            timeoutNanoseconds: timeoutNanoseconds
        )
    }
}

private struct PosixFakeServiceProcessRunner {
    let environmentOverrides: [String: String]

    func run(executableURL: URL, input: Data, timeoutNanoseconds: UInt64?) throws -> Data {
        let executablePath = executableURL.path
        return try runFakeServiceProcess(
            executablePath: executablePath,
            input: input,
            environmentOverrides: environmentOverrides,
            timeoutNanoseconds: timeoutNanoseconds
        )
    }
}

private func runFakeServiceProcess(
    executablePath: String,
    input: Data,
    environmentOverrides: [String: String],
    timeoutNanoseconds: UInt64?
) throws -> Data {
    var stdinPipe: [Int32] = [-1, -1]
    var stdoutPipe: [Int32] = [-1, -1]
    var stderrPipe: [Int32] = [-1, -1]

    try checkErrno(pipe(&stdinPipe), "pipe stdin")
    try checkErrno(pipe(&stdoutPipe), "pipe stdout")
    try checkErrno(pipe(&stderrPipe), "pipe stderr")

    defer {
        closeIfOpen(&stdinPipe[0])
        closeIfOpen(&stdinPipe[1])
        closeIfOpen(&stdoutPipe[0])
        closeIfOpen(&stdoutPipe[1])
        closeIfOpen(&stderrPipe[0])
        closeIfOpen(&stderrPipe[1])
    }

    var actions: posix_spawn_file_actions_t? = nil
    try checkPOSIX(posix_spawn_file_actions_init(&actions), "posix_spawn_file_actions_init")
    defer { posix_spawn_file_actions_destroy(&actions) }

    try checkPOSIX(posix_spawn_file_actions_adddup2(&actions, stdinPipe[0], STDIN_FILENO), "dup stdin")
    try checkPOSIX(posix_spawn_file_actions_adddup2(&actions, stdoutPipe[1], STDOUT_FILENO), "dup stdout")
    try checkPOSIX(posix_spawn_file_actions_adddup2(&actions, stderrPipe[1], STDERR_FILENO), "dup stderr")
    try checkPOSIX(posix_spawn_file_actions_addclose(&actions, stdinPipe[0]), "close child stdin reader")
    try checkPOSIX(posix_spawn_file_actions_addclose(&actions, stdinPipe[1]), "close child stdin writer")
    try checkPOSIX(posix_spawn_file_actions_addclose(&actions, stdoutPipe[0]), "close child stdout reader")
    try checkPOSIX(posix_spawn_file_actions_addclose(&actions, stdoutPipe[1]), "close child stdout writer")
    try checkPOSIX(posix_spawn_file_actions_addclose(&actions, stderrPipe[0]), "close child stderr reader")
    try checkPOSIX(posix_spawn_file_actions_addclose(&actions, stderrPipe[1]), "close child stderr writer")

    var pid: pid_t = 0
    let spawnResult = try withTemporaryEnvironment(environmentOverrides) {
        executablePath.withCString { pathPointer in
            var argv: [UnsafeMutablePointer<CChar>?] = [
                UnsafeMutablePointer(mutating: pathPointer),
                nil
            ]
            return argv.withUnsafeMutableBufferPointer { argvBuffer in
                posix_spawn(
                    &pid,
                    pathPointer,
                    &actions,
                    nil,
                    argvBuffer.baseAddress,
                    skillsCopilotNativeTestEnviron().pointee
                )
            }
        }
    }
    try checkPOSIX(spawnResult, "posix_spawn")

    closeIfOpen(&stdinPipe[0])
    closeIfOpen(&stdoutPipe[1])
    closeIfOpen(&stderrPipe[1])

    try writeAll(input, to: stdinPipe[1])
    closeIfOpen(&stdinPipe[1])

    try setNonBlocking(stdoutPipe[0])
    try setNonBlocking(stderrPipe[0])

    let startedAt = DispatchTime.now().uptimeNanoseconds
    let deadline = timeoutNanoseconds.map { startedAt &+ $0 }
    var stdout = Data()
    var stderr = Data()
    var stdoutOpen = true
    var stderrOpen = true
    var processExited = false
    var status: Int32 = 0

    while stdoutOpen || stderrOpen || !processExited {
        if let deadline, DispatchTime.now().uptimeNanoseconds >= deadline {
            kill(pid, SIGKILL)
            _ = waitpid(pid, &status, 0)
            throw ServiceClient.ClientError.processTimedOut
        }

        if stdoutOpen {
            stdoutOpen = try readAvailable(from: stdoutPipe[0], into: &stdout)
        }
        if stderrOpen {
            stderrOpen = try readAvailable(from: stderrPipe[0], into: &stderr)
        }
        if !processExited {
            let waitResult = waitpid(pid, &status, WNOHANG)
            if waitResult == pid {
                processExited = true
            } else if waitResult == -1 && errno != EINTR {
                throw POSIXProcessError(operation: "waitpid", code: errno)
            }
        }
        if stdoutOpen || stderrOpen || !processExited {
            usleep(1_000)
        }
    }

    let exitCode = normalizedExitCode(status)
    guard exitCode == 0 else {
        let message = String(data: stderr, encoding: .utf8) ?? ""
        throw ServiceClient.ClientError.processFailed(exitCode, message)
    }
    return stdout
}

private func checkPOSIX(_ result: Int32, _ operation: String) throws {
    guard result != 0 else { return }
    throw POSIXProcessError(operation: operation, code: result)
}

private func checkErrno(_ result: Int32, _ operation: String) throws {
    guard result == 0 else {
        throw POSIXProcessError(operation: operation, code: errno)
    }
}

private func closeIfOpen(_ fd: inout Int32) {
    guard fd >= 0 else { return }
    close(fd)
    fd = -1
}

private func setNonBlocking(_ fd: Int32) throws {
    let flags = fcntl(fd, F_GETFL)
    guard flags >= 0 else {
        throw POSIXProcessError(operation: "fcntl(F_GETFL)", code: errno)
    }
    guard fcntl(fd, F_SETFL, flags | O_NONBLOCK) == 0 else {
        throw POSIXProcessError(operation: "fcntl(F_SETFL)", code: errno)
    }
}

private func writeAll(_ data: Data, to fd: Int32) throws {
    guard !data.isEmpty else { return }
    try data.withUnsafeBytes { rawBuffer in
        guard var pointer = rawBuffer.baseAddress?.assumingMemoryBound(to: UInt8.self) else { return }
        var remaining = rawBuffer.count
        while remaining > 0 {
            let written = Darwin.write(fd, pointer, remaining)
            if written > 0 {
                pointer = pointer.advanced(by: written)
                remaining -= written
            } else if written == -1 && errno == EINTR {
                continue
            } else {
                throw POSIXProcessError(operation: "write", code: errno)
            }
        }
    }
}

private func readAvailable(from fd: Int32, into data: inout Data) throws -> Bool {
    var buffer = [UInt8](repeating: 0, count: 4096)
    while true {
        let count = Darwin.read(fd, &buffer, buffer.count)
        if count > 0 {
            data.append(buffer, count: count)
        } else if count == 0 {
            return false
        } else if errno == EINTR {
            continue
        } else if errno == EAGAIN || errno == EWOULDBLOCK {
            return true
        } else {
            throw POSIXProcessError(operation: "read", code: errno)
        }
    }
}

private func withTemporaryEnvironment<Result>(
    _ overrides: [String: String],
    _ body: () throws -> Result
) throws -> Result {
    var previousValues: [String: String?] = [:]
    for key in overrides.keys {
        if let value = getenv(key) {
            previousValues[key] = String(cString: value)
        } else {
            previousValues[key] = nil
        }
    }

    for (key, value) in overrides {
        guard setenv(key, value, 1) == 0 else {
            throw POSIXProcessError(operation: "setenv", code: errno)
        }
    }

    defer {
        for (key, previousValue) in previousValues {
            if let previousValue {
                _ = setenv(key, previousValue, 1)
            } else {
                _ = unsetenv(key)
            }
        }
    }

    return try body()
}

private func normalizedExitCode(_ status: Int32) -> Int32 {
    if status & 0x7f == 0 {
        return (status >> 8) & 0xff
    }
    return 128 + (status & 0x7f)
}

private struct POSIXProcessError: Error, CustomStringConvertible {
    let operation: String
    let code: Int32

    var description: String {
        "\(operation) failed: \(String(cString: strerror(code)))"
    }
}

@_silgen_name("_NSGetEnviron")
private func skillsCopilotNativeTestEnviron() -> UnsafeMutablePointer<UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?>
