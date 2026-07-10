import { posix as path } from "node:path";

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

function stripFencedCode(markdown) {
  let fence;
  return markdown
    .split("\n")
    .map((line) => {
      const match = line.match(/^ {0,3}(`{3,}|~{3,})/);
      if (fence) {
        if (
          match &&
          match[1][0] === fence.character &&
          match[1].length >= fence.length
        ) {
          fence = undefined;
        }
        return "";
      }
      if (match) {
        fence = { character: match[1][0], length: match[1].length };
        return "";
      }
      return line;
    })
    .join("\n");
}

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
  const text = stripFencedCode(markdown);
  const references = [];
  const markdownLink = /!?\[[^\]\n]*\]\(\s*(<[^>\n]+>|[^\s)]+)(?:\s+(?:"[^"]*"|'[^']*'|\([^)]*\)))?\s*\)/g;
  for (const match of text.matchAll(markdownLink)) {
    const target = normalizeTarget(match[1], source, false);
    if (!target) continue;
    references.push({
      source,
      target,
      line: lineAt(text, match.index),
      kind: "markdown",
      offset: match.index,
    });
  }

  const codeSpan = /(`+)([^`\n]*?)\1/g;
  for (const match of text.matchAll(codeSpan)) {
    const target = repositoryCodeTarget(match[2], source);
    if (!target) continue;
    references.push({
      source,
      target,
      line: lineAt(text, match.index),
      kind: "backtick",
      offset: match.index,
    });
  }

  return references
    .sort((left, right) => left.offset - right.offset)
    .map(({ offset: _offset, ...reference }) => reference);
}

export function collectDeclaredCreatePaths(markdown, sourcePath) {
  const source = path.normalize(sourcePath.replaceAll("\\", "/"));
  const text = stripFencedCode(markdown);
  const declared = new Set();
  for (const line of text.split("\n")) {
    const match = line.match(/^- Create: `([^`]+)`\s*$/);
    if (!match) continue;
    const target = normalizeTarget(match[1], source, true);
    if (target && !target.includes("#")) declared.add(target);
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
  return command
    .split("&&")
    .map((term) => term.trim())
    .map((term) => {
      const match = term.match(
        /^pnpm\s+(?:run\s+)?([^\s;&]+)(?:\s+.*)?$/,
      );
      if (!match) {
        throw new Error(`gate term must be pnpm <script>: ${term}`);
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
  const text = stripFencedCode(markdown);
  const slugs = new Set();
  const occurrences = new Map();
  const lines = text.split("\n");

  function addHeading(rawLabel) {
    const label = rawLabel
      .replace(/<[^>]*>/g, "")
      .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1")
      .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
      .replace(/[`*_~]/g, "")
      .trim()
      .toLocaleLowerCase("en-US");
    const base = label
      .replace(/[^\p{L}\p{N}\p{M}\s_-]/gu, "")
      .replace(/\s+/g, "-");
    if (!base) return;
    const count = occurrences.get(base) ?? 0;
    occurrences.set(base, count + 1);
    slugs.add(count === 0 ? base : `${base}-${count}`);
  }

  for (let index = 0; index < lines.length; index += 1) {
    const atx = lines[index].match(/^ {0,3}#{1,6}\s+(.+?)\s*#*\s*$/);
    if (atx) {
      addHeading(atx[1]);
      continue;
    }
    if (
      index + 1 < lines.length &&
      lines[index].trim() &&
      /^ {0,3}(?:=+|-+)\s*$/.test(lines[index + 1])
    ) {
      addHeading(lines[index].trim());
      index += 1;
    }
  }
  return slugs;
}
