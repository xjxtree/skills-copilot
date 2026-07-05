#!/usr/bin/env node

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "..");
const methodPattern = /^[A-Za-z][A-Za-z0-9]*\.[A-Za-z][A-Za-z0-9]*$/;
const methodLiteralPattern = /"([A-Za-z][A-Za-z0-9]*\.[A-Za-z][A-Za-z0-9]*)"/g;

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

function parseDocumentedMethods(markdown) {
  const methodsHeader = markdown.match(/^## Methods\s*$/m);
  if (!methodsHeader) {
    fail("docs/service-protocol.md is missing the ## Methods section");
  }

  const lines = markdown.slice(methodsHeader.index).split(/\r?\n/);
  const methods = [];
  let inTable = false;
  for (const line of lines) {
    if (!inTable) {
      if (/^\|\s*Method\s*\|/.test(line)) {
        inTable = true;
      }
      continue;
    }
    if (/^\|\s*-+\s*\|/.test(line)) {
      continue;
    }
    if (!line.startsWith("|")) {
      if (methods.length > 0) {
        break;
      }
      continue;
    }
    const row = line.match(/^\|\s*`([^`]+)`\s*\|/);
    if (row) {
      const method = row[1].trim();
      if (!methodPattern.test(method)) {
        fail(`docs/service-protocol.md has an unparseable method row: ${method}`);
      }
      methods.push(method);
    }
  }

  if (methods.length === 0) {
    fail("docs/service-protocol.md method table has no parseable methods");
  }
  assertNoDuplicates(methods, "docs/service-protocol.md method table");
  return uniqueSorted(methods);
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

const docsPath = join(repoRoot, "docs", "service-protocol.md");
const serviceSrcDir = join(repoRoot, "crates", "service", "src");
const protocolPath = join(repoRoot, "crates", "service", "src", "protocol.rs");
const fixturesDir = join(repoRoot, "fixtures", "service-protocol");
const statusFixturePath = join(fixturesDir, "service.status.response.json");

const documentedMethods = parseDocumentedMethods(readRequired(docsPath));
const rustSource = [
  "lib.rs",
  "service_host.rs",
  "service_llm.rs",
  "service_app_search.rs",
  "service_observability_helpers.rs",
  "service_support_helpers.rs",
].map((file) => readRequired(join(serviceSrcDir, file))).join("\n");
const protocolSource = readRequired(protocolPath);
const supportedMethods = parseSupportedMethods(protocolSource, "crates/service/src/protocol.rs");
const dispatchMethods = parseDispatchMethods(rustSource);
const fixtureMethods = parseFixtureMethods(fixturesDir);
const statusFixtureMethods = parseStatusFixtureMethods(statusFixturePath);

const protocolMethods = union(supportedMethods, dispatchMethods, fixtureMethods.methods, statusFixtureMethods);
const errors = [];

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

const requestCasesMissingResponse = difference(fixtureMethods.requestCases, fixtureMethods.responseCases);
if (requestCasesMissingResponse.length > 0) {
  errors.push(["request fixture cases missing matching response fixture cases", requestCasesMissingResponse]);
}

const responseCasesMissingRequest = difference(fixtureMethods.responseCases, fixtureMethods.requestCases);
if (responseCasesMissingRequest.length > 0) {
  errors.push(["response fixture cases missing matching request fixture cases", responseCasesMissingRequest]);
}

if (errors.length > 0) {
  console.error("service protocol drift verification failed");
  for (const [label, values] of errors) {
    console.error(`\n${label}:`);
    console.error(formatList(values));
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
    `${fixtureMethods.requestCases.length} request fixture cases,`,
    `${fixtureMethods.responseCases.length} response fixture cases`,
  ].join(" "),
);
