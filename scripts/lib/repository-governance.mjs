import { posix as path } from "node:path";

import {
  collectInlineNodes,
  collectReferenceDefinitions,
  containerStacksEqual,
  parseMarkdownBlocks,
  renderInlineText,
} from "./markdown-governance-parser.mjs";

const ROOT_MARKDOWN_PATHS = new Set([
  "README.md",
  "AGENTS.md",
  "CONTRIBUTING.md",
  "CLAUDE.md",
]);
// `SKILL.md` is a format basename used throughout prose for arbitrary skill
// directories, not a reference to a sibling repository document.
const GENERIC_MARKDOWN_FILENAMES = new Set(["SKILL.md"]);
const REPOSITORY_PREFIX = /^(?:docs|fixtures|\.github|scripts)\//;
const EXTERNAL_DESTINATION = /^(?:https?:|mailto:|data:)/i;

function lineAt(text, offset) {
  let line = 1;
  for (let index = 0; index < offset; index += 1) {
    if (text.charCodeAt(index) === 10) line += 1;
  }
  return line;
}

function safelyDecode(value) {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

function splitDestination(destination) {
  const hash = destination.indexOf("#");
  if (hash === -1) return { pathname: destination, fragment: "" };
  return {
    pathname: destination.slice(0, hash),
    fragment: destination.slice(hash + 1),
  };
}

function normalizeTarget(destination, sourcePath, rootRelative) {
  let target = destination.trim();
  if (target.startsWith("<") && target.endsWith(">")) {
    target = target.slice(1, -1).trim();
  }
  if (!target || EXTERNAL_DESTINATION.test(target)) return undefined;

  const { pathname, fragment } = splitDestination(target);
  const decodedPath = safelyDecode(pathname.replaceAll("\\", "/"));
  let normalizedPath;
  if (!decodedPath) {
    normalizedPath = sourcePath;
  } else if (rootRelative || decodedPath.startsWith("/")) {
    normalizedPath = path.normalize(decodedPath.replace(/^\/+/, ""));
  } else {
    normalizedPath = path.normalize(path.join(path.dirname(sourcePath), decodedPath));
  }
  if (normalizedPath === ".") normalizedPath = sourcePath;
  return fragment
    ? `${normalizedPath}#${safelyDecode(fragment)}`
    : normalizedPath;
}

function repositoryCodeTarget(token, sourcePath) {
  const value = token
    .trim()
    .replace(/:L?\d+(?:-L?\d+)?(?=#|$)/i, "");
  if (!value) return undefined;
  const { pathname } = splitDestination(value);
  if (REPOSITORY_PREFIX.test(pathname)) {
    return normalizeTarget(value, sourcePath, true);
  }
  if (ROOT_MARKDOWN_PATHS.has(pathname)) {
    return normalizeTarget(value, sourcePath, true);
  }
  if (GENERIC_MARKDOWN_FILENAMES.has(pathname)) return undefined;
  if (/^[^/\\]+\.md(?:#.*)?$/i.test(value)) {
    return normalizeTarget(value, sourcePath, false);
  }
  return undefined;
}

export function collectMarkdownReferences(markdown, sourcePath) {
  const source = path.normalize(sourcePath.replaceAll("\\", "/"));
  const document = parseMarkdownBlocks(markdown);
  const text = document.maskedText;
  const references = [];
  const { definitions, text: definitionsMasked } =
    collectReferenceDefinitions(document);
  const { codeSpans, links } = collectInlineNodes(
    definitionsMasked,
    definitions,
  );

  for (const codeSpan of codeSpans) {
    const target = repositoryCodeTarget(codeSpan.content, source);
    if (target) {
      references.push({
        source,
        target,
        line: lineAt(text, codeSpan.start),
        kind: "backtick",
        offset: codeSpan.start,
      });
    }
  }

  for (const link of links) {
    const target = normalizeTarget(link.destination, source, false);
    if (target) {
      references.push({
        source,
        target,
        line: lineAt(text, link.start),
        kind: "markdown",
        offset: link.start,
      });
    }
  }

  return references
    .sort((left, right) => left.offset - right.offset)
    .map(({ offset: _offset, ...reference }) => reference);
}

export function collectDeclaredCreatePaths(markdown, _sourcePath) {
  const text = parseMarkdownBlocks(markdown).maskedText;
  const declared = new Set();
  for (const line of text.split("\n")) {
    const match = line.match(/^- Create: `([^`]+)`\s*$/);
    if (!match) continue;
    const rawTarget = match[1];
    let decodedTarget;
    try {
      decodedTarget = decodeURIComponent(rawTarget);
    } catch {
      continue;
    }
    const repositoryPath = decodedTarget.replaceAll("\\", "/");
    if (
      rawTarget !== rawTarget.trim() ||
      /[\u0000-\u001f\u007f]/u.test(repositoryPath) ||
      /[*?[\]{}]/u.test(repositoryPath) ||
      repositoryPath.startsWith("/") ||
      /^[A-Za-z][A-Za-z\d+.-]*:/u.test(repositoryPath) ||
      repositoryPath.endsWith("/") ||
      /(?:^|\/)\.{1,2}$/u.test(repositoryPath) ||
      repositoryPath.includes("#")
    ) {
      continue;
    }
    const target = path.normalize(repositoryPath);
    if (target === "." || target === ".." || target.startsWith("../")) {
      continue;
    }
    declared.add(target);
  }
  return declared;
}

function targetParts(target) {
  const hash = target.indexOf("#");
  if (hash === -1) return { pathname: target, anchor: "" };
  return {
    pathname: target.slice(0, hash),
    anchor: safelyDecode(target.slice(hash + 1)).toLocaleLowerCase("en-US"),
  };
}

function trackedPathExists(pathname, trackedFiles) {
  if (trackedFiles.has(pathname)) return true;
  if (/[*?]/.test(pathname)) {
    let expression = "^";
    for (let index = 0; index < pathname.length; index += 1) {
      const character = pathname[index];
      if (character === "*" && pathname[index + 1] === "*") {
        expression += ".*";
        index += 1;
      } else if (character === "*") {
        expression += "[^/]*";
      } else if (character === "?") {
        expression += "[^/]";
      } else {
        expression += character.replace(/[\\^$.*+?()[\]{}|]/g, "\\$&");
      }
    }
    const matcher = new RegExp(`${expression}$`);
    for (const tracked of trackedFiles) {
      if (matcher.test(tracked)) return true;
    }
    return false;
  }
  const prefix = pathname.endsWith("/") ? pathname : `${pathname}/`;
  for (const tracked of trackedFiles) {
    if (tracked.startsWith(prefix)) return true;
  }
  return false;
}

export function validateReferences({
  references,
  trackedFiles,
  headingsByFile,
  declaredCreates = new Map(),
}) {
  const errors = [];
  for (const reference of references) {
    const { pathname, anchor } = targetParts(reference.target);
    if (!trackedPathExists(pathname, trackedFiles)) {
      const declared = declaredCreates.get(reference.source);
      const allowedCreate =
        reference.kind === "backtick" && declared?.has(pathname) === true;
      if (!allowedCreate) {
        errors.push(
          `${reference.source}:${reference.line} -> ${reference.target} is missing`,
        );
      }
      continue;
    }
    if (!anchor) continue;
    const headings = headingsByFile.get(pathname);
    if (!headings?.has(anchor)) {
      errors.push(
        `${reference.source}:${reference.line} -> ${reference.target} has no matching anchor`,
      );
    }
  }
  return errors;
}

export function parseGateMembers(command) {
  if (/[\r\n\u2028\u2029]/u.test(command)) {
    throw new Error("gate command must be a single line");
  }
  return command
    .split("&&")
    .map((term) => term.replace(/^[ \t]+|[ \t]+$/gu, ""))
    .map((term) => {
      const match = term.match(
        /^pnpm[ \t]+(?:run[ \t]+)?([A-Za-z0-9@][A-Za-z0-9._:@/-]*)$/,
      );
      if (!match) {
        throw new Error(
          `gate term must be exactly pnpm [run] <script>: ${term}`,
        );
      }
      return match[1];
    });
}

export function validateGateMembers(actual, expected) {
  if (
    actual.length === expected.length &&
    actual.every((member, index) => member === expected[index])
  ) {
    return [];
  }
  return [
    `gate members differ: expected ${expected.join(" -> ")}; actual ${actual.join(" -> ")}`,
  ];
}

export function collectHeadingSlugs(markdown) {
  const document = parseMarkdownBlocks(markdown);
  const { definitions } = collectReferenceDefinitions(document);
  const slugs = new Set();
  const occurrences = new Map();

  function addHeading(rawLabel) {
    const label = renderInlineText(rawLabel, definitions)
      .trim()
      .toLocaleLowerCase("en-US");
    const base = label
      .replace(/[^\p{L}\p{N}\p{M}\s_-]/gu, "")
      .replace(/\s/gu, "-");
    if (!base) return;
    let slug = base;
    let suffix = occurrences.get(base) ?? 0;
    while (occurrences.has(slug)) {
      suffix += 1;
      occurrences.set(base, suffix);
      slug = `${base}-${suffix}`;
    }
    occurrences.set(slug, 0);
    slugs.add(slug);
  }

  for (let index = 0; index < document.lines.length; index += 1) {
    const current = document.lines[index];
    if (current.masked) continue;
    const atx = current.content.match(/^ {0,3}#{1,6}(?:[ \t]+|$)(.*)$/u);
    if (atx) {
      addHeading(atx[1].replace(/[ \t]+#+[ \t]*$/u, ""));
      continue;
    }
    const next = document.lines[index + 1];
    if (
      next &&
      !next.masked &&
      current.content.trim() &&
      containerStacksEqual(current.containers, next.containers) &&
      /^ {0,3}(?:=+|-+)[ \t]*$/u.test(next.content)
    ) {
      addHeading(current.content.trim());
      index += 1;
    }
  }
  return slugs;
}
