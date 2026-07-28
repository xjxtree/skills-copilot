import assert from "node:assert/strict";
import test from "node:test";

import { findUnsafeSwiftUIPathSinks } from "../verify-swift-ui-privacy.mjs";

test("flags raw path members in visible SwiftUI sinks", () => {
  const source = `
    Text(document.target)
      .help("\\(skill.name)\\n\\(skill.displayPath)")
      .accessibilityLabel(rootPath)
  `;

  const violations = findUnsafeSwiftUIPathSinks(source);

  assert.deepEqual(
    violations.map((violation) => violation.sink),
    ["Text", ".help", ".accessibilityLabel"],
  );
});

test("flags path aliases appended to user-facing summaries", () => {
  const source = `
    if let project = session.projectRoot {
      parts.append(DisplayText.collapsePath(project, limit: 32))
      lines.append(project)
    }
  `;

  const violations = findUnsafeSwiftUIPathSinks(source, "Views/SidebarView.swift");

  assert.equal(violations.some((violation) => violation.sink === ".append"), true);
  assert.equal(
    violations.some((violation) => violation.sink === "DisplayText.collapsePath"),
    true,
  );
});

test("allows privacy-safe path rendering in visible sinks", () => {
  const source = `
    Text(DisplayText.privacyPath(path, privacyModeEnabled: privacyModeEnabled))
      .help(DisplayText.privacyPath(document.target, privacyModeEnabled: privacyModeEnabled))
      .accessibilityLabel(AgentConfigDisplay.pathSummary(document.target))
    parts.append(DisplayText.configPathSummary(project))
  `;

  assert.deepEqual(
    findUnsafeSwiftUIPathSinks(source, "Views/SidebarView.swift"),
    [],
  );
});

test("ignores comments, ordinary strings, and path argument labels", () => {
  const source = `
    // Text(document.target)
    let example = ".help(skill.displayPath)"
    Text(UIStrings.removeRecentProject(context.name, path: recentProjectPath(context)))
  `;

  assert.deepEqual(findUnsafeSwiftUIPathSinks(source), []);
});

test("handles multiline sink arguments and string interpolation", () => {
  const source = `
    Label(
      "\\(skill.name) \\(skill.displayPath)",
      systemImage: "doc"
    )
  `;

  const violations = findUnsafeSwiftUIPathSinks(source);

  assert.equal(violations.length, 1);
  assert.equal(violations[0].sink, "Label");
});

test("covers custom summary chips that render path values", () => {
  const source = `
    SummaryChip(
      title: UIStrings.target,
      value: snapshot.target,
      systemImage: "scope"
    )
  `;

  const violations = findUnsafeSwiftUIPathSinks(source);

  assert.equal(violations.length, 1);
  assert.equal(violations[0].sink, "SummaryChip");
});
