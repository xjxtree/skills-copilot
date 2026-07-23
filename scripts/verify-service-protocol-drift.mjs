#!/usr/bin/env node

import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "..");
const methodPattern = /^[A-Za-z][A-Za-z0-9]*\.[A-Za-z][A-Za-z0-9]*$/;
const methodLiteralPattern = /"([A-Za-z][A-Za-z0-9]*\.[A-Za-z][A-Za-z0-9]*)"/g;
const processValues = new Set(["never", "conditional", "always"]);
const networkValues = new Set(["never", "conditional", "always"]);
const confirmationValues = new Set(["none", "required"]);
const effectFieldNames = ["writes", "process", "network", "confirmation"];
const writeValues = new Set([
  "app_data",
  "audit",
  "keychain",
  "agent_config",
  "agent_files",
  "export",
  "external_manager_state",
]);
const writeLabels = new Map([
  ["app_data", "App-local data"],
  ["audit", "Blocked-attempt audit only"],
  ["keychain", "Keychain"],
  ["agent_config", "Agent config"],
  ["agent_files", "Agent skill files"],
  ["export", "Export destination"],
  ["external_manager_state", "External manager state may change when invoked"],
]);
const writeValuesByLabel = new Map(
  [...writeLabels].map(([value, label]) => [label, value]),
);

function titleCase(value) {
  return value.charAt(0).toUpperCase() + value.slice(1);
}

function isPlainObject(value) {
  return typeof value === "object"
    && value !== null
    && !Array.isArray(value)
    && Object.getPrototypeOf(value) === Object.prototype;
}

function parseDocumentedEnum(label, value, allowedValues, method) {
  const parsed = value.toLowerCase();
  if (!allowedValues.has(parsed)) {
    throw new Error(`invalid ${label} value for ${method}`);
  }
  return parsed;
}

export function renderMethodEffectsTable(effects) {
  const rows = [
    "| Method | Local writes | External process | Network | Confirmation |",
    "| --- | --- | --- | --- | --- |",
  ];
  for (const [method, effect] of effects) {
    const writes = effect.writes.length === 0
      ? "None"
      : effect.writes.map((value) => writeLabels.get(value)).join(", ");
    rows.push(
      `| \`${method}\` | ${writes} | ${titleCase(effect.process)} | ${titleCase(effect.network)} | ${titleCase(effect.confirmation)} |`,
    );
  }
  return rows.join("\n");
}

