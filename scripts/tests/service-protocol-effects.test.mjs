import assert from "node:assert/strict";
import test from "node:test";
import {
  parseDocumentedMethodEffects,
  parseMethodEffects,
  replaceMethodEffectsTable,
  renderMethodEffectsTable,
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
  return JSON.stringify({ schema_version: 1, methods });
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

test("renders canonical audit and external-manager write labels", () => {
  const effects = new Map([
    ["script.execute", { ...readOnly, writes: ["audit"], confirmation: "required" }],
    ["skillManager.search", {
      ...readOnly,
      writes: ["external_manager_state"],
      process: "conditional",
      network: "conditional",
    }],
  ]);

  assert.equal(renderMethodEffectsTable(effects), [
    "| Method | Local writes | External process | Network | Confirmation |",
    "| --- | --- | --- | --- | --- |",
    "| `script.execute` | Blocked-attempt audit only | Never | Never | Required |",
    "| `skillManager.search` | External manager state may change when invoked | Conditional | Conditional | None |",
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
    "| `skillManager.search` | External manager state may change when invoked | Conditional | Conditional | None |",
    "",
    "## Provider Observability",
  ].join("\n");

  assert.deepEqual(parseDocumentedMethodEffects(markdown), new Map([
    ["script.execute", { ...readOnly, writes: ["audit"], confirmation: "required" }],
    ["skillManager.search", {
      ...readOnly,
      writes: ["external_manager_state"],
      process: "conditional",
      network: "conditional",
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
