import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  collectSwiftRPCMethods,
  parseActionInventory,
  parseDocumentedMethodEffects,
  parseMethodEffects,
  replaceMethodEffectsTable,
  renderMethodEffectsTable,
  validateActionInventory,
  validateFixtureBinding,
  validateLifecycleBindingPlacements,
  validateMethodEffects,
  writeMethodEffectsDocTable,
} from "../verify-service-protocol-drift.mjs";

const readOnly = {
  writes: [],
  process: "never",
  network: "never",
  confirmation: "none",
};

function manifestRaw(methods) {
  return JSON.stringify({ schema_version: 2, methods });
}

const signedLifecycle = {
  type: "signed_preview",
  preview_method: "demo.previewWrite",
  kinds: ["save_config"],
  intents: ["save_config"],
  target_kinds: ["config"],
  precondition_kinds: ["agent_config"],
  target_agent_binding: "claude_code",
  target_scope_binding: "agent_global",
  project_binding: "absent",
  network: "none",
  impacts: ["agent_config"],
  readback: ["agent_config"],
};

function actionInventoryRaw(overrides = {}) {
  return JSON.stringify({
    schema_version: 2,
    swift_client_methods: ["demo.previewWrite", "demo.write"],
    blocked_compatibility_methods: [],
    action_lifecycle: {
      "demo.write": signedLifecycle,
    },
    methods: {},
    ...overrides,
  });
}

test("rejects a supported method missing from the effects manifest", () => {
  const errors = validateMethodEffects({
    documentedRows: new Map([["app.version", readOnly]]),
    effects: new Map(),
    supportedMethods: ["app.version"],
  });
  assert.deepEqual(errors, ["supported methods missing effect entries: app.version"]);
});

test("rejects effect entries that are absent from SUPPORTED_METHODS", () => {
  const errors = validateMethodEffects({
    documentedRows: new Map(),
    effects: new Map([["legacy.method", readOnly]]),
    supportedMethods: [],
  });
  assert.deepEqual(errors, ["effect entries missing from SUPPORTED_METHODS: legacy.method"]);
});

test("rejects a documentation row whose side effects differ", () => {
  const effects = new Map([[
    "script.execute",
    { ...readOnly, writes: ["audit"], confirmation: "required" },
  ]]);
  const errors = validateMethodEffects({
    documentedRows: new Map([["script.execute", readOnly]]),
    effects,
    supportedMethods: ["script.execute"],
  });
  assert.match(errors.join("\n"), /script\.execute.*writes.*confirmation/);
});

test("rejects array and non-object methods containers", () => {
  for (const methods of [[], "app.version", 42]) {
    assert.throws(
      () => parseMethodEffects(manifestRaw(methods)),
      /methods field must be a plain object/,
    );
  }
});

test("rejects invalid method keys", () => {
  assert.throws(
    () => parseMethodEffects(manifestRaw({ "app-version": readOnly })),
    /invalid method key: app-version/,
  );
});

test("rejects null, array, and non-object effect values", () => {
  for (const effect of [null, [], "read-only"]) {
    assert.throws(
      () => parseMethodEffects(manifestRaw({ "app.version": effect })),
      /effect value for app\.version must be a plain object/,
    );
  }
});

test("rejects every missing effect field", () => {
  for (const field of ["writes", "process", "network", "confirmation"]) {
    const effect = { ...readOnly };
    delete effect[field];
    assert.throws(
      () => parseMethodEffects(manifestRaw({ "app.version": effect })),
      /effect for app\.version must have exactly writes, process, network, confirmation/,
    );
  }
});

test("rejects extra effect fields", () => {
  assert.throws(
    () => parseMethodEffects(manifestRaw({
      "app.version": { ...readOnly, telemetry: "never" },
    })),
    /effect for app\.version must have exactly writes, process, network, confirmation/,
  );
});

