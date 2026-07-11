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
  let state = "code";
  while (index < source.length) {
    const nextTwo = source.slice(index, index + 2);
    const nextThree = source.slice(index, index + 3);
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
    if (nextTwo === "//") {
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

function blockRange(structure, declarationPattern) {
  const declaration = declarationPattern.exec(structure);
  if (declaration === null) return undefined;
  const opening = structure.indexOf("{", declaration.index + declaration[0].length);
  if (opening < 0) return undefined;
  let depth = 0;
  for (let index = opening; index < structure.length; index += 1) {
    if (structure[index] === "{") depth += 1;
    else if (structure[index] === "}") {
      depth -= 1;
      if (depth === 0) return { start: declaration.index, end: index + 1 };
    }
  }
  return undefined;
}

function ownerSource(source, owner) {
  const sanitized = sanitizeSwift(source);
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

function hasAttachedAccessibilityIdentifier(owner, scope, identifier) {
  const source = scope.code;
  const escaped = identifier.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
  const pagingMatch = identifier.match(/^(.*)\.(?:load-more|load-all|cancel)$/u);
  const helperCalls = [
    ...source.matchAll(
      /\.accessibilityIdentifier\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(/gu,
    ),
  ].map((match) => match[1]);
  const returnedByAccessibilityHelper = helperCalls.some((helper) => {
    const body = functionBlock(owner, helper);
    return body !== undefined && new RegExp(`["']${escaped}["']`, "u").test(body);
  });
  const invokedHelpers = [
    ...source.matchAll(/\b([a-z][A-Za-z0-9_]*)\s*\(/gu),
  ].map((match) => match[1]);
  const attachedInInvokedHelper = invokedHelpers.some((helper) => {
    const body = functionBlock(owner, helper);
    return body !== undefined && (
      new RegExp(`\\.accessibilityIdentifier\\(\\s*["']${escaped}["']\\s*\\)`, "u").test(body) ||
      new RegExp(`\\b[A-Za-z_][A-Za-z0-9_]*AccessibilityIdentifier\\s*:\\s*["']${escaped}["']`, "u").test(body)
    );
  });
  return (
    new RegExp(`\\.accessibilityIdentifier\\(\\s*["']${escaped}["']\\s*\\)`, "u").test(source) ||
    new RegExp(`\\b[A-Za-z_][A-Za-z0-9_]*AccessibilityIdentifier\\s*:\\s*["']${escaped}["']`, "u").test(source) ||
    new RegExp(`\\baccessibilityIdentifier\\s*:\\s*["']${escaped}["']`, "u").test(source) ||
    returnedByAccessibilityHelper ||
    attachedInInvokedHelper ||
    (pagingMatch !== null &&
      new RegExp(
        `ListCompletenessFooter\\s*\\([\\s\\S]{0,1200}?accessibilityIdentifierPrefix\\s*:\\s*["']${pagingMatch[1].replace(/[.*+?^${}()|[\]\\]/gu, "\\$&")}["']`,
        "u",
      ).test(source))
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
        !hasAttachedAccessibilityIdentifier(owner, scope ?? owner, identifier)
      ) {
        errors.push(
          `${id}: declared ${field} is not attached to an accessibility control in owner ${surface.owner}${surface.control_scope ? ` scope ${surface.control_scope}` : ""}: ${identifier}`,
        );
      }
    }
  }
  return errors;
}

export function findUndeclaredPrefixLists(source, { relativePath } = {}) {
  const errors = [];
  const sanitized = sanitizeSwift(source);
  const lines = sanitized.code.split(/\r?\n/u);
  const structureLines = sanitized.structure.split(/\r?\n/u);
  const approvedComponentLines = new Set();
  let inApprovedComponent = false;
  let approvedBraceDepth = 0;
  let sawApprovedOpeningBrace = false;
  for (let index = 0; index < lines.length; index += 1) {
    const line = structureLines[index];
    if (!inApprovedComponent && /struct\s+DenseDisclosureList\b/u.test(line)) {
      inApprovedComponent = true;
    }
    if (inApprovedComponent) {
      approvedComponentLines.add(index);
      const openings = line.match(/\{/gu)?.length ?? 0;
      const closings = line.match(/\}/gu)?.length ?? 0;
      if (openings > 0) sawApprovedOpeningBrace = true;
      approvedBraceDepth += openings - closings;
      if (sawApprovedOpeningBrace && approvedBraceDepth === 0) {
        inApprovedComponent = false;
        sawApprovedOpeningBrace = false;
      }
    }
  }
  const denseSource = [...approvedComponentLines].sort((left, right) => left - right)
    .map((index) => lines[index])
    .join("\n");
  const denseIsCanonical =
    approvedComponentLines.size > 0 &&
    (relativePath === undefined || relativePath === canonicalDenseDisclosurePath) &&
    /DisclosureGroup\s*\(/u.test(denseSource) &&
    /items\.prefix\s*\(/u.test(denseSource) &&
    /items\.dropFirst\s*\(/u.test(denseSource);
  if (!denseIsCanonical) approvedComponentLines.clear();

  const expandableLines = new Set();
  let inExpandable = false;
  let expandableDepth = 0;
  let sawExpandableBrace = false;
  for (let index = 0; index < structureLines.length; index += 1) {
    const line = structureLines[index];
    if (!inExpandable && /struct\s+ExpandableSummaryList\b/u.test(line)) {
      inExpandable = true;
    }
    if (!inExpandable) continue;
    expandableLines.add(index);
    const openings = line.match(/\{/gu)?.length ?? 0;
    const closings = line.match(/\}/gu)?.length ?? 0;
    if (openings > 0) sawExpandableBrace = true;
    expandableDepth += openings - closings;
    if (sawExpandableBrace && expandableDepth === 0) {
      inExpandable = false;
      sawExpandableBrace = false;
    }
  }
  const expandableSource = [...expandableLines].sort((left, right) => left - right)
    .map((index) => lines[index])
    .join("\n");
  const expandableIsCanonical =
    expandableLines.size > 0 &&
    (relativePath === undefined || relativePath === canonicalExpandableSummaryPath) &&
    /isExpanded\s*\?\s*items\s*:\s*Array\s*\(\s*items\.prefix/u.test(expandableSource) &&
    /ForEach\s*\(\s*visibleItems\s*\)/u.test(expandableSource) &&
    /\.accessibilityIdentifier\s*\(\s*accessibilityIdentifier\s*\)/u.test(expandableSource);
  if (expandableIsCanonical) {
    for (const line of expandableLines) approvedComponentLines.add(line);
  }

  const truncatedAliases = new Set();
  for (let index = 0; index < lines.length; index += 1) {
    const local = lines[index].match(
      /\b(?:let|var)\s+([A-Za-z_][A-Za-z0-9_]*)[^=\n]*=\s*[^\n]*\.prefix\s*\(/u,
    );
    if (local !== null) truncatedAliases.add(local[1]);
  }
  for (const match of sanitized.structure.matchAll(
    /\bvar\s+([A-Za-z_][A-Za-z0-9_]*)[^=\n{]*\{/gu,
  )) {
    const range = blockRange(
      sanitized.structure,
      new RegExp(`\\bvar\\s+${match[1]}[^=\\n{]*`, "gu"),
    );
    if (range !== undefined && /\.prefix\s*\(/u.test(sanitized.code.slice(range.start, range.end))) {
      truncatedAliases.add(match[1]);
    }
  }
  for (let index = 0; index < lines.length; index += 1) {
    if (approvedComponentLines.has(index)) continue;
    const call = lines[index].match(/\b(?:ForEach|List)\s*\(/u);
    if (call === null) continue;
    let expression = lines[index].slice(call.index);
    let continuation = index + 1;
    while (
      !expression.includes("{") &&
      continuation < lines.length &&
      continuation <= index + 30 &&
      !approvedComponentLines.has(continuation)
    ) {
      expression += `\n${lines[continuation]}`;
      continuation += 1;
    }
    const collectionExpression = expression.split("{", 1)[0];
    const usesTruncatedAlias = [...truncatedAliases].some((alias) =>
      new RegExp(`\\b${alias}\\b`, "u").test(collectionExpression),
    );
    if (/\.prefix\s*\(/u.test(collectionExpression) || usesTruncatedAlias) {
      errors.push(`undeclared prefix-defined formal list at line ${index + 1}`);
    }
  }
  return errors;
}
