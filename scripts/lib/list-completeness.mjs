import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const moduleDirectory = dirname(fileURLToPath(import.meta.url));
const defaultRepoRoot = resolve(moduleDirectory, "../..");
const defaultManifestPath = "scripts/list-completeness-surfaces.json";
const policies = new Set(["complete", "paged", "summary_with_expand"]);
const limitations = new Set([
  "page_failed",
  "safety_budget",
  "source_changed",
  "source_limited",
  "unreadable_source",
  "unsupported_protocol",
]);
const canonicalDenseDisclosurePath =
  "apps/macos/Sources/SkillsCopilot/Views/DetailPresentationPrimitives.swift";
const canonicalExpandableSummaryPath =
  "apps/macos/Sources/SkillsCopilot/Views/ListCompletenessControls.swift";

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function sanitizeSwift(source) {
  const code = source.split("");
  const structure = source.split("");
  let index = 0;
  let blockCommentDepth = 0;
  let rawStringClosing = "";
  let state = "code";
  while (index < source.length) {
    const nextTwo = source.slice(index, index + 2);
    const nextThree = source.slice(index, index + 3);
    if (state === "raw-string") {
      if (rawStringClosing && source.startsWith(rawStringClosing, index)) {
        for (let offset = 0; offset < rawStringClosing.length; offset += 1) {
          code[index + offset] = structure[index + offset] = " ";
        }
        index += rawStringClosing.length;
        rawStringClosing = "";
        state = "code";
      } else {
        if (source[index] !== "\n") code[index] = structure[index] = " ";
        index += 1;
      }
      continue;
    }
    if (state === "line-comment") {
      if (source[index] === "\n") state = "code";
      else code[index] = structure[index] = " ";
      index += 1;
      continue;
    }
    if (state === "block-comment") {
      if (nextTwo === "/*") {
        code[index] = code[index + 1] = " ";
        structure[index] = structure[index + 1] = " ";
        blockCommentDepth += 1;
        index += 2;
      } else if (nextTwo === "*/") {
        code[index] = code[index + 1] = " ";
        structure[index] = structure[index + 1] = " ";
        blockCommentDepth -= 1;
        index += 2;
        if (blockCommentDepth === 0) state = "code";
      } else {
        if (source[index] !== "\n") code[index] = structure[index] = " ";
        index += 1;
      }
      continue;
    }
    if (state === "multiline-string") {
      if (nextThree === '\"\"\"') {
        for (let offset = 0; offset < 3; offset += 1) {
          code[index + offset] = structure[index + offset] = " ";
        }
        index += 3;
        state = "code";
      } else {
        if (source[index] !== "\n") code[index] = structure[index] = " ";
        index += 1;
      }
      continue;
    }
    if (state === "string") {
      structure[index] = source[index] === "\n" ? "\n" : " ";
      if (source[index] === "\\") {
        if (index + 1 < source.length) {
          structure[index + 1] = source[index + 1] === "\n" ? "\n" : " ";
        }
        index += 2;
      } else if (source[index] === '"') {
        state = "code";
        index += 1;
      } else {
        index += 1;
      }
      continue;
    }
    const rawString = source.slice(index).match(/^(#+)("""|")/u);
    if (rawString !== null) {
      const [opening, hashes, quotes] = rawString;
      rawStringClosing = `${quotes}${hashes}`;
      for (let offset = 0; offset < opening.length; offset += 1) {
        code[index + offset] = structure[index + offset] = " ";
      }
      state = "raw-string";
      index += opening.length;
    } else if (nextTwo === "//") {
      code[index] = code[index + 1] = " ";
      structure[index] = structure[index + 1] = " ";
      state = "line-comment";
      index += 2;
    } else if (nextTwo === "/*") {
      code[index] = code[index + 1] = " ";
      structure[index] = structure[index + 1] = " ";
      blockCommentDepth = 1;
      state = "block-comment";
      index += 2;
    } else if (nextThree === '\"\"\"') {
      for (let offset = 0; offset < 3; offset += 1) {
        code[index + offset] = structure[index + offset] = " ";
      }
      state = "multiline-string";
      index += 3;
    } else if (source[index] === '"') {
      structure[index] = " ";
      state = "string";
      index += 1;
    } else {
      index += 1;
    }
  }
  return { code: code.join(""), structure: structure.join("") };
}

function blockRangeAt(structure, declarationStart, declarationLength = 0) {
  const opening = structure.indexOf("{", declarationStart + declarationLength);
  if (opening < 0) return undefined;
  let depth = 0;
  for (let index = opening; index < structure.length; index += 1) {
    if (structure[index] === "{") depth += 1;
    else if (structure[index] === "}") {
      depth -= 1;
      if (depth === 0) return { start: declarationStart, opening, end: index + 1 };
    }
  }
  return undefined;
}

function blockRange(structure, declarationPattern) {
  const declaration = declarationPattern.exec(structure);
  if (declaration === null) return undefined;
  return blockRangeAt(structure, declaration.index, declaration[0].length);
}

function withoutCompileTimeFalseBranches(sanitized) {
  const code = sanitized.code.split("");
  const structure = sanitized.structure.split("");
  let searchFrom = 0;
  while (searchFrom < structure.length) {
    const text = structure.join("");
    const match = /\bif\s+(?:false|\(\s*false\s*\))\s*\{/gu;
    match.lastIndex = searchFrom;
    const declaration = match.exec(text);
    if (declaration === null) break;
    const range = blockRangeAt(text, declaration.index, declaration[0].length - 1);
    if (range === undefined) break;
    for (let index = declaration.index; index < range.end; index += 1) {
      if (code[index] !== "\n") code[index] = " ";
      if (structure[index] !== "\n") structure[index] = " ";
    }
    searchFrom = range.end;
  }
  return { code: code.join(""), structure: structure.join("") };
}

function ownerSource(source, owner) {
  const sanitized = withoutCompileTimeFalseBranches(sanitizeSwift(source));
  const escaped = owner.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
  const range = blockRange(
    sanitized.structure,
    new RegExp(`\\b(?:struct|class|enum|actor|extension)\\s+${escaped}\\b`, "gu"),
  );
  if (range === undefined) return undefined;
  return {
    code: sanitized.code.slice(range.start, range.end),
    structure: sanitized.structure.slice(range.start, range.end),
  };
}

function functionBlock(owner, helper) {
  const escaped = helper.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
  const range = blockRange(
    owner.structure,
    new RegExp(`\\bfunc\\s+${escaped}\\b`, "gu"),
  );
  return range === undefined ? undefined : owner.code.slice(range.start, range.end);
}

function memberSource(owner, member) {
  const escaped = member.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
  const range = blockRange(
    owner.structure,
    new RegExp(`\\bvar\\s+${escaped}\\b[^=\\n{]*`, "gu"),
  );
  if (range === undefined) return undefined;
  return {
    code: owner.code.slice(range.start, range.end),
    structure: owner.structure.slice(range.start, range.end),
  };
}

function identifierLiteralPattern(identifier) {
  const escaped = identifier.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
  return `["']${escaped}["']`;
}

function matchingDelimiter(structure, opening, open = "(", close = ")") {
  if (structure[opening] !== open) return undefined;
  let depth = 0;
  for (let index = opening; index < structure.length; index += 1) {
    if (structure[index] === open) depth += 1;
    else if (structure[index] === close) {
      depth -= 1;
      if (depth === 0) return index;
    }
  }
  return undefined;
}

function skipWhitespace(source, start) {
  let index = start;
  while (index < source.length && /\s/u.test(source[index])) index += 1;
  return index;
}

function callRanges(sanitized, name) {
  const ranges = [];
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
  for (const match of sanitized.structure.matchAll(new RegExp(`\\b${escaped}\\s*\\(`, "gu"))) {
    const opening = sanitized.structure.indexOf("(", match.index);
    const closing = matchingDelimiter(sanitized.structure, opening);
    if (closing === undefined) continue;
    let end = closing + 1;
    let cursor = skipWhitespace(sanitized.structure, end);
    if (sanitized.structure[cursor] === "{") {
      const trailingEnd = matchingDelimiter(sanitized.structure, cursor, "{", "}");
      if (trailingEnd !== undefined) {
        end = trailingEnd + 1;
        cursor = skipWhitespace(sanitized.structure, end);
        const label = sanitized.structure.slice(cursor).match(/^(?:label|content)\s*:\s*\{/u);
        if (label !== null) {
          const labelOpening = sanitized.structure.indexOf("{", cursor);
          const labelEnd = matchingDelimiter(sanitized.structure, labelOpening, "{", "}");
          if (labelEnd !== undefined) end = labelEnd + 1;
        }
      }
    }
    cursor = skipWhitespace(sanitized.structure, end);
    while (sanitized.structure[cursor] === ".") {
      const modifier = sanitized.structure.slice(cursor).match(/^\.[A-Za-z_][A-Za-z0-9_]*\s*\(/u);
      if (modifier === null) break;
      const modifierOpening = sanitized.structure.indexOf("(", cursor);
      const modifierEnd = matchingDelimiter(sanitized.structure, modifierOpening);
      if (modifierEnd === undefined) break;
      end = modifierEnd + 1;
      cursor = skipWhitespace(sanitized.structure, end);
    }
    ranges.push({
      name,
      start: match.index,
      opening,
      closing,
      end,
      code: sanitized.code.slice(match.index, end),
      structure: sanitized.structure.slice(match.index, end),
      arguments: sanitized.structure.slice(opening + 1, closing),
      argumentsCode: sanitized.code.slice(opening + 1, closing),
    });
  }
  return ranges;
}

function argumentSegments(argumentsSource) {
  const segments = [];
  let start = 0;
  const depths = { "(": 0, "[": 0, "{": 0 };
  const closingFor = { ")": "(", "]": "[", "}": "{" };
  for (let index = 0; index <= argumentsSource.length; index += 1) {
    const character = argumentsSource[index];
    if (character in depths) depths[character] += 1;
    else if (character in closingFor) depths[closingFor[character]] -= 1;
    const atBoundary = index === argumentsSource.length ||
      (character === "," && Object.values(depths).every((depth) => depth === 0));
    if (atBoundary) {
      segments.push(argumentsSource.slice(start, index).trim());
      start = index + 1;
    }
  }
  return segments;
}

function callArgument(call, label, useCode = false) {
  const segments = argumentSegments(useCode ? call.argumentsCode : call.arguments);
  if (label === undefined) {
    const first = segments[0] ?? "";
    const labeled = first.match(/^[A-Za-z_][A-Za-z0-9_]*\s*:\s*([\s\S]*)$/u);
    return labeled?.[1] ?? first;
  }
  const escaped = label.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
  for (const segment of segments) {
    const match = segment.match(new RegExp(`^${escaped}\\s*:\\s*([\\s\\S]*)$`, "u"));
    if (match !== null) return match[1];
  }
  return undefined;
}

function normalizedExpression(value) {
  return value.replace(/\s+/gu, "");
}

function declarationBody(structure, name) {
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
  const computed = blockRange(
    structure,
    new RegExp(`\\b(?:var|func)\\s+${escaped}\\b[^=\\n{]*`, "gu"),
  );
  if (computed !== undefined) return structure.slice(computed.start, computed.end);
  const assignment = new RegExp(`\\b(?:let|var)\\s+${escaped}\\b[^=\\n{]*=`, "gu").exec(structure);
  if (assignment === null) return undefined;
  return initializerExpression(structure, assignment.index + assignment[0].length);
}

function expressionDependsOnSource(expression, source, structure, visited = new Set()) {
  const normalizedSource = normalizedExpression(source);
  if (normalizedExpression(expression).includes(normalizedSource)) return true;
  for (const identifier of expression.matchAll(/\b[A-Za-z_][A-Za-z0-9_]*\b/gu)) {
    const name = identifier[0];
    if (visited.has(name)) continue;
    const body = declarationBody(structure, name);
    if (body === undefined) continue;
    const nextVisited = new Set(visited);
    nextVisited.add(name);
    if (expressionDependsOnSource(body, source, structure, nextVisited)) return true;
  }
  return false;
}

function exactIdentifierInCall(call, identifier) {
  const literal = identifierLiteralPattern(identifier);
  return new RegExp(
    `\\.accessibilityIdentifier\\(\\s*${literal}\\s*\\)`,
    "u",
  ).test(call.code) || new RegExp(
    `\\baccessibilityIdentifier\\s*:\\s*${literal}`,
    "u",
  ).test(call.argumentsCode);
}

function sourceHasFormalSink(structure, source) {
  for (const name of ["ForEach", "List", "DenseDisclosureList", "ExpandableSummaryList"]) {
    for (const call of callRanges({ code: structure, structure }, name)) {
      if (expressionDependsOnSource(callArgument(call), source, structure)) return true;
    }
  }
  return false;
}

function pagedControlBinding(fileSource, owner, scope, surface) {
  const prefix = surface.full_access_id.replace(/\.(?:load-more|load-all|cancel|show-all-returned)$/u, "");
  const footers = callRanges(scope, "ListCompletenessFooter");
  let hasStatus = false;
  let hasFull = false;
  for (const footer of footers) {
    const status = exactIdentifierInCall(footer, surface.status_id);
    const declaredPrefix = callArgument(footer, "accessibilityIdentifierPrefix", true);
    const full = declaredPrefix !== undefined &&
      normalizedExpression(declaredPrefix) === normalizedExpression(`"${prefix}"`);
    hasStatus ||= status;
    hasFull ||= full;
    const target = normalizedExpression(footer.structure).includes(
      normalizedExpression(surface.control_anchor),
    );
    if (target && status && full) return { status: true, full: true, sameTarget: true };
  }

  for (const helperCall of callRanges(scope, "skillManagerSearchFooter")) {
    const status = exactIdentifierInCall(helperCall, surface.status_id);
    const helperBody = functionBlock(owner, "skillManagerSearchFooter") ?? "";
    const full = new RegExp(
      `\\.accessibilityIdentifier\\(\\s*${identifierLiteralPattern(surface.full_access_id)}\\s*\\)`,
      "u",
    ).test(helperBody);
    hasStatus ||= status;
    hasFull ||= full;
    const target = normalizedExpression(helperCall.structure).includes(
      normalizedExpression(surface.control_anchor),
    );
    if (target && status && full && sourceHasFormalSink(scope.structure, surface.source)) {
      return { status: true, full: true, sameTarget: true };
    }
  }
  return { status: hasStatus, full: hasFull, sameTarget: false };
}

function knownHelperSummaryBinding(fileSource, owner, scope, surface) {
  for (const definition of [
    { name: "BatchToggleItemList", collectionLabel: "items", idLabel: "showAllAccessibilityIdentifier" },
    { name: "TaskCockpitCandidateList", collectionLabel: "rows", idLabel: "accessibilityIdentifier" },
    { name: "TaskCockpitContextList", collectionLabel: "rows", idLabel: "accessibilityIdentifier" },
  ]) {
    for (const call of callRanges(scope, definition.name)) {
      const collection = callArgument(call, definition.collectionLabel);
      const identifier = callArgument(call, definition.idLabel, true);
      if (identifier === undefined || !new RegExp(`^${identifierLiteralPattern(surface.full_access_id)}$`, "u").test(identifier.trim())) {
        continue;
      }
      const range = namedTypeRange(fileSource.structure, definition.name);
      if (range === undefined) continue;
      const implementation = {
        code: fileSource.code.slice(range.start, range.end),
        structure: fileSource.structure.slice(range.start, range.end),
      };
      const forwardsArguments = callRanges(implementation, "ExpandableSummaryList").some((expandable) =>
        normalizedExpression(callArgument(expandable)) === definition.collectionLabel &&
        normalizedExpression(callArgument(expandable, "accessibilityIdentifier") ?? "") === definition.idLabel,
      );
      if (!forwardsArguments) continue;
      return {
        hasIdentifierControl: true,
        matchesSource: collection !== undefined &&
          expressionDependsOnSource(collection, surface.source, owner.structure),
      };
    }
  }
  return { hasIdentifierControl: false, matchesSource: false };
}

function dynamicButtonSummaryBinding(owner, scope, surface) {
  if (!sourceHasFormalSink(scope.structure, surface.source)) {
    return { hasIdentifierControl: false, matchesSource: false };
  }
  for (const button of callRanges(scope, "Button")) {
    if (/\brole\s*:\s*\.destructive\b/u.test(button.structure)) continue;
    const helper = button.structure.match(
      /\.accessibilityIdentifier\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*\)/u,
    );
    if (helper === null) continue;
    const body = functionBlock(owner, helper[1]);
    if (body === undefined || !new RegExp(identifierLiteralPattern(surface.full_access_id), "u").test(body)) {
      continue;
    }
    const sourceName = surface.source.match(/\b[A-Za-z_][A-Za-z0-9_]*\b/u)?.[0];
    const sourceDefinition = sourceName === undefined ? undefined : declarationBody(scope.structure, sourceName);
    const discriminants = new Set([
      ...(sourceDefinition ?? "").matchAll(/\b[A-Za-z_][A-Za-z0-9_]*\b/gu),
    ].map((match) => match[0]).filter((name) => ![sourceName, "let", "var", "filter", "results"].includes(name)));
    const matchesSource = [...discriminants].some((name) =>
      new RegExp(`\\b${name}\\b`, "u").test(helper[2]) &&
      new RegExp(`\\b${name}\\b`, "u").test(button.structure),
    );
    return { hasIdentifierControl: true, matchesSource };
  }
  return { hasIdentifierControl: false, matchesSource: false };
}

function summaryControlBinding(fileSource, owner, scope, surface) {
  let hasIdentifierControl = false;
  for (const call of callRanges(scope, "ExpandableSummaryList")) {
    if (!exactIdentifierInCall(call, surface.full_access_id)) continue;
    hasIdentifierControl = true;
    if (expressionDependsOnSource(callArgument(call), surface.source, owner.structure)) {
      return { full: true, wrongSource: false };
    }
  }
  const helper = knownHelperSummaryBinding(fileSource, owner, scope, surface);
  hasIdentifierControl ||= helper.hasIdentifierControl;
  if (helper.matchesSource) return { full: true, wrongSource: false };
  const button = dynamicButtonSummaryBinding(owner, scope, surface);
  hasIdentifierControl ||= button.hasIdentifierControl;
  if (button.matchesSource) return { full: true, wrongSource: false };
  return { full: false, wrongSource: hasIdentifierControl };
}

export function loadListCompletenessManifest({
  repoRoot = defaultRepoRoot,
  manifestPath = defaultManifestPath,
} = {}) {
  return JSON.parse(readFileSync(join(repoRoot, manifestPath), "utf8"));
}

export function verifyListSurfaceInventory(manifest, { repoRoot } = {}) {
  const errors = [];
  if (!isObject(manifest) || manifest.schema_version !== 1) {
    return ["list completeness manifest must use schema_version 1"];
  }
  if (!Array.isArray(manifest.surfaces)) {
    return ["list completeness manifest surfaces must be an array"];
  }

  const seen = new Set();
  for (const surface of manifest.surfaces) {
    if (typeof surface?.id !== "string" || !surface.id) {
      errors.push("list completeness surface is missing id");
      continue;
    }
    const id = surface.id;
    if (seen.has(id)) {
      errors.push(`duplicate list completeness surface id: ${id}`);
      continue;
    }
    seen.add(id);

    let missingDeclaration = false;
    for (const field of ["file", "owner", "source", "total_count_source"]) {
      if (typeof surface[field] !== "string" || !surface[field]) {
        errors.push(`${id}: surface is missing ${field}`);
        missingDeclaration = true;
      }
    }
    if (!Array.isArray(surface.allowed_limitations)) {
      errors.push(`${id}: surface is missing allowed_limitations`);
      missingDeclaration = true;
    } else {
      for (const limitation of surface.allowed_limitations) {
        if (!limitations.has(limitation)) {
          errors.push(`${id}: unknown allowed limitation: ${String(limitation)}`);
          missingDeclaration = true;
        }
      }
    }
    if (
      ["paged", "summary_with_expand"].includes(surface.policy) &&
      (typeof surface.control_scope !== "string" || !surface.control_scope)
    ) {
      errors.push(`${id}: ${surface.policy} surface is missing control_scope`);
      missingDeclaration = true;
    }
    if (
      surface.policy === "paged" &&
      (typeof surface.control_anchor !== "string" || !surface.control_anchor)
    ) {
      errors.push(`${id}: paged surface is missing control_anchor`);
      missingDeclaration = true;
    }
    if (missingDeclaration) continue;

    if (!policies.has(surface?.policy)) {
      errors.push(`${id}: unknown list completeness policy: ${String(surface?.policy)}`);
      continue;
    }
    if (surface.policy === "paged" && !surface.full_access_id) {
      errors.push(`${id}: paged surface is missing full_access_id`);
      continue;
    }
    if (surface.policy === "paged" && !surface.status_id) {
      errors.push(`${id}: paged surface is missing status_id`);
      continue;
    }
    if (surface.policy === "summary_with_expand" && !surface.full_access_id) {
      errors.push(`${id}: summary_with_expand is missing full_access_id`);
      continue;
    }

  }

  const fullAccessOwners = new Map();
  for (const surface of manifest.surfaces) {
    if (typeof surface?.full_access_id !== "string" || !surface.full_access_id) continue;
    const prior = fullAccessOwners.get(surface.full_access_id);
    if (prior !== undefined && prior !== surface.id) {
      errors.push(`duplicate list completeness full_access_id: ${surface.full_access_id}`);
    } else {
      fullAccessOwners.set(surface.full_access_id, surface.id);
    }
  }

  if (repoRoot === undefined || errors.length > 0) return errors;
  for (const surface of manifest.surfaces) {
    const id = surface.id;
    const relativePath = surface.file;
    const absolutePath = join(repoRoot, relativePath ?? "");
    if (typeof relativePath !== "string" || !relativePath || !existsSync(absolutePath)) {
      errors.push(`${id}: declared file is missing: ${String(relativePath)}`);
      continue;
    }
    const source = readFileSync(absolutePath, "utf8");
    const fileSource = withoutCompileTimeFalseBranches(sanitizeSwift(source));
    const owner = ownerSource(source, surface.owner);
    if (owner === undefined) {
      errors.push(`${id}: declared owner is missing: ${surface.owner}`);
      continue;
    }
    const scope = typeof surface.control_scope === "string"
      ? memberSource(owner, surface.control_scope)
      : undefined;
    if (surface.control_scope && scope === undefined) {
      errors.push(
        `${id}: declared control scope is missing in owner ${surface.owner}: ${surface.control_scope}`,
      );
      continue;
    }
    const sourceContext = scope ?? owner;
    if (!sourceContext.structure.includes(surface.source)) {
      errors.push(
        `${id}: declared source anchor is not reachable in owner ${surface.owner}${surface.control_scope ? ` scope ${surface.control_scope}` : ""}: ${surface.source}`,
      );
    }
    const controlContext = scope ?? owner;
    const scopeLabel = `owner ${surface.owner}${surface.control_scope ? ` scope ${surface.control_scope}` : ""}`;
    if (surface.policy === "paged") {
      const binding = pagedControlBinding(fileSource, owner, controlContext, surface);
      if (!binding.sameTarget && binding.status && binding.full) {
        errors.push(
          `${id}: declared status_id and full_access_id are not attached to the same target control in ${scopeLabel}`,
        );
      } else if (!binding.sameTarget) {
        if (!binding.status) {
          errors.push(
            `${id}: declared status_id is not attached to an accessibility control in ${scopeLabel}: ${surface.status_id}`,
          );
        }
        if (!binding.full) {
          errors.push(
            `${id}: declared full_access_id is not attached to an accessibility control in ${scopeLabel}: ${surface.full_access_id}`,
          );
        }
      }
    } else if (surface.policy === "summary_with_expand") {
      const binding = summaryControlBinding(fileSource, owner, controlContext, surface);
      if (!binding.full) {
        errors.push(binding.wrongSource
          ? `${id}: declared full_access_id is not attached to the declared source control in ${scopeLabel}: ${surface.full_access_id}`
          : `${id}: declared full_access_id is not attached to an accessibility control in ${scopeLabel}: ${surface.full_access_id}`);
      }
    }
  }
  return errors;
}

function namedTypeRange(structure, name) {
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
  return blockRange(
    structure,
    new RegExp(`\\bstruct\\s+${escaped}\\b`, "gu"),
  );
}

function linesForRange(source, range) {
  const start = source.slice(0, range.start).split(/\r?\n/u).length - 1;
  const end = source.slice(0, range.end).split(/\r?\n/u).length - 1;
  return new Set(Array.from({ length: end - start + 1 }, (_, offset) => start + offset));
}

function trailingClosureBody(call) {
  const relativeClosing = call.closing - call.start;
  const opening = skipWhitespace(call.structure, relativeClosing + 1);
  if (call.structure[opening] !== "{") return undefined;
  const closing = matchingDelimiter(call.structure, opening, "{", "}");
  return closing === undefined ? undefined : call.structure.slice(opening + 1, closing);
}

function forEachRenders(call, collectionPattern) {
  if (!collectionPattern.test(normalizedExpression(callArgument(call)))) return false;
  const body = trailingClosureBody(call) ?? "";
  return /\browContent\s*\(\s*item\s*\)/u.test(body);
}

function denseIsCanonical(component) {
  const sanitized = { code: component, structure: component };
  const loops = callRanges(sanitized, "ForEach");
  const visible = loops.some((call) => forEachRenders(
    call,
    /^Array\(items\.prefix\(visibleLimit\)\.enumerated\(\)\)$/u,
  ));
  if (!visible) return false;
  for (const disclosure of callRanges(sanitized, "DisclosureGroup")) {
    const body = trailingClosureBody(disclosure);
    if (body === undefined) continue;
    const bodySource = { code: body, structure: body };
    if (callRanges(bodySource, "ForEach").some((call) => forEachRenders(
      call,
      /^(?:Array\()?items\.dropFirst\(visibleLimit\)(?:\.enumerated\(\)\))?\)?$/u,
    ))) {
      return true;
    }
  }
  return false;
}

function expandableIdentifiedButtonMutatesState(component) {
  const sanitized = { code: component, structure: component };
  for (const button of callRanges(sanitized, "Button")) {
    if (!/\.accessibilityIdentifier\s*\(\s*accessibilityIdentifier\s*\)/u.test(button.structure)) {
      continue;
    }
    const action = trailingClosureBody(button) ?? "";
    if (/\bisExpanded\s*=\s*true\b/u.test(action) || /\bisExpanded\.toggle\s*\(\s*\)/u.test(action)) {
      return true;
    }
  }
  return false;
}

function initializerExpression(structure, start) {
  const depths = { "(": 0, "[": 0, "{": 0 };
  const closingFor = { ")": "(", "]": "[", "}": "{" };
  for (let index = start; index < structure.length; index += 1) {
    const character = structure[index];
    if (character in depths) depths[character] += 1;
    else if (character in closingFor) depths[closingFor[character]] -= 1;
    if (character !== "\n" || !Object.values(depths).every((depth) => depth === 0)) {
      continue;
    }
    const next = skipWhitespace(structure, index + 1);
    if (structure[next] === "." || structure.slice(next, next + 2) === "?.") continue;
    return structure.slice(start, index);
  }
  return structure.slice(start);
}

function braceDepths(structure) {
  const result = new Array(structure.length + 1).fill(0);
  let depth = 0;
  for (let index = 0; index < structure.length; index += 1) {
    result[index] = depth;
    if (structure[index] === "{") depth += 1;
    else if (structure[index] === "}") depth = Math.max(0, depth - 1);
  }
  result[structure.length] = depth;
  return result;
}

function typeRanges(structure) {
  const ranges = [];
  for (const match of structure.matchAll(
    /\b(?:struct|class|enum|actor|extension)\s+[A-Za-z_][A-Za-z0-9_.]*/gu,
  )) {
    const range = blockRangeAt(structure, match.index, match[0].length);
    if (range !== undefined) ranges.push(range);
  }
  return ranges;
}

function containingRange(ranges, index) {
  return ranges
    .filter((range) => range.start <= index && index < range.end)
    .sort((left, right) => (left.end - left.start) - (right.end - right.start))[0];
}

function directCallableDefinitions(structure, container, depths) {
  const definitions = [];
  const start = container === undefined ? 0 : container.opening + 1;
  const end = container === undefined ? structure.length : container.end - 1;
  const directDepth = container === undefined ? 0 : depths[container.opening] + 1;
  const patterns = [
    /\bfunc\s+([A-Za-z_][A-Za-z0-9_]*)\b/gu,
    /\bvar\s+([A-Za-z_][A-Za-z0-9_]*)[^=\n{]*\{/gu,
  ];
  for (const pattern of patterns) {
    pattern.lastIndex = start;
    while (pattern.lastIndex < end) {
      const match = pattern.exec(structure);
      if (match === null || match.index >= end) break;
      if (depths[match.index] !== directDepth) continue;
      const range = blockRangeAt(structure, match.index, match[0].length - (match[0].endsWith("{") ? 1 : 0));
      if (range === undefined || range.end > end + 1) continue;
      definitions.push({
        name: match[1],
        expression: structure.slice(range.opening + 1, range.end - 1),
        range,
      });
    }
  }
  return definitions;
}

function assignmentDefinitions(structure, range, before) {
  const definitions = [];
  const start = range === undefined ? 0 : range.opening + 1;
  const end = Math.min(before, range === undefined ? before : range.end - 1);
  const pattern = /\b(?:let|var)\s+([A-Za-z_][A-Za-z0-9_]*)[^=\n{]*=/gu;
  pattern.lastIndex = start;
  while (pattern.lastIndex < end) {
    const match = pattern.exec(structure);
    if (match === null || match.index >= end) break;
    definitions.push({
      name: match[1],
      expression: initializerExpression(structure.slice(0, end), match.index + match[0].length),
      position: match.index,
    });
  }
  return definitions;
}

function prefixTaintedNames(definitions) {
  const byName = new Map();
  for (const definition of definitions) byName.set(definition.name, definition.expression);
  const tainted = new Set();
  let changed = true;
  while (changed) {
    changed = false;
    for (const [name, expression] of byName) {
      if (tainted.has(name)) continue;
      const isTainted = /\.prefix\s*\(/u.test(expression) ||
        [...tainted].some((source) => new RegExp(`\\b${source}\\b`, "u").test(expression));
      if (isTainted) {
        tainted.add(name);
        changed = true;
      }
    }
  }
  return tainted;
}

function visibleDefinitionsForSink(structure, sink, types, depths) {
  const container = containingRange(types, sink.start);
  const callableDefinitions = directCallableDefinitions(structure, container, depths);
  const member = containingRange(callableDefinitions.map((definition) => definition.range), sink.start);
  const localDefinitions = assignmentDefinitions(structure, member ?? container, sink.start);
  return [...callableDefinitions, ...localDefinitions];
}

export function findUndeclaredPrefixLists(source, { relativePath } = {}) {
  const sanitized = withoutCompileTimeFalseBranches(sanitizeSwift(source));
  const structure = sanitized.structure;
  const lines = structure.split(/\r?\n/u);
  const approvedLines = new Set();

  const denseRange = namedTypeRange(structure, "DenseDisclosureList");
  if (denseRange !== undefined) {
    const component = structure.slice(denseRange.start, denseRange.end);
    const canonical =
      (relativePath === undefined || relativePath === canonicalDenseDisclosurePath) &&
      denseIsCanonical(component);
    if (canonical) {
      for (const line of linesForRange(structure, denseRange)) approvedLines.add(line);
    }
  }

  const expandableRange = namedTypeRange(structure, "ExpandableSummaryList");
  if (expandableRange !== undefined) {
    const component = structure.slice(expandableRange.start, expandableRange.end);
    const canonical =
      (relativePath === undefined || relativePath === canonicalExpandableSummaryPath) &&
      /\bisExpanded\s*\?\s*items\s*:\s*Array\s*\(\s*items\.prefix\s*\(\s*visibleLimit\s*\)/u.test(component) &&
      /\bForEach\s*\(\s*visibleItems\s*\)/u.test(component) &&
      /\.accessibilityIdentifier\s*\(\s*accessibilityIdentifier\s*\)/u.test(component) &&
      expandableIdentifiedButtonMutatesState(component);
    if (canonical) {
      for (const line of linesForRange(structure, expandableRange)) approvedLines.add(line);
    }
  }

  const errors = [];
  const types = typeRanges(structure);
  const depths = braceDepths(structure);
  const sinks = ["ForEach", "List", "DenseDisclosureList", "ExpandableSummaryList"]
    .flatMap((name) => callRanges(sanitized, name))
    .sort((left, right) => left.start - right.start);
  for (const sink of sinks) {
    const line = structure.slice(0, sink.start).split(/\r?\n/u).length - 1;
    if (approvedLines.has(line)) continue;
    const collection = callArgument(sink);
    if (!collection) continue;
    const tainted = prefixTaintedNames(visibleDefinitionsForSink(
      structure,
      sink,
      types,
      depths,
    ));
    if (
      /\.prefix\s*\(/u.test(collection) ||
      [...tainted].some((name) => new RegExp(`\\b${name}\\b`, "u").test(collection))
    ) {
      errors.push(`undeclared prefix-defined formal list at line ${line + 1}`);
    }
  }
  return errors;
}