test("rejects duplicate writes", () => {
  assert.throws(
    () => parseMethodEffects(manifestRaw({
      "script.execute": { ...readOnly, writes: ["audit", "audit"] },
    })),
    /duplicate writes value for script\.execute/,
  );
});

test("rejects unsupported write values", () => {
  assert.throws(
    () => parseMethodEffects(manifestRaw({
      "app.version": { ...readOnly, writes: ["project_files"] },
    })),
    /invalid writes value for app\.version/,
  );
});

test("rejects invalid process, network, and confirmation enums", () => {
  for (const [field, value] of [
    ["process", "sometimes"],
    ["network", "sometimes"],
    ["confirmation", "optional"],
  ]) {
    assert.throws(
      () => parseMethodEffects(manifestRaw({
        "app.version": { ...readOnly, [field]: value },
      })),
      new RegExp(`invalid ${field} value for app\\.version`),
    );
  }
});

test("requires the strict schema-version-2 action inventory shape", () => {
  assert.throws(
    () => parseActionInventory(actionInventoryRaw({ schema_version: 1 })),
    /schema_version 2/,
  );
  assert.throws(
    () => parseActionInventory(actionInventoryRaw({ unexpected: true })),
    /must have exactly/,
  );
  const missingPreconditions = { ...signedLifecycle };
  delete missingPreconditions.precondition_kinds;
  assert.throws(
    () => parseActionInventory(actionInventoryRaw({
      action_lifecycle: { "demo.write": missingPreconditions },
    })),
    /must have exactly/,
  );
});

test("requires every effectful method to have a complete lifecycle", () => {
  const inventory = parseActionInventory(actionInventoryRaw({
    swift_client_methods: [],
    action_lifecycle: {},
  }));
  const errors = validateActionInventory({
    effects: new Map([
      ["demo.write", {
        writes: ["app_data"],
        process: "never",
        network: "never",
        confirmation: "required",
      }],
    ]),
    inventory,
    supportedMethods: ["demo.write"],
    swiftRPC: { methods: [], dynamicCalls: [], fallbackFiles: [] },
  });
  assert.match(errors.join("\n"), /effectful methods missing action lifecycle/);
});

test("validates config rollback agent, scope, and project binding combinations", () => {
  const lifecycle = {
    ...signedLifecycle,
    target_agent_binding: "config_agent",
    target_scope_binding: "agent_config_scope",
    project_binding: "scope_dependent_optional",
  };
  const action = (agent, scope, projectID) => ({
    target: { agent, scope },
    project_id: projectID,
  });

  assert.deepEqual(
    validateFixtureBinding(
      action("claude-code", "agent-global", null),
      {},
      lifecycle,
      "snapshot.rollback",
    ),
    [],
  );
  assert.deepEqual(
    validateFixtureBinding(
      action("codex", "agent-global", "project-current"),
      {},
      lifecycle,
      "snapshot.rollback",
    ),
    [],
  );
  assert.deepEqual(
    validateFixtureBinding(
      action("opencode", "agent-project", "project-current"),
      {},
      lifecycle,
      "snapshot.rollback",
    ),
    [],
  );
  assert.match(
    validateFixtureBinding(
      action("opencode", "agent-project", null),
      {},
      lifecycle,
      "snapshot.rollback",
    ).join("\n"),
    /requires a project binding/,
  );
  assert.match(
    validateFixtureBinding(
      action("tool-global", "agent-global", null),
      {},
      lifecycle,
      "snapshot.rollback",
    ).join("\n"),
    /not a supported config agent/,
  );
  assert.match(
    validateFixtureBinding(
      action("codex", "tool-global", null),
      {},
      lifecycle,
      "snapshot.rollback",
    ).join("\n"),
    /not a supported config scope/,
  );
  assert.match(
    validateFixtureBinding(
      action("codex", "agent-project", "project-current"),
      {},
      lifecycle,
      "snapshot.rollback",
    ).join("\n"),
    /unsupported config agent\/scope combination/,
  );
});

