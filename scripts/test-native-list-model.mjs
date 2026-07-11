#!/usr/bin/env node

import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const tempDir = await mkdtemp(join(tmpdir(), "skills-copilot-native-list-test-"));
const runnerPath = join(tempDir, "NativeListModelTest.swift");
const binaryPath = join(tempDir, "NativeListModelTest");

const runnerSource = String.raw`
import Foundation

@main
struct NativeListModelTest {
    static func main() {
        assertEqual(
            filter(searchText: "alpha").map(\.id),
            ["alpha"],
            "search matches name"
        )
        assertEqual(
            filter(searchText: "codex:gamma").map(\.id),
            ["gamma"],
            "search matches definition"
        )
        assertEqual(
            filter(searchText: "open code").map(\.id),
            ["omega"],
            "search matches opencode alias"
        )
        assertEqual(
            filter(searchText: "project/beta").map(\.id),
            ["beta"],
            "search matches path"
        )

        assertEqual(
            filter(agentFilter: .all).map(\.id),
            ["alpha", "beta", "delta", "epsilon", "gamma", "omega", "theta", "zeta"],
            "all agent filter"
        )
        assertEqual(
            filter(agentFilter: .claudeCode).map(\.id),
            ["alpha", "beta", "delta", "theta", "zeta"],
            "claude code agent filter"
        )
        assertEqual(
            filter(agentFilter: .codex).map(\.id),
            ["epsilon", "gamma"],
            "codex agent filter"
        )
        assertEqual(
            filter(agentFilter: .opencode).map(\.id),
            ["omega"],
            "opencode agent filter"
        )

        let groups = SkillListModel.groupedByAgent(filter(agentFilter: .all))
        assertEqual(
            groups.map(\.title),
            [UIStrings.claudeCode, UIStrings.codex, UIStrings.opencode],
            "agent group titles"
        )
        assertEqual(
            groups.map { $0.skills.map(\.id) },
            [["alpha", "beta", "delta", "theta", "zeta"], ["epsilon", "gamma"], ["omega"]],
            "agent group rows"
        )
        assertEqual(
            DisplayText.toggleDisabledReason(for: skills.first { $0.id == "omega" }!, isWriting: false),
            nil,
            "opencode toggle disabled reason"
        )

        assertEqual(
            filter(stateFilter: .enabled).map(\.id),
            ["alpha", "gamma", "omega"],
            "enabled filter"
        )
        assertEqual(
            filter(stateFilter: .disabled).map(\.id),
            ["beta"],
            "disabled filter"
        )
        assertEqual(
            filter(stateFilter: .broken).map(\.id),
            ["delta"],
            "broken filter"
        )
        assertEqual(
            filter(stateFilter: .missing).map(\.id),
            ["epsilon"],
            "missing filter"
        )
        assertEqual(
            filter(stateFilter: .shadowed).map(\.id),
            ["zeta"],
            "shadowed filter"
        )
        assertEqual(
            filter(stateFilter: .unknown).map(\.id),
            ["theta"],
            "unknown filter"
        )
        assertEqual(
            filter(stateFilter: .withFindings).map(\.id),
            ["delta", "epsilon", "gamma", "theta"],
            "problem items filter"
        )

        assertEqual(
            filter(sortOrder: .name).map(\.id),
            ["alpha", "beta", "delta", "epsilon", "gamma", "omega", "theta", "zeta"],
            "sort by name"
        )
        assertEqual(
            filter(sortOrder: .scope).map(\.id),
            ["alpha", "delta", "epsilon", "gamma", "omega", "theta", "zeta", "beta"],
            "sort by scope"
        )
        assertEqual(
            filter(sortOrder: .state).map(\.id),
            ["delta", "epsilon", "beta", "alpha", "gamma", "omega", "zeta", "theta"],
            "sort by state"
        )
        assertEqual(
            filter(sortOrder: .path).map(\.id),
            ["epsilon", "gamma", "alpha", "zeta", "omega", "beta", "delta", "theta"],
            "sort by path"
        )
    }

    private static func filter(
        searchText: String = "",
        agentFilter: SkillAgentFilter = .all,
        stateFilter: SkillStateFilter = .all,
        sortOrder: SkillSortOrder = .name
    ) -> [SkillRecord] {
        SkillListModel.filteredAndSorted(
            skills: skills,
            findings: findings,
            conflicts: conflicts,
            searchText: searchText,
            agentFilter: agentFilter,
            stateFilter: stateFilter,
            sortOrder: sortOrder
        )
    }

    private static let skills: [SkillRecord] = [
        skill(
            id: "beta",
            scope: "agent-project",
            path: "/tmp/project/beta/SKILL.md",
            definitionId: "def.beta",
            name: "Beta",
            state: "loaded",
            enabled: false
        ),
        skill(
            id: "gamma",
            agent: "codex",
            scope: "agent-global",
            path: "/tmp/codex/skills/gamma/SKILL.md",
            definitionId: "codex:gamma",
            name: "Gamma",
            state: "loaded",
            enabled: true
        ),
        skill(
            id: "epsilon",
            agent: "codex",
            scope: "agent-global",
            path: "/tmp/codex/skills/epsilon/SKILL.md",
            definitionId: "codex:epsilon",
            name: "Epsilon",
            state: "missing",
            enabled: false
        ),
        skill(
            id: "alpha",
            scope: "agent-global",
            path: "/tmp/global/alpha/SKILL.md",
            definitionId: "def.alpha",
            name: "Alpha",
            state: "loaded",
            enabled: true
        ),
        skill(
            id: "zeta",
            scope: "agent-global",
            path: "/tmp/global/zeta/SKILL.md",
            definitionId: "def.zeta",
            name: "Zeta",
            state: "shadowed",
            enabled: true
        ),
        skill(
            id: "delta",
            scope: "agent-global",
            path: "/tmp/project/delta/SKILL.md",
            definitionId: "def.delta",
            name: "Delta",
            state: "broken",
            enabled: false
        ),
        skill(
            id: "omega",
            agent: "opencode",
            scope: "agent-global",
            path: "/tmp/opencode/skills/omega/SKILL.md",
            definitionId: "opencode:omega",
            name: "Omega",
            state: "loaded",
            enabled: true
        ),
        skill(
            id: "theta",
            scope: "agent-global",
            path: "/tmp/project/theta/SKILL.md",
            definitionId: "def.theta",
            name: "Theta",
            state: "root-error",
            enabled: false
        ),
    ]

    private static let findings: [RuleFindingRecord] = [
        RuleFindingRecord(
            id: "finding-1",
            instanceId: "gamma",
            definitionId: nil,
            ruleId: "frontmatter.required-fields",
            severity: "warning",
            message: "Missing description",
            suggestion: nil,
            createdAt: 0
        ),
    ]

    private static let conflicts: [ConflictGroupRecord] = [
        ConflictGroupRecord(
            id: "conflict-1",
            definitionId: "def.codex",
            reason: "name-collision",
            winnerId: "gamma",
            instanceIds: ["gamma", "epsilon"]
        ),
    ]

    private static func skill(
        id: String,
        agent: String = "claude-code",
        scope: String,
        path: String,
        definitionId: String,
        name: String,
        state: String,
        enabled: Bool
    ) -> SkillRecord {
        SkillRecord(
            id: id,
            agent: agent,
            scope: scope,
            path: path,
            displayPath: path,
            definitionId: definitionId,
            name: name,
            state: state,
            enabled: enabled
        )
    }

    private static func assertEqual<T: Equatable>(_ actual: T, _ expected: T, _ label: String) {
        if actual != expected {
            fputs("native-list-model-test: \(label) failed: \(actual) != \(expected)\n", stderr)
            exit(1)
        }
    }
}
`;