export function replaceMethodEffectsTable(markdown, table) {
  const methodsHeader = markdown.match(/^## Methods[ \t]*$/m);
  if (!methodsHeader) {
    throw new Error("docs/service-protocol.md is missing the ## Methods section");
  }
  const sectionStart = methodsHeader.index + methodsHeader[0].length;
  const afterHeader = markdown.slice(sectionStart);
  const nextHeadingOffset = afterHeader.search(/^##\s+/m);
  if (nextHeadingOffset === -1) {
    throw new Error("docs/service-protocol.md Methods section is missing a following level-two heading");
  }
  return [
    markdown.slice(0, sectionStart),
    "\n\n",
    table,
    "\n\n",
    afterHeader.slice(nextHeadingOffset),
  ].join("");
}

export function parseDocumentedMethodEffects(markdown) {
  const methodsHeader = markdown.match(/^## Methods[ \t]*$/m);
  if (!methodsHeader) {
    throw new Error("docs/service-protocol.md is missing the ## Methods section");
  }

  const sectionStart = methodsHeader.index + methodsHeader[0].length;
  const afterHeader = markdown.slice(sectionStart);
  const nextHeadingOffset = afterHeader.search(/^##\s+/m);
  const section = nextHeadingOffset === -1
    ? afterHeader
    : afterHeader.slice(0, nextHeadingOffset);
  const lines = section.split(/\r?\n/);
  const tableHeader = "| Method | Local writes | External process | Network | Confirmation |";
  const tableIndex = lines.findIndex((line) => line.trim() === tableHeader);
  if (tableIndex === -1 || lines[tableIndex + 1]?.trim() !== "| --- | --- | --- | --- | --- |") {
    throw new Error("docs/service-protocol.md method table must use the five effect columns");
  }

  const effects = new Map();
  for (const line of lines.slice(tableIndex + 2)) {
    if (!line.startsWith("|")) {
      if (effects.size > 0) break;
      continue;
    }
    const columns = line.slice(1, -1).split("|").map((value) => value.trim());
    if (columns.length !== 5) {
      throw new Error(`docs/service-protocol.md has an unparseable method row: ${line}`);
    }
    const methodMatch = columns[0].match(/^`([^`]+)`$/);
    const method = methodMatch?.[1];
    if (!method || !methodPattern.test(method)) {
      throw new Error(`docs/service-protocol.md has an unparseable method row: ${columns[0]}`);
    }
    if (effects.has(method)) {
      throw new Error(`docs/service-protocol.md method table contains duplicate methods: ${method}`);
    }

    const writes = columns[1] === "None"
      ? []
      : columns[1].split(", ").map((label) => {
        const value = writeValuesByLabel.get(label);
        if (!value) throw new Error(`invalid writes value for ${method}`);
        return value;
      }).sort();
    effects.set(method, {
      writes,
      process: parseDocumentedEnum("process", columns[2], processValues, method),
      network: parseDocumentedEnum("network", columns[3], networkValues, method),
      confirmation: parseDocumentedEnum(
        "confirmation",
        columns[4],
        confirmationValues,
        method,
      ),
    });
  }

  if (effects.size === 0) {
    throw new Error("docs/service-protocol.md method table has no parseable methods");
  }
  return effects;
}

export function parseMethodEffects(raw) {
  const parsed = JSON.parse(raw);
  if (!isPlainObject(parsed) || parsed.schema_version !== 2) {
    throw new Error("method effects manifest must use schema_version 2 and an object-valued methods field");
  }
  if (!isPlainObject(parsed.methods)) {
    throw new Error("method effects manifest methods field must be a plain object");
  }
  return new Map(Object.entries(parsed.methods).map(([method, effect]) => {
    if (!methodPattern.test(method)) {
      throw new Error(`invalid method key: ${method}`);
    }
    if (!isPlainObject(effect)) {
      throw new Error(`effect value for ${method} must be a plain object`);
    }
    const fields = Object.keys(effect);
    if (
      fields.length !== effectFieldNames.length
      || effectFieldNames.some((field) => !Object.hasOwn(effect, field))
    ) {
      throw new Error(
        `effect for ${method} must have exactly writes, process, network, confirmation`,
      );
    }
    if (!Array.isArray(effect.writes) || effect.writes.some((value) => !writeValues.has(value))) {
      throw new Error(`invalid writes value for ${method}`);
    }
    if (new Set(effect.writes).size !== effect.writes.length) {
      throw new Error(`duplicate writes value for ${method}`);
    }
    if (!processValues.has(effect.process)) throw new Error(`invalid process value for ${method}`);
    if (!networkValues.has(effect.network)) throw new Error(`invalid network value for ${method}`);
    if (!confirmationValues.has(effect.confirmation)) throw new Error(`invalid confirmation value for ${method}`);
    return [method, {
      writes: [...effect.writes].sort(),
      process: effect.process,
      network: effect.network,
      confirmation: effect.confirmation,
    }];
  }));
}

export function validateMethodEffects({ documentedRows, effects, supportedMethods }) {
  const errors = [];
  const supported = new Set(supportedMethods);
  const missing = supportedMethods.filter((method) => !effects.has(method));
  const extra = [...effects.keys()].filter((method) => !supported.has(method));
  if (missing.length) errors.push(`supported methods missing effect entries: ${missing.sort().join(", ")}`);
  if (extra.length) errors.push(`effect entries missing from SUPPORTED_METHODS: ${extra.sort().join(", ")}`);
  for (const method of supportedMethods) {
    const expected = effects.get(method);
    const documented = documentedRows.get(method);
    if (expected && documented && JSON.stringify(expected) !== JSON.stringify(documented)) {
      errors.push(`${method} documentation differs in writes/process/network/confirmation`);
    }
  }
  return errors;
}

export function writeMethodEffectsDocTable({
  docsPath,
  docsSource,
  effects,
  supportedMethods,
  writeFile,
}) {
  const errors = validateMethodEffects({
    documentedRows: new Map(),
    effects,
    supportedMethods,
  });
  if (errors.length > 0) {
    throw new Error(`method effects manifest failed supported-method preflight: ${errors.join("; ")}`);
  }

  const renderedDocs = replaceMethodEffectsTable(
    docsSource,
    renderMethodEffectsTable(effects),
  );
  if (renderedDocs !== docsSource) {
    writeFile(docsPath, renderedDocs);
  }
  return renderedDocs;
}

function fail(message) {
  console.error(`service protocol drift verification failed: ${message}`);
  process.exit(1);
}

function readRequired(path) {
  if (!existsSync(path)) {
    fail(`missing required file at ${path}`);
  }
  return readFileSync(path, "utf8");
}

function uniqueSorted(values) {
  return [...new Set(values)].sort((a, b) => a.localeCompare(b));
}

function difference(left, right) {
  const rightSet = new Set(right);
  return uniqueSorted(left.filter((value) => !rightSet.has(value)));
}

function union(...sets) {
  return uniqueSorted(sets.flatMap((set) => [...set]));
}

function readFilesRecursively(root, suffix) {
  const files = [];
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const path = join(root, entry.name);
    if (entry.isDirectory()) {
      files.push(...readFilesRecursively(path, suffix));
    } else if (entry.isFile() && path.endsWith(suffix)) {
      files.push(path);
    }
  }
  return files;
}

function collectMethodLiterals(text) {
  return uniqueSorted([...text.matchAll(methodLiteralPattern)].map((match) => match[1]));
}

export function parseActionInventory(raw) {
  const parsed = JSON.parse(raw);
  if (!isPlainObject(parsed)) {
    throw new Error("method effects manifest must be a plain object");
  }
  const topLevelFields = [
    "schema_version",
    "swift_client_methods",
    "blocked_compatibility_methods",
    "action_lifecycle",
    "methods",
  ];
  if (
    parsed.schema_version !== 2
    || Object.keys(parsed).length !== topLevelFields.length
    || topLevelFields.some((field) => !Object.hasOwn(parsed, field))
  ) {
    throw new Error(
      `method effects manifest schema_version 2 must have exactly ${topLevelFields.join(", ")}`,
    );
  }
  const swiftClientMethods = parsed.swift_client_methods;
  const blockedCompatibilityMethods = parsed.blocked_compatibility_methods;
  const actionLifecycle = parsed.action_lifecycle;
  for (const [field, value] of [
    ["swift_client_methods", swiftClientMethods],
    ["blocked_compatibility_methods", blockedCompatibilityMethods],
  ]) {
    if (!Array.isArray(value) || value.some((method) => typeof method !== "string" || !methodPattern.test(method))) {
      throw new Error(`${field} must be an array of service method names`);
    }
    const duplicates = duplicateValues(value);
    if (duplicates.length > 0) {
      throw new Error(`${field} contains duplicate methods: ${duplicates.join(", ")}`);
    }
  }
  if (!isPlainObject(actionLifecycle)) {
    throw new Error("action_lifecycle must be a plain object");
  }
  const lifecycleFieldNames = [
    "type",
    "preview_method",
    "kinds",
    "intents",
    "target_kinds",
    "precondition_kinds",
    "target_agent_binding",
    "target_scope_binding",
    "project_binding",
    "network",
    "impacts",
    "readback",
  ];
  const lifecycle = new Map();
  for (const [method, record] of Object.entries(actionLifecycle)) {
    if (!methodPattern.test(method) || !isPlainObject(record)) {
      throw new Error(`invalid action_lifecycle record for ${method}`);
    }
    if (
      Object.keys(record).length !== lifecycleFieldNames.length
      || lifecycleFieldNames.some((field) => !Object.hasOwn(record, field))
    ) {
      throw new Error(`action_lifecycle record for ${method} must have exactly ${lifecycleFieldNames.join(", ")}`);
    }
    if (!["signed_preview", "explicit_refresh"].includes(record.type)) {
      throw new Error(`invalid lifecycle type for ${method}`);
    }
    if (
      record.preview_method !== null
      && (typeof record.preview_method !== "string" || !methodPattern.test(record.preview_method))
    ) {
      throw new Error(`invalid preview_method for ${method}`);
    }
    for (const field of [
      "kinds",
      "intents",
      "target_kinds",
      "precondition_kinds",
      "impacts",
      "readback",
    ]) {
      if (
        !Array.isArray(record[field])
        || record[field].some((value) => typeof value !== "string" || value.length === 0)
        || new Set(record[field]).size !== record[field].length
      ) {
        throw new Error(`invalid ${field} for ${method}`);
      }
    }
    if (!["none", "required"].includes(record.network)) {
      throw new Error(`invalid action network posture for ${method}`);
    }
    if (
      ![
        "absent",
        "required",
        "request_target_agent",
        "request_single_agent_or_absent",
        "tool_global",
        "claude_code",
      ]
        .includes(record.target_agent_binding)
    ) {
      throw new Error(`invalid target_agent_binding for ${method}`);
    }
    if (
      ![
        "absent",
        "required",
        "request_target_scope",
        "request_scope",
        "project_context_optional",
        "tool_global",
        "agent_global",
      ].includes(record.target_scope_binding)
    ) {
      throw new Error(`invalid target_scope_binding for ${method}`);
    }
    if (
      ![
        "absent",
        "optional",
        "required",
        "matches_target",
        "scope_dependent",
        "project_context_optional",
      ]
        .includes(record.project_binding)
    ) {
      throw new Error(`invalid project_binding for ${method}`);
    }
    lifecycle.set(method, record);
  }
  return {
    swiftClientMethods: [...swiftClientMethods].sort(),
    blockedCompatibilityMethods: [...blockedCompatibilityMethods].sort(),
    lifecycle,
  };
}

export function collectSwiftRPCMethods(swiftRoot) {
  const methods = [];
  const dynamicCalls = [];
  const fallbackFiles = [];
  for (const path of readFilesRecursively(swiftRoot, ".swift")) {
    const source = readRequired(path);
    for (const match of source.matchAll(/\bmethod\s*:\s*"([A-Za-z][A-Za-z0-9]*\.[A-Za-z][A-Za-z0-9]*)"/g)) {
      methods.push(match[1]);
    }
    if (
      !path.endsWith("ServiceClientTransport.swift")
      && /\bcall\s*\(\s*method\s*:\s*(?!")[A-Za-z_]/s.test(source)
    ) {
      dynamicCalls.push(path);
    }
    if (source.includes("unknown_method")) {
      fallbackFiles.push(path);
    }
  }
  return {
    methods: uniqueSorted(methods),
    dynamicCalls: uniqueSorted(dynamicCalls),
    fallbackFiles: uniqueSorted(fallbackFiles),
  };
}

export function validateActionInventory({
  effects,
  inventory,
  supportedMethods,
  swiftRPC,
}) {
  const errors = [];
  const supported = new Set(supportedMethods);
  const consequential = [...effects]
    .filter(([, effect]) => effect.writes.length > 0 || effect.process !== "never" || effect.network !== "never")
    .map(([method]) => method)
    .sort();
  const lifecycleMethods = [...inventory.lifecycle.keys()].sort();
  const missingLifecycle = difference(consequential, lifecycleMethods);
  const extraLifecycle = difference(lifecycleMethods, consequential);
  if (missingLifecycle.length) {
    errors.push(`effectful methods missing action lifecycle declarations: ${missingLifecycle.join(", ")}`);
  }
  if (extraLifecycle.length) {
    errors.push(`action lifecycle declarations without declared effects: ${extraLifecycle.join(", ")}`);
  }

  const blocked = new Set(inventory.blockedCompatibilityMethods);
  for (const method of inventory.blockedCompatibilityMethods) {
    const effect = effects.get(method);
    if (!supported.has(method)) {
      errors.push(`blocked compatibility method is unsupported: ${method}`);
    } else if (
      !effect
      || effect.writes.length > 0
      || effect.process !== "never"
      || effect.network !== "never"
      || effect.confirmation !== "none"
    ) {
      errors.push(`blocked compatibility method is not zero-effect: ${method}`);
    }
  }

  for (const [method, lifecycle] of inventory.lifecycle) {
    const effect = effects.get(method);
    if (!effect || !supported.has(method)) continue;
    if (lifecycle.type === "explicit_refresh") {
      if (
        !["catalog.scanAll", "catalog.scanClaude"].includes(method)
        || lifecycle.preview_method !== null
        || lifecycle.kinds.length > 0
        || lifecycle.intents.length > 0
        || lifecycle.target_kinds.length > 0
        || lifecycle.precondition_kinds.length > 0
        || lifecycle.target_agent_binding !== "absent"
        || lifecycle.target_scope_binding !== "absent"
        || lifecycle.project_binding !== "absent"
        || effect.confirmation !== "none"
      ) {
        errors.push(`${method} has an invalid explicit-refresh lifecycle`);
      }
    } else {
      if (
        !lifecycle.preview_method
        || !supported.has(lifecycle.preview_method)
        || effect.confirmation !== "required"
        || lifecycle.kinds.length === 0
        || lifecycle.intents.length === 0
        || lifecycle.target_kinds.length === 0
        || lifecycle.precondition_kinds.length === 0
        || lifecycle.impacts.length === 0
        || lifecycle.readback.length === 0
      ) {
        errors.push(`${method} has an incomplete signed-preview lifecycle`);
      } else {
        const previewEffect = effects.get(lifecycle.preview_method);
        if (
          !previewEffect
          || (
            lifecycle.preview_method !== method
            && (
              previewEffect.writes.length > 0
              || previewEffect.process !== "never"
              || previewEffect.network !== "never"
              || previewEffect.confirmation !== "none"
            )
          )
        ) {
          errors.push(`${method} preview method is not locally read-only: ${lifecycle.preview_method}`);
        }
      }
    }
    const expectedNetwork = effect.network === "never" ? "none" : "required";
    if (lifecycle.network !== expectedNetwork) {
      errors.push(`${method} lifecycle network posture does not match method effects`);
    }
  }

  const declaredSwift = inventory.swiftClientMethods;
  const missingSwift = difference(declaredSwift, swiftRPC.methods);
  const undeclaredSwift = difference(swiftRPC.methods, declaredSwift);
  const unsupportedSwift = difference(swiftRPC.methods, supportedMethods);
  if (missingSwift.length) {
    errors.push(`declared Swift client methods missing from production sources: ${missingSwift.join(", ")}`);
  }
  if (undeclaredSwift.length) {
    errors.push(`production Swift RPC methods missing from inventory: ${undeclaredSwift.join(", ")}`);
  }
  if (unsupportedSwift.length) {
    errors.push(`production Swift RPC methods missing from SUPPORTED_METHODS: ${unsupportedSwift.join(", ")}`);
  }
  const blockedSwift = swiftRPC.methods.filter((method) => blocked.has(method));
  if (blockedSwift.length) {
    errors.push(`blocked compatibility methods called by production Swift: ${blockedSwift.join(", ")}`);
  }
  for (const [method, lifecycle] of inventory.lifecycle) {
    if (!declaredSwift.includes(method)) {
      errors.push(`effectful lifecycle method is not bound by the Swift client inventory: ${method}`);
    }
    if (lifecycle.preview_method && !declaredSwift.includes(lifecycle.preview_method)) {
      errors.push(`lifecycle preview method is not bound by the Swift client inventory: ${lifecycle.preview_method}`);
    }
  }
  if (swiftRPC.dynamicCalls.length) {
    errors.push(`production Swift contains dynamic RPC method dispatch: ${swiftRPC.dynamicCalls.join(", ")}`);
  }
  if (swiftRPC.fallbackFiles.length) {
    errors.push(`production Swift contains unknown_method fallbacks: ${swiftRPC.fallbackFiles.join(", ")}`);
  }
  return errors;
}

function normalizeScopeBinding(scope) {
  if (scope === "project") return "agent-project";
  if (scope === "global") return "agent-global";
  return scope;
}

function validateFixtureBinding(action, requestParams, lifecycle, method) {
  const errors = [];
  const agent = action?.target?.agent ?? null;
  const scope = action?.target?.scope ?? null;
  const projectID = action?.project_id ?? null;
  const requireEqual = (actual, expected, label) => {
    if (actual !== expected) {
      errors.push(`${method} preview fixture ${label} binding differs from its request`);
    }
  };

  switch (lifecycle.target_agent_binding) {
  case "absent":
    requireEqual(agent, null, "target agent");
    break;
  case "required":
    if (typeof agent !== "string" || agent.length === 0) {
      errors.push(`${method} preview fixture requires a target agent`);
    }
    break;
  case "optional":
    if (projectID !== null && (typeof projectID !== "string" || projectID.length === 0)) {
      errors.push(`${method} preview fixture has an invalid optional project binding`);
    }
    break;
  case "request_target_agent":
    requireEqual(agent, requestParams?.target_agent ?? null, "target agent");
    break;
  case "request_single_agent_or_absent": {
    const agents = Array.isArray(requestParams?.agents) ? requestParams.agents : [];
    requireEqual(agent, agents.length === 1 ? agents[0] : null, "target agent");
    break;
  }
  case "tool_global":
    requireEqual(agent, "tool-global", "target agent");
    break;
  case "claude_code":
    requireEqual(agent, "claude-code", "target agent");
    break;
  default:
    errors.push(`${method} has an unsupported target agent binding`);
  }

  switch (lifecycle.target_scope_binding) {
  case "absent":
    requireEqual(scope, null, "target scope");
    break;
  case "required":
    if (typeof scope !== "string" || scope.length === 0) {
      errors.push(`${method} preview fixture requires a target scope`);
    }
    break;
  case "request_target_scope":
    requireEqual(scope, normalizeScopeBinding(requestParams?.target_scope ?? null), "target scope");
    break;
  case "request_scope":
    requireEqual(scope, normalizeScopeBinding(requestParams?.scope ?? null), "target scope");
    break;
  case "project_context_optional":
    if (
      !(
        (scope === null && projectID === null)
        || (scope === "agent-project" && typeof projectID === "string" && projectID.length > 0)
      )
    ) {
      errors.push(`${method} preview fixture has an invalid optional project-context scope`);
    }
    break;
  case "tool_global":
    requireEqual(scope, "tool-global", "target scope");
    break;
  case "agent_global":
    requireEqual(scope, "agent-global", "target scope");
    break;
  default:
    errors.push(`${method} has an unsupported target scope binding`);
  }

  switch (lifecycle.project_binding) {
  case "absent":
    requireEqual(projectID, null, "project");
    break;
  case "required":
    if (typeof projectID !== "string" || projectID.length === 0) {
      errors.push(`${method} preview fixture requires a project binding`);
    }
    break;
  case "matches_target":
    requireEqual(projectID, action?.target?.id ?? null, "project");
    break;
  case "scope_dependent":
    if (scope === "agent-project") {
      if (typeof projectID !== "string" || projectID.length === 0) {
        errors.push(`${method} project-scoped preview fixture requires a project binding`);
      }
    } else {
      requireEqual(projectID, null, "project");
    }
    break;
  case "project_context_optional":
    if (
      !(
        (projectID === null && scope === null)
        || (typeof projectID === "string" && projectID.length > 0 && scope === "agent-project")
      )
    ) {
      errors.push(`${method} preview fixture has an invalid optional project-context binding`);
    }
    break;
  default:
    errors.push(`${method} has an unsupported project binding`);
  }
  return errors;
}

function fixturePath(fixturesDir, caseName, suffix) {
  return join(fixturesDir, `${caseName}.${suffix}.json`);
}

function actionFromResponse(response) {
  return response?.result?.action ?? response?.result?.preview?.action;
}

function preconditionsFromResponse(response) {
  return response?.result?.preconditions ?? response?.result?.preview?.preconditions;
}

function confirmationFromParams(params) {
  const wrapped = params?.action_confirmation ?? params?.confirmation;
  if (isPlainObject(wrapped)) {
    return {
      confirmed: wrapped.confirmed,
      previewToken: wrapped.preview_token,
      reference: wrapped.reference,
    };
  }
  return {
    confirmed: params?.confirmed,
    previewToken: params?.preview_token,
    reference: params?.action_reference,
  };
}

function sameActionReference(reference, action) {
  return isPlainObject(reference)
    && reference.action_id === action.id
    && reference.source_revision === action.source_revision
    && (reference.project_id ?? null) === (action.project_id ?? null)
    && JSON.stringify(reference.target) === JSON.stringify(action.target);
}

function objectContainsForbiddenKeys(value, forbiddenKeys) {
  const found = [];
  const pending = [value];
  while (pending.length > 0) {
    const current = pending.pop();
    if (Array.isArray(current)) {
      pending.push(...current);
    } else if (isPlainObject(current)) {
      for (const [key, child] of Object.entries(current)) {
        if (forbiddenKeys.has(key)) found.push(key);
        pending.push(child);
      }
    }
  }
  return uniqueSorted(found);
}

function validateLifecycleFixtures(fixturesDir, inventory, swiftRoot) {
  const errors = [];
  for (const [applyMethod, lifecycle] of inventory.lifecycle) {
    if (lifecycle.type === "explicit_refresh") {
      const request = JSON.parse(readRequired(join(fixturesDir, `${applyMethod}.request.json`)));
      if (request?.params?.explicit_refresh !== true) {
        errors.push(`${applyMethod} fixture must carry explicit_refresh=true`);
      }
      continue;
    }
    const previewCase = applyMethod === "skillManager.deleteLocal"
      ? "skillManager.deleteLocal.eligible"
      : lifecycle.preview_method;
    const previewRequest = JSON.parse(
      readRequired(fixturePath(fixturesDir, previewCase, "request")),
    );
    const response = JSON.parse(
      readRequired(fixturePath(fixturesDir, previewCase, "response")),
    );
    const action = actionFromResponse(response);
    if (!isPlainObject(action)) {
      errors.push(`${applyMethod} preview fixture is missing an action descriptor`);
      continue;
    }
    const preconditions = preconditionsFromResponse(response);
    const actualPreconditionKinds = uniqueSorted(
      Array.isArray(preconditions) ? preconditions.map((value) => value?.kind) : [],
    );
    if (
      action.preview_method !== lifecycle.preview_method
      || action.apply_method !== applyMethod
      || !lifecycle.kinds.includes(action.kind)
      || !lifecycle.intents.includes(action.intent)
      || !lifecycle.target_kinds.includes(action?.target?.kind)
      || action.network !== lifecycle.network
      || action.confirmation_required !== true
      || JSON.stringify(actualPreconditionKinds) !== JSON.stringify(
        uniqueSorted(lifecycle.precondition_kinds),
      )
      || !Array.isArray(preconditions)
      || preconditions.some((precondition) => (
        !isPlainObject(precondition)
        || typeof precondition.target_id !== "string"
        || precondition.target_id.length === 0
        || typeof precondition.expected_revision !== "string"
        || precondition.expected_revision.length === 0
      ))
      || JSON.stringify(action.impacts) !== JSON.stringify(lifecycle.impacts)
      || JSON.stringify(action.readback) !== JSON.stringify(lifecycle.readback)
      || !Array.isArray(action.evidence_refs)
      || action.evidence_refs.length === 0
      || typeof action.source_revision !== "string"
      || action.source_revision.length === 0
    ) {
      errors.push(`${applyMethod} preview fixture differs from its action lifecycle declaration`);
    }
    errors.push(...validateFixtureBinding(
      action,
      previewRequest?.params,
      lifecycle,
      applyMethod,
    ));
    if (
      lifecycle.target_agent_binding === "request_single_agent_or_absent"
      && applyMethod === "skillManager.applyInstall"
    ) {
      const singleCase = "skillManager.previewInstall.singleAgent";
      const singleRequest = JSON.parse(
        readRequired(fixturePath(fixturesDir, singleCase, "request")),
      );
      const singleResponse = JSON.parse(
        readRequired(fixturePath(fixturesDir, singleCase, "response")),
      );
      const singleAction = actionFromResponse(singleResponse);
      if (!isPlainObject(singleAction)) {
        errors.push(`${applyMethod} single-agent preview fixture is missing an action descriptor`);
      } else {
        errors.push(...validateFixtureBinding(
          singleAction,
          singleRequest?.params,
          lifecycle,
          `${applyMethod} single-agent`,
        ));
      }
    }

    const applyCase = applyMethod === lifecycle.preview_method
      ? `${applyMethod}.apply`
      : applyMethod;
    const applyRequest = JSON.parse(
      readRequired(fixturePath(fixturesDir, applyCase, "request")),
    );
    const applyResponse = JSON.parse(
      readRequired(fixturePath(fixturesDir, applyCase, "response")),
    );
    const confirmation = confirmationFromParams(applyRequest?.params);
    if (
      confirmation.confirmed !== true
      || typeof confirmation.previewToken !== "string"
      || confirmation.previewToken.length === 0
      || !sameActionReference(confirmation.reference, action)
    ) {
      errors.push(`${applyMethod} apply fixture lacks its exact confirmed preview token and action reference`);
    }
    const leakedAuthorizationKeys = objectContainsForbiddenKeys(
      applyResponse?.result,
      new Set(["preview_token", "action_confirmation", "confirmation"]),
    );
    if (leakedAuthorizationKeys.length > 0) {
      errors.push(
        `${applyMethod} apply response fixture leaks consumed authorization fields: ${leakedAuthorizationKeys.join(", ")}`,
      );
    }
    const returnedAction = actionFromResponse(applyResponse);
    if (
      returnedAction != null
      && JSON.stringify(returnedAction) !== JSON.stringify(action)
    ) {
      errors.push(`${applyMethod} apply response fixture returns a different action descriptor`);
    }
    const readback = applyResponse?.result?.readback;
    const observationDomains = uniqueSorted(
      Array.isArray(readback?.observations)
        ? readback.observations.map((observation) => observation?.domain)
        : [],
    );
    if (
      !isPlainObject(readback)
      || readback.action_id !== action.id
      || (readback.project_id ?? null) !== (action.project_id ?? null)
      || JSON.stringify(readback.domains) !== JSON.stringify(lifecycle.readback)
      || readback.verified !== true
      || !Array.isArray(readback.target_ids)
      || readback.target_ids.length === 0
      || difference(lifecycle.readback, observationDomains).length > 0
    ) {
      errors.push(`${applyMethod} apply response fixture lacks its exact verified read-back`);
    }
  }

  const forbiddenAIKeys = new Set([
    "action_confirmation",
    "action_reference",
    "preview_token",
  ]);
  for (const entry of readdirSync(fixturesDir, { withFileTypes: true })) {
    if (
      entry.isFile()
      && entry.name.startsWith("llm.prepareAction.")
      && entry.name.endsWith(".response.json")
    ) {
      const response = JSON.parse(readRequired(join(fixturesDir, entry.name)));
      for (const key of objectContainsForbiddenKeys(response?.result, forbiddenAIKeys)) {
        errors.push(`llm.prepareAction fixture exposes forbidden authorization field: ${key}`);
      }
    }
  }
  const swiftPrepareModel = readRequired(
    join(swiftRoot, "Models", "SkillRecord.swift"),
  );
  const prepareStruct = swiftPrepareModel.match(
    /struct\s+LLMPrepareResult\b([\s\S]*?)(?=\n(?:struct|enum|final class|class|actor)\s)/,
  )?.[1] ?? "";
  for (const token of ["actionConfirmation", "action_reference", "previewToken"]) {
    if (prepareStruct.includes(token)) {
      errors.push(`Swift LLMPrepareResult exposes forbidden authorization field: ${token}`);
    }
  }

  const blockedLocalDeleteRequest = JSON.parse(
    readRequired(fixturePath(fixturesDir, "skillManager.deleteLocal", "request")),
  );
  const blockedLocalDeleteResponse = JSON.parse(
    readRequired(fixturePath(fixturesDir, "skillManager.deleteLocal", "response")),
  );
  if (
    blockedLocalDeleteRequest?.params?.confirmed !== false
    || blockedLocalDeleteRequest?.params?.preview_token != null
    || blockedLocalDeleteRequest?.params?.action_reference != null
    || blockedLocalDeleteResponse?.result?.physical_delete_allowed !== false
    || blockedLocalDeleteResponse?.result?.action != null
    || blockedLocalDeleteResponse?.result?.preview_token != null
  ) {
    errors.push("skillManager.deleteLocal blocked fixture must remain a zero-authorization preview");
  }
  const eligibleLocalDeleteResponse = JSON.parse(
    readRequired(fixturePath(fixturesDir, "skillManager.deleteLocal.eligible", "response")),
  );
  if (
    eligibleLocalDeleteResponse?.result?.physical_delete_allowed !== true
    || !isPlainObject(eligibleLocalDeleteResponse?.result?.action)
    || typeof eligibleLocalDeleteResponse?.result?.preview_token !== "string"
  ) {
    errors.push("skillManager.deleteLocal eligible fixture must expose a signed action preview");
  }
  return uniqueSorted(errors);
}

function duplicateValues(values) {
  const seen = new Set();
  const duplicates = [];
  for (const value of values) {
    if (seen.has(value)) duplicates.push(value);
    seen.add(value);
  }
  return uniqueSorted(duplicates);
}

function assertNoDuplicates(values, label) {
  const duplicates = duplicateValues(values);
  if (duplicates.length > 0) {
    fail(`${label} contains duplicate methods: ${duplicates.join(", ")}`);
  }
}

function parseSupportedMethods(rustSource, label) {
  const block = rustSource.match(/const\s+SUPPORTED_METHODS\s*:\s*&\s*\[\s*&str\s*\]\s*=\s*&\s*\[([\s\S]*?)\];/);
  if (!block) {
    fail(`${label} SUPPORTED_METHODS block was not parseable`);
  }
  const methods = collectMethodLiterals(block[1]);
  if (methods.length === 0) {
    fail(`${label} SUPPORTED_METHODS block had no parseable methods`);
  }
  return methods;
}

function extractFunctionBody(source, functionName) {
  const signatureIndex = source.indexOf(`fn ${functionName}`);
  if (signatureIndex === -1) {
    fail(`crates/service/src/lib.rs missing fn ${functionName}`);
  }
  const bodyStart = source.indexOf("{", signatureIndex);
  if (bodyStart === -1) {
    fail(`crates/service/src/lib.rs fn ${functionName} body was not parseable`);
  }

  let depth = 0;
  for (let index = bodyStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return source.slice(bodyStart + 1, index);
      }
    }
  }
  fail(`crates/service/src/lib.rs fn ${functionName} body was not closed`);
}

function parseDispatchMethods(rustSource) {
  const body = extractFunctionBody(rustSource, "handle_result");
  const methods = [];
  for (const line of body.split(/\r?\n/)) {
    const arrowIndex = line.indexOf("=>");
    if (arrowIndex === -1) {
      continue;
    }
    const armPattern = /"([A-Za-z][A-Za-z0-9]*\.[A-Za-z][A-Za-z0-9]*)"/g;
    for (const match of line.slice(0, arrowIndex).matchAll(armPattern)) {
      methods.push(match[1]);
    }
  }
  if (methods.length === 0) {
    fail("crates/service/src/lib.rs handle_result dispatch arms had no parseable methods");
  }
  return uniqueSorted(methods);
}

function parseFixtureCase(filename, suffix) {
  if (!filename.endsWith(suffix)) {
    return null;
  }
  const caseName = filename.slice(0, -suffix.length);
  const parts = caseName.split(".");
  if (parts.length < 2) {
    fail(`fixtures/service-protocol filename is not method-shaped: ${filename}`);
  }
  const method = `${parts[0]}.${parts[1]}`;
  if (!methodPattern.test(method)) {
    fail(`fixtures/service-protocol filename has an unparseable method: ${filename}`);
  }
  return { caseName, method };
}

function parseFixtureMethods(fixturesDir) {
  if (!existsSync(fixturesDir)) {
    fail(`missing required fixtures directory at ${fixturesDir}`);
  }
  const requestCases = [];
  const responseCases = [];
  const requestMethods = [];
  const responseMethods = [];

  for (const entry of readdirSync(fixturesDir, { withFileTypes: true })) {
    if (!entry.isFile()) {
      continue;
    }
    const requestCase = parseFixtureCase(entry.name, ".request.json");
    if (requestCase) {
      requestCases.push(requestCase.caseName);
      requestMethods.push(requestCase.method);
      continue;
    }
    const responseCase = parseFixtureCase(entry.name, ".response.json");
    if (responseCase) {
      responseCases.push(responseCase.caseName);
      responseMethods.push(responseCase.method);
    }
  }

  if (requestCases.length === 0 || responseCases.length === 0) {
    fail("fixtures/service-protocol has no parseable request/response filename pairs");
  }

  return {
    requestCases: uniqueSorted(requestCases),
    responseCases: uniqueSorted(responseCases),
    requestMethods: uniqueSorted(requestMethods),
    responseMethods: uniqueSorted(responseMethods),
    methods: union(requestMethods, responseMethods),
  };
}

function parseStatusFixtureMethods(path) {
  let fixture;
  try {
    fixture = JSON.parse(readRequired(path));
  } catch (error) {
    fail(`service.status response fixture is not valid JSON: ${error.message}`);
  }

  const methods = fixture?.result?.supported_methods;
  if (!Array.isArray(methods)) {
    fail("service.status response fixture is missing result.supported_methods");
  }
  for (const method of methods) {
    if (typeof method !== "string" || !methodPattern.test(method)) {
      fail(`service.status response fixture has an unparseable supported method: ${String(method)}`);
    }
  }
  assertNoDuplicates(methods, "service.status response fixture supported_methods");
  return methods;
}

function formatList(values) {
  return values.map((value) => `  - ${value}`).join("\n");
}

function main() {
  const args = process.argv.slice(2);
  const writeDocTable = args.length === 1 && args[0] === "--write-doc-table";
  if (args.length > 0 && !writeDocTable) {
    throw new Error(`unsupported arguments: ${args.join(" ")}`);
  }

  const docsPath = join(repoRoot, "docs", "service-protocol.md");
  const serviceSrcDir = join(repoRoot, "crates", "service", "src");
  const protocolPath = join(repoRoot, "crates", "service", "src", "protocol.rs");
  const fixturesDir = join(repoRoot, "fixtures", "service-protocol");
  const statusFixturePath = join(fixturesDir, "service.status.response.json");
  const effectsPath = join(fixturesDir, "method-effects.json");

  const protocolSource = readRequired(protocolPath);
  const supportedMethods = parseSupportedMethods(
    protocolSource,
    "crates/service/src/protocol.rs",
  );
  const effectsRaw = readRequired(effectsPath);
  const effects = parseMethodEffects(effectsRaw);
  const actionInventory = parseActionInventory(effectsRaw);
  const expectedTable = renderMethodEffectsTable(effects);
  let docsSource = readRequired(docsPath);
  if (writeDocTable) {
    docsSource = writeMethodEffectsDocTable({
      docsPath,
      docsSource,
      effects,
      supportedMethods,
      writeFile: writeFileSync,
    });
  }
  const renderedDocs = replaceMethodEffectsTable(docsSource, expectedTable);

  const documentedRows = parseDocumentedMethodEffects(docsSource);
  const documentedMethods = uniqueSorted([...documentedRows.keys()]);
  const rustSource = [
    "lib.rs",
    "service_host.rs",
    "service_llm.rs",
    "service_app_search.rs",
    "service_observability_helpers.rs",
    "service_support_helpers.rs",
  ].map((file) => readRequired(join(serviceSrcDir, file))).join("\n");
  const dispatchMethods = parseDispatchMethods(rustSource);
  const fixtureMethods = parseFixtureMethods(fixturesDir);
  const statusFixtureMethods = parseStatusFixtureMethods(statusFixturePath);
  const swiftRPC = collectSwiftRPCMethods(
    join(repoRoot, "apps", "macos", "Sources", "SkillsCopilot"),
  );

  const protocolMethods = union(
    supportedMethods,
    dispatchMethods,
    fixtureMethods.methods,
    statusFixtureMethods,
  );
  const errors = [];

  if (docsSource !== renderedDocs) {
    errors.push(["docs/service-protocol.md Methods table differs from method-effects.json", []]);
  }
  for (const error of validateMethodEffects({ documentedRows, effects, supportedMethods })) {
    errors.push([error, []]);
  }
  for (const error of validateActionInventory({
    effects,
    inventory: actionInventory,
    supportedMethods,
    swiftRPC,
  })) {
    errors.push([error, []]);
  }
  for (const error of validateLifecycleFixtures(
    fixturesDir,
    actionInventory,
    join(repoRoot, "apps", "macos", "Sources", "SkillsCopilot"),
  )) {
    errors.push([error, []]);
  }

  const missingDocs = difference(protocolMethods, documentedMethods);
  if (missingDocs.length > 0) {
    errors.push(["methods present in fixtures or Rust service but missing from docs/service-protocol.md", missingDocs]);
  }

  const staleDocs = difference(documentedMethods, protocolMethods);
  if (staleDocs.length > 0) {
    errors.push(["methods documented but absent from fixtures and Rust service", staleDocs]);
  }

  const supportedMissingDispatch = difference(supportedMethods, dispatchMethods);
  if (supportedMissingDispatch.length > 0) {
    errors.push(["SUPPORTED_METHODS entries missing handle_result dispatch arms", supportedMissingDispatch]);
  }

  const dispatchMissingSupported = difference(dispatchMethods, supportedMethods);
  if (dispatchMissingSupported.length > 0) {
    errors.push(["handle_result dispatch arms missing from SUPPORTED_METHODS", dispatchMissingSupported]);
  }

  const unsupportedFixtureMethods = difference(fixtureMethods.methods, supportedMethods);
  if (unsupportedFixtureMethods.length > 0) {
    errors.push(["fixture filenames for methods missing from SUPPORTED_METHODS", unsupportedFixtureMethods]);
  }

  const statusMissingSupported = difference(supportedMethods, statusFixtureMethods);
  if (statusMissingSupported.length > 0) {
    errors.push(["SUPPORTED_METHODS entries missing from service.status response fixture", statusMissingSupported]);
  }

  const statusUnsupportedMethods = difference(statusFixtureMethods, supportedMethods);
  if (statusUnsupportedMethods.length > 0) {
    errors.push(["service.status response fixture methods missing from SUPPORTED_METHODS", statusUnsupportedMethods]);
  }

  const requestCasesMissingResponse = difference(
    fixtureMethods.requestCases,
    fixtureMethods.responseCases,
  );
  if (requestCasesMissingResponse.length > 0) {
    errors.push(["request fixture cases missing matching response fixture cases", requestCasesMissingResponse]);
  }

  const responseCasesMissingRequest = difference(
    fixtureMethods.responseCases,
    fixtureMethods.requestCases,
  );
  if (responseCasesMissingRequest.length > 0) {
    errors.push(["response fixture cases missing matching request fixture cases", responseCasesMissingRequest]);
  }

  if (errors.length > 0) {
    console.error("service protocol drift verification failed");
    for (const [label, values] of errors) {
      console.error(`\n${label}:`);
      if (values.length > 0) console.error(formatList(values));
    }
    process.exit(1);
  }

  console.log(
    [
      "service protocol drift verification passed:",
      `${documentedMethods.length} documented methods,`,
      `${supportedMethods.length} supported methods,`,
      `${dispatchMethods.length} dispatch arms,`,
      `${statusFixtureMethods.length} status fixture methods,`,
      `${effects.size} effect-manifest methods,`,
      `${actionInventory.lifecycle.size} effectful lifecycle declarations,`,
      `${swiftRPC.methods.length} production Swift RPC methods,`,
      `${fixtureMethods.requestCases.length} request fixture cases,`,
      `${fixtureMethods.responseCases.length} response fixture cases`,
    ].join(" "),
  );
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    main();
  } catch (error) {
    fail(error.message);
  }
}
