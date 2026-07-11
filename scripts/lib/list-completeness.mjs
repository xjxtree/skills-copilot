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
    const match = /\bif\s+false\s*\{/gu;
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

function buttonBindsIdentifier(source, identifier) {
  const literal = identifierLiteralPattern(identifier);
  for (const match of source.matchAll(/\bButton\s*\(/gu)) {
    const nextButton = source.indexOf("Button(", match.index + match[0].length);
    const end = Math.min(
      nextButton < 0 ? source.length : nextButton,
      match.index + 1800,
    );
    const invocation = source.slice(match.index, end);
    if (/\brole\s*:\s*\.destructive\b/u.test(invocation)) continue;
    if (new RegExp(`\\.accessibilityIdentifier\\(\\s*${literal}\\s*\\)`, "u").test(invocation)) {
      return true;
    }
  }
  return false;
}

function buttonUsesAccessibilityHelper(source, helper) {
  const escaped = helper.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
  for (const match of source.matchAll(/\bButton\s*\(/gu)) {
    const invocation = source.slice(match.index, match.index + 1800);
    if (/\brole\s*:\s*\.destructive\b/u.test(invocation)) continue;
    if (new RegExp(`\\.accessibilityIdentifier\\(\\s*${escaped}\\s*\\(`, "u").test(invocation)) {
      return true;
    }
  }
  return false;
}

function knownExpandableHelperBinds(fileSource, scopeSource, identifier) {
  const literal = identifierLiteralPattern(identifier);
  for (const [helper, argument] of [
    ["BatchToggleItemList", "showAllAccessibilityIdentifier"],
    ["TaskCockpitCandidateList", "accessibilityIdentifier"],
    ["TaskCockpitContextList", "accessibilityIdentifier"],
  ]) {
    const invocation = new RegExp(
      `\\b${helper}\\s*\\([\\s\\S]{0,1800}?\\b${argument}\\s*:\\s*${literal}`,
      "u",
    );
    if (!invocation.test(scopeSource)) continue;
    const range = namedTypeRange(fileSource.structure, helper);
    if (range === undefined) continue;
    const implementation = fileSource.structure.slice(range.start, range.end);
    if (new RegExp(
      `\\bExpandableSummaryList\\s*\\([\\s\\S]{0,1800}?\\baccessibilityIdentifier\\s*:\\s*${argument}\\b`,
      "u",
    ).test(implementation)) {
      return true;
    }
  }
  return false;
}

function hasAttachedAccessibilityIdentifier(fileSource, owner, scope, surface, field) {
  const source = scope.code;
  const identifier = surface[field];
  const escaped = identifier.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
  const pagingMatch = identifier.match(/^(.*)\.(?:load-more|load-all|cancel)$/u);
  const helperCalls = [
    ...source.matchAll(
      /\.accessibilityIdentifier\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(/gu,
    ),
  ].map((match) => match[1]);
  const returnedByAccessibilityHelper = helperCalls.some((helper) => {
    const body = functionBlock(owner, helper);
    if (body === undefined || !new RegExp(`["']${escaped}["']`, "u").test(body)) {
      return false;
    }
    return buttonUsesAccessibilityHelper(source, helper);
  });
  const invokedHelpers = [
    ...source.matchAll(/\b([a-z][A-Za-z0-9_]*)\s*\(/gu),
  ].map((match) => match[1]);
  const attachedInInvokedHelper = invokedHelpers.some((helper) => {
    const body = functionBlock(owner, helper);
    return body !== undefined && buttonBindsIdentifier(body, identifier);
  });
  if (field === "status_id") {
    return new RegExp(
      `(?:ListCompletenessFooter|skillManagerSearchFooter)\\s*\\([\\s\\S]{0,1800}?\\)\\s*\\.accessibilityIdentifier\\(\\s*["']${escaped}["']\\s*\\)`,
      "u",
    ).test(source);
  }
  if (surface.policy === "paged") {
    return attachedInInvokedHelper ||
      (pagingMatch !== null &&
        new RegExp(
          `ListCompletenessFooter\\s*\\([\\s\\S]{0,1800}?accessibilityIdentifierPrefix\\s*:\\s*["']${pagingMatch[1].replace(/[.*+?^${}()|[\]\\]/gu, "\\$&")}["']`,
          "u",
        ).test(source));
  }
  return (
    new RegExp(
      `ExpandableSummaryList\\s*\\([\\s\\S]{0,1600}?\\baccessibilityIdentifier\\s*:\\s*["']${escaped}["']`,
      "u",
    ).test(source) ||
    knownExpandableHelperBinds(fileSource, source, identifier) ||
    returnedByAccessibilityHelper
  );
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
    for (const field of ["status_id", "full_access_id"]) {
      const identifier = surface[field];
      if (
        typeof identifier === "string" &&
        identifier &&
        !hasAttachedAccessibilityIdentifier(fileSource, owner, scope ?? owner, surface, field)
      ) {
        errors.push(
          `${id}: declared ${field} is not attached to an accessibility control in owner ${surface.owner}${surface.control_scope ? ` scope ${surface.control_scope}` : ""}: ${identifier}`,
        );
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

function disclosureRendersRemainder(component) {
  const disclosure = /\bDisclosureGroup\s*(?:\([^)]*\))?\s*\{/gu.exec(component);
  if (disclosure === null) return false;
  const range = blockRangeAt(component, disclosure.index, disclosure[0].length - 1);
  if (range === undefined) return false;
  const content = component.slice(range.opening + 1, range.end - 1);
  return /\bForEach\s*\(\s*Array\s*\(\s*items\.dropFirst\s*\(\s*visibleLimit\s*\)/u.test(content) ||
    /\bForEach\s*\(\s*items\.dropFirst\s*\(\s*visibleLimit\s*\)/u.test(content);
}

function expandableButtonMutatesState(component) {
  for (const button of component.matchAll(/\bButton\s*\(/gu)) {
    const range = blockRangeAt(component, button.index, button[0].length);
    if (range === undefined) continue;
    const action = component.slice(range.opening + 1, range.end - 1);
    if (/\bisExpanded\s*=\s*true\b/u.test(action) || /\bisExpanded\.toggle\s*\(\s*\)/u.test(action)) {
      return true;
    }
  }
  return false;
}

function declarationExpressions(structure) {
  const definitions = new Map();
  const add = (name, expression) => {
    const existing = definitions.get(name) ?? [];
    existing.push(expression);
    definitions.set(name, existing);
  };
  const lines = structure.split(/\r?\n/u);
  for (let index = 0; index < lines.length; index += 1) {
    const declaration = lines[index].match(
      /\b(?:let|var)\s+([A-Za-z_][A-Za-z0-9_]*)[^=\n{]*=\s*(.*)$/u,
    );
    if (declaration === null) continue;
    let expression = declaration[2];
    let continuation = index + 1;
    while (continuation < lines.length && continuation <= index + 30) {
      const trimmed = lines[continuation].trim();
      if (!trimmed.startsWith(".") && !trimmed.startsWith("?.")) break;
      expression += `\n${lines[continuation]}`;
      continuation += 1;
    }
    add(declaration[1], expression);
  }
  for (const method of structure.matchAll(/\bfunc\s+([A-Za-z_][A-Za-z0-9_]*)\b/gu)) {
    const range = blockRangeAt(structure, method.index, method[0].length);
    if (range !== undefined) add(method[1], structure.slice(range.opening + 1, range.end - 1));
  }
  for (const property of structure.matchAll(
    /\bvar\s+([A-Za-z_][A-Za-z0-9_]*)[^=\n{]*\{/gu,
  )) {
    const range = blockRangeAt(structure, property.index, property[0].length - 1);
    if (range !== undefined) add(property[1], structure.slice(range.opening + 1, range.end - 1));
  }
  return definitions;
}

function prefixTaintedNames(structure) {
  const definitions = declarationExpressions(structure);
  const tainted = new Set();
  let changed = true;
  while (changed) {
    changed = false;
    for (const [name, expressions] of definitions) {
      if (tainted.has(name)) continue;
      const isTainted = expressions.some((expression) =>
        /\.prefix\s*\(/u.test(expression) ||
        [...tainted].some((source) => new RegExp(`\\b${source}\\b`, "u").test(expression)),
      );
      if (isTainted) {
        tainted.add(name);
        changed = true;
      }
    }
  }
  return tainted;
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
      /\bForEach\s*\(\s*Array\s*\(\s*items\.prefix\s*\(\s*visibleLimit\s*\)/u.test(component) &&
      disclosureRendersRemainder(component);
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
      expandableButtonMutatesState(component);
    if (canonical) {
      for (const line of linesForRange(structure, expandableRange)) approvedLines.add(line);
    }
  }

  const tainted = prefixTaintedNames(structure);
  const errors = [];
  for (let index = 0; index < lines.length; index += 1) {
    if (approvedLines.has(index)) continue;
    const call = lines[index].match(/\b(?:ForEach|List)\s*\(/u);
    if (call === null) continue;
    let expression = lines[index].slice(call.index);
    let continuation = index + 1;
    while (!expression.includes("{") && continuation < lines.length && continuation <= index + 30) {
      expression += `\n${lines[continuation]}`;
      continuation += 1;
    }
    const collection = expression.split("{", 1)[0];
    if (
      /\.prefix\s*\(/u.test(collection) ||
      [...tainted].some((name) => new RegExp(`\\b${name}\\b`, "u").test(collection))
    ) {
      errors.push(`undeclared prefix-defined formal list at line ${index + 1}`);
    }
  }
  return errors;
}
