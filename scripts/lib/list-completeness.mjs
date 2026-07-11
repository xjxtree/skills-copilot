import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const moduleDirectory = dirname(fileURLToPath(import.meta.url));
const defaultRepoRoot = resolve(moduleDirectory, "../..");
const defaultManifestPath = "scripts/list-completeness-surfaces.json";
const policies = new Set(["complete", "paged", "summary_with_expand"]);

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function hasAttachedAccessibilityIdentifier(source, identifier) {
  const escaped = identifier.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
  const pagingMatch = identifier.match(/^(.*)\.(?:load-more|load-all|cancel)$/u);
  const forwardedLabels = [
    ...source.matchAll(
      new RegExp(
        `\\b([A-Za-z_][A-Za-z0-9_]*AccessibilityIdentifier)\\s*:\\s*["']${escaped}["']`,
        "gu",
      ),
    ),
  ].map((match) => match[1]);
  const forwardedToAccessibilityParameter = forwardedLabels.some((label) =>
    new RegExp(`\\baccessibilityIdentifier\\s*:\\s*${label}\\b`, "u").test(source),
  );
  const helperCalls = [
    ...source.matchAll(
      /\.accessibilityIdentifier\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(/gu,
    ),
  ].map((match) => match[1]);
  const returnedByAccessibilityHelper = helperCalls.some((helper) => {
    const helperStart = source.search(
      new RegExp(`\\bfunc\\s+${helper}\\b`, "u"),
    );
    return helperStart >= 0 &&
      new RegExp(`["']${escaped}["']`, "u").test(source.slice(helperStart));
  });
  return (
    new RegExp(`\\.accessibilityIdentifier\\(\\s*["']${escaped}["']\\s*\\)`, "u").test(source) ||
    new RegExp(`accessibilityIdentifier\\s*:\\s*["']${escaped}["']`, "u").test(source) ||
    forwardedToAccessibilityParameter ||
    returnedByAccessibilityHelper ||
    (pagingMatch !== null &&
      new RegExp(
        `accessibilityIdentifierPrefix\\s*:\\s*["']${pagingMatch[1].replace(/[.*+?^${}()|[\]\\]/gu, "\\$&")}["']`,
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
    for (const field of ["file", "source"]) {
      if (typeof surface[field] !== "string" || !surface[field]) {
        errors.push(`${id}: surface is missing ${field}`);
        missingDeclaration = true;
      }
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
    if (surface.policy === "summary_with_expand" && !surface.full_access_id) {
      errors.push(`${id}: summary_with_expand is missing full_access_id`);
      continue;
    }

    if (repoRoot === undefined) continue;
    const relativePath = surface.file;
    const absolutePath = join(repoRoot, relativePath ?? "");
    if (typeof relativePath !== "string" || !relativePath || !existsSync(absolutePath)) {
      errors.push(`${id}: declared file is missing: ${String(relativePath)}`);
      continue;
    }
    const source = readFileSync(absolutePath, "utf8");
    for (const field of ["status_id", "full_access_id"]) {
      const identifier = surface[field];
      if (
        typeof identifier === "string" &&
        identifier &&
        !hasAttachedAccessibilityIdentifier(source, identifier)
      ) {
        errors.push(
          `${id}: declared ${field} is not attached to an accessibility control: ${identifier}`,
        );
      }
    }
  }
  return errors;
}

export function findUndeclaredPrefixLists(source) {
  const errors = [];
  const lines = source.split(/\r?\n/u);
  const approvedComponentLines = new Set();
  let inApprovedComponent = false;
  let approvedBraceDepth = 0;
  let sawApprovedOpeningBrace = false;
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
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
    if (/\.prefix\s*\(/u.test(expression.split("{", 1)[0])) {
      errors.push(`undeclared prefix-defined formal list at line ${index + 1}`);
    }
  }
  return errors;
}
