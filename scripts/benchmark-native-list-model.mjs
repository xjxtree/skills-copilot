#!/usr/bin/env node

import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import {
  effectiveMaximum,
  loadPerformanceBudgets,
} from "./lib/quality-budgets.mjs";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const performanceBudgets = await loadPerformanceBudgets(
  join(repoRoot, "scripts/performance-budgets.json"),
);
const tempDir = await mkdtemp(join(tmpdir(), "skills-copilot-native-list-bench-"));
const runnerPath = join(tempDir, "NativeListModelBench.swift");
const binaryPath = join(tempDir, "NativeListModelBench");
const iterations = Number(process.env.NATIVE_LIST_BENCH_ITERATIONS ?? 80);
const warmups = Number(process.env.NATIVE_LIST_BENCH_WARMUPS ?? 12);
const maxP95Ms = effectiveMaximum(
  performanceBudgets.native_list.max_p95_ms,
  process.env.NATIVE_LIST_BENCH_MAX_P95_MS,
  process.env.CI === "true",
);

await writeFile(runnerPath, makeRunnerSource({ iterations, maxP95Ms, warmups }), "utf8");

try {
  run("swiftc", [
    "-O",
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
} finally {
  await rm(tempDir, { force: true, recursive: true });
}

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: "inherit",
  });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

function makeRunnerSource({ iterations, maxP95Ms, warmups }) {
  return String.raw`
import Foundation

@main
struct NativeListModelBench {
    static let iterations = ${iterations}
    static let warmups = ${warmups}
    static let maxP95Ms = ${maxP95Ms}
    static let skills = makeCatalog(10_000)
    static let findings = makeFindings(skills)
    static let conflicts = makeConflicts(skills)

    static func main() {
        print("native-list-model-bench: records=\(skills.count) iterations=\(iterations) warmups=\(warmups)")
        print("native-list-model-bench: budget max_p95_ms=\(maxP95Ms)")
        var failed = false
        let scenarios: [(String, () -> Void)] = [
            ("sort:name", {
                _ = runModel(searchText: "", stateFilter: .all, sortOrder: .name)
            }),
            ("query:path-fragment", {
                _ = runModel(searchText: "skill-0099", stateFilter: .all, sortOrder: .name)
            }),
            ("filter:enabled", {
                _ = runModel(searchText: "", agentFilter: .all, stateFilter: .enabled, sortOrder: .name)
            }),
            ("filter:agent-codex", {
                _ = runModel(searchText: "", agentFilter: .codex, stateFilter: .all, sortOrder: .name)
            }),
            ("filter:findings", {
                _ = runModel(searchText: "", agentFilter: .all, stateFilter: .withFindings, sortOrder: .name)
            }),
            ("sort:path", {
                _ = runModel(searchText: "", agentFilter: .all, stateFilter: .all, sortOrder: .path)
            }),
        ]

        for (name, run) in scenarios {
            let samples = measure(run)
            let stats = summarize(samples)
            print(
                "native-list-model-bench: \(name) p50_ms=\(format(stats.p50)) p95_ms=\(format(stats.p95)) max_ms=\(format(stats.max))"
            )
            if stats.p95 > Double(maxP95Ms) {
                failed = true
            }
        }

        if failed {
            fputs("native-list-model-bench: failed p95 threshold \(maxP95Ms)ms\n", stderr)
            exit(1)
        }
    }

    static func runModel(
        searchText: String,
        agentFilter: SkillAgentFilter = .all,
        stateFilter: SkillStateFilter,
        sortOrder: SkillSortOrder
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

    static func measure(_ run: () -> Void) -> [Double] {
        for _ in 0..<warmups {
            run()
        }
        var samples: [Double] = []
        samples.reserveCapacity(iterations)
        for _ in 0..<iterations {
            let start = DispatchTime.now().uptimeNanoseconds
            run()
            let end = DispatchTime.now().uptimeNanoseconds
            samples.append(Double(end - start) / 1_000_000)
        }
        return samples
    }

    static func summarize(_ samples: [Double]) -> (p50: Double, p95: Double, max: Double) {
        let sorted = samples.sorted()
        return (
            p50: percentile(sorted, 0.5),
            p95: percentile(sorted, 0.95),
            max: sorted.last ?? 0
        )
    }

    static func percentile(_ sorted: [Double], _ pct: Double) -> Double {
        if sorted.isEmpty {
            return 0
        }
        let index = min(sorted.count - 1, max(0, Int(ceil(Double(sorted.count) * pct)) - 1))
        return sorted[index]
    }

    static func format(_ value: Double) -> String {
        String(format: "%.2f", value)
    }

    static func makeCatalog(_ count: Int) -> [SkillRecord] {
        var records: [SkillRecord] = []
        records.reserveCapacity(count)
        for index in 0..<count {
            let id = "skill-\(String(format: "%05d", index))"
            let enabled = index % 3 != 0
            let scope = index % 2 == 0 ? "agent-global" : "agent-project"
            let agent = index % 5 == 0 ? "codex" : "claude-code"
            let agentDirectory = agent == "codex" ? ".codex" : ".claude"
            records.append(
                SkillRecord(
                    id: id,
                    agent: agent,
                    scope: scope,
                    path: "/tmp/skills-copilot-bench/\(agentDirectory)/skills/\(id)/SKILL.md",
                    displayPath: "~/\(agentDirectory)/skills/\(id)/SKILL.md",
                    definitionId: "\(agent):\(id)",
                    name: id,
                    state: "loaded",
                    enabled: enabled
                )
            )
        }
        return records
    }

    static func makeFindings(_ skills: [SkillRecord]) -> [RuleFindingRecord] {
        skills.enumerated().compactMap { index, skill in
            guard index % 7 == 0 else {
                return nil
            }
            return RuleFindingRecord(
                id: "finding-\(skill.id)",
                instanceId: skill.id,
                definitionId: skill.definitionId,
                ruleId: "frontmatter.required-fields",
                severity: index % 14 == 0 ? "error" : "warning",
                message: "Synthetic benchmark finding",
                suggestion: nil,
                createdAt: 0
            )
        }
    }

    static func makeConflicts(_ skills: [SkillRecord]) -> [ConflictGroupRecord] {
        skills.enumerated().compactMap { index, skill in
            guard index % 11 == 0 else {
                return nil
            }
            return ConflictGroupRecord(
                id: "conflict-\(skill.id)",
                definitionId: skill.definitionId,
                reason: "name-collision",
                winnerId: skill.id,
                instanceIds: [skill.id]
            )
        }
    }
}
`;
}
