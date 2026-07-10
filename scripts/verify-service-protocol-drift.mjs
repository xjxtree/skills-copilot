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
  if (!isPlainObject(parsed) || parsed.schema_version !== 1) {
    throw new Error("method effects manifest must use schema_version 1 and an object-valued methods field");
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

function collectMethodLiterals(text) {
  return uniqueSorted([...text.matchAll(methodLiteralPattern)].map((match) => match[1]));
}

function assertNoDuplicates(values, label) {
  const seen = new Set();
  const duplicates = [];
  for (const value of values) {
    if (seen.has(value)) {
      duplicates.push(value);
    }
    seen.add(value);
  }
  if (duplicates.length > 0) {
    fail(`${label} contains duplicate methods: ${uniqueSorted(duplicates).join(", ")}`);
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
  const effects = parseMethodEffects(readRequired(effectsPath));
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