test("binds batch toggle action agent and scope to affected items", () => {
  const lifecycle = {
    ...signedLifecycle,
    target_agent_binding: "affected_common_or_absent",
    target_scope_binding: "affected_common_or_absent",
    project_binding: "scope_dependent_optional",
  };
  const mixedAgentGlobal = {
    affected_items: [
      { agent: "claude-code", scope: "agent-global" },
      { agent: "codex", scope: "agent-global" },
    ],
  };
  assert.deepEqual(
    validateFixtureBinding(
      {
        target: { agent: null, scope: "agent-global" },
        project_id: null,
      },
      {},
      lifecycle,
      "batch.applySkillToggles",
      mixedAgentGlobal,
    ),
    [],
  );
  assert.match(
    validateFixtureBinding(
      {
        target: { agent: "claude-code", scope: null },
        project_id: null,
      },
      {},
      lifecycle,
      "batch.applySkillToggles",
      mixedAgentGlobal,
    ).join("\n"),
    /target agent binding differs/,
  );

  const project = {
    affected_items: [
      { agent: "opencode", scope: "agent-project" },
    ],
  };
  assert.deepEqual(
    validateFixtureBinding(
      {
        target: { agent: "opencode", scope: "agent-project" },
        project_id: "project-current",
      },
      {},
      lifecycle,
      "batch.applySkillToggles",
      project,
    ),
    [],
  );
  assert.match(
    validateFixtureBinding(
      {
        target: { agent: "opencode", scope: "agent-project" },
        project_id: null,
      },
      {},
      lifecycle,
      "batch.applySkillToggles",
      project,
    ).join("\n"),
    /requires a project binding/,
  );
});

test("keeps batch-only affected-item bindings off provider lifecycles", () => {
  const providerLifecycle = {
    ...signedLifecycle,
    target_agent_binding: "absent",
    target_scope_binding: "absent",
    project_binding: "absent",
  };
  const inventory = {
    lifecycle: new Map([
      ["llm.saveProviderProfile", providerLifecycle],
      ["batch.applySkillToggles", {
        ...signedLifecycle,
        target_agent_binding: "affected_common_or_absent",
        target_scope_binding: "affected_common_or_absent",
        project_binding: "scope_dependent_optional",
      }],
      ["snapshot.rollback", {
        ...signedLifecycle,
        target_agent_binding: "config_agent",
        target_scope_binding: "agent_config_scope",
        project_binding: "scope_dependent_optional",
      }],
    ]),
  };
  assert.deepEqual(validateLifecycleBindingPlacements(inventory), []);

  inventory.lifecycle.set("llm.saveProviderProfile", {
    ...providerLifecycle,
    target_agent_binding: "affected_common_or_absent",
  });
  inventory.lifecycle.set("batch.applySkillToggles", signedLifecycle);
  inventory.lifecycle.set("llm.deleteProviderProfile", {
    ...signedLifecycle,
    target_scope_binding: "agent_config_scope",
  });
  const errors = validateLifecycleBindingPlacements(inventory);
  assert.ok(errors.includes(
    "llm.saveProviderProfile illegally owns batch affected-item bindings",
  ));
  assert.ok(errors.includes(
    "llm.deleteProviderProfile illegally owns snapshot config bindings",
  ));
  assert.ok(errors.includes(
    "llm.saveProviderProfile lifecycle binding placement is invalid",
  ));
  assert.ok(errors.includes(
    "batch.applySkillToggles lifecycle binding placement is invalid",
  ));
});

test("rejects blocked compatibility methods with effects or Swift calls", () => {
  const inventory = parseActionInventory(actionInventoryRaw({
    swift_client_methods: ["demo.blocked"],
    blocked_compatibility_methods: ["demo.blocked"],
    action_lifecycle: {},
  }));
  const errors = validateActionInventory({
    effects: new Map([[
      "demo.blocked",
      {
        writes: ["audit"],
        process: "never",
        network: "never",
        confirmation: "none",
      },
    ]]),
    inventory,
    supportedMethods: ["demo.blocked"],
    swiftRPC: {
      methods: ["demo.blocked"],
      dynamicCalls: [],
      fallbackFiles: [],
    },
  });
  assert.match(errors.join("\n"), /not zero-effect/);
  assert.match(errors.join("\n"), /called by production Swift/);
});