await writeFile(runnerPath, runnerSource, "utf8");

try {
  run("swiftc", [
    join(repoRoot, "apps/macos/Sources/SkillsCopilot/Models/ListCompleteness.swift"),
    join(repoRoot, "apps/macos/Sources/SkillsCopilot/Models/SkillRecord.swift"),
    join(repoRoot, "apps/macos/Sources/SkillsCopilot/Models/FindingTriageState.swift"),
    join(repoRoot, "apps/macos/Sources/SkillsCopilot/Models/ScriptExecutionPreview.swift"),
    join(repoRoot, "apps/macos/Sources/SkillsCopilot/Support/UIStrings.swift"),
    join(repoRoot, "apps/macos/Sources/SkillsCopilot/Support/Formatters.swift"),
    join(repoRoot, "apps/macos/Sources/SkillsCopilot/Stores/FindingExplainabilityModel.swift"),
    join(repoRoot, "apps/macos/Sources/SkillsCopilot/Stores/SkillListModel.swift"),
    runnerPath,
    "-o",
    binaryPath,
  ]);
  run(binaryPath, []);
  console.log("native-list-model-test: checks passed");
} finally {
  await rm(tempDir, { force: true, recursive: true });
}

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: "pipe",
  });
  if (result.status !== 0) {
    if (result.stdout) process.stdout.write(result.stdout);
    if (result.stderr) process.stderr.write(result.stderr);
    process.exit(result.status ?? 1);
  }
}