test("detects dynamic Swift RPC dispatch and unknown-method fallback branches", () => {
  const directory = mkdtempSync(join(tmpdir(), "service-inventory-"));
  try {
    writeFileSync(
      join(directory, "Client.swift"),
      [
        "func run(method: String) async throws {",
        "  _ = try await call(method: method, params: Empty())",
        "  if error.code == \"unknown_method\" {}",
        "}",
      ].join("\n"),
    );
    const collected = collectSwiftRPCMethods(directory);
    assert.equal(collected.dynamicCalls.length, 1);
    assert.equal(collected.fallbackFiles.length, 1);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("renders canonical audit and external-manager write labels", () => {
  const effects = new Map([
    ["script.execute", { ...readOnly, writes: ["audit"], confirmation: "required" }],
    ["skillManager.applySearch", {
      ...readOnly,
      writes: ["app_data", "external_manager_state"],
      process: "always",
      network: "always",
      confirmation: "required",
    }],
  ]);

  assert.equal(renderMethodEffectsTable(effects), [
    "| Method | Local writes | External process | Network | Confirmation |",
    "| --- | --- | --- | --- | --- |",
    "| `script.execute` | Blocked-attempt audit only | Never | Never | Required |",
    "| `skillManager.applySearch` | App-local data, External manager state may change when invoked | Always | Always | Required |",
  ].join("\n"));
});

test("parses all five documentation columns back to effect enums", () => {
  const markdown = [
    "# Service Protocol",
    "",
    "## Methods",
    "",
    "| Method | Local writes | External process | Network | Confirmation |",
    "| --- | --- | --- | --- | --- |",
    "| `script.execute` | Blocked-attempt audit only | Never | Never | Required |",
    "| `skillManager.applySearch` | App-local data, External manager state may change when invoked | Always | Always | Required |",
    "",
    "## Provider Observability",
  ].join("\n");

  assert.deepEqual(parseDocumentedMethodEffects(markdown), new Map([
    ["script.execute", { ...readOnly, writes: ["audit"], confirmation: "required" }],
    ["skillManager.applySearch", {
      ...readOnly,
      writes: ["app_data", "external_manager_state"],
      process: "always",
      network: "always",
      confirmation: "required",
    }],
  ]));
});

test("replaces only the Methods section content", () => {
  const markdown = [
    "# Service Protocol",
    "",
    "Before methods.",
    "",
    "## Methods",
    "",
    "old table",
    "",
    "## Provider Observability",
    "",
    "After methods.",
    "",
  ].join("\n");
  const table = [
    "| Method | Local writes | External process | Network | Confirmation |",
    "| --- | --- | --- | --- | --- |",
    "| `app.version` | None | Never | Never | None |",
  ].join("\n");

  assert.equal(replaceMethodEffectsTable(markdown, table), [
    "# Service Protocol",
    "",
    "Before methods.",
    "",
    "## Methods",
    "",
    table,
    "",
    "## Provider Observability",
    "",
    "After methods.",
    "",
  ].join("\n"));
});

test("preflights supported methods before writing the documentation table", () => {
  let writeCalls = 0;
  const effects = new Map([
    ["app.version", readOnly],
    ["legacy.method", readOnly],
  ]);

  assert.throws(
    () => writeMethodEffectsDocTable({
      docsPath: "docs/service-protocol.md",
      docsSource: "not valid protocol Markdown",
      effects,
      supportedMethods: ["app.version"],
      writeFile() {
        writeCalls += 1;
      },
    }),
    /effect entries missing from SUPPORTED_METHODS: legacy\.method/,
  );
  assert.equal(writeCalls, 0);
});
