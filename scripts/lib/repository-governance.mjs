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
const HTML_ENTITIES = new Map([
  ["amp", "&"],
  ["apos", "'"],
  ["gt", ">"],
  ["lt", "<"],
  ["nbsp", "\u00a0"],
  ["quot", '"'],
]);

function parseContainerLine(line) {
  let offset = line.match(/^ {0,3}/u)?.[0].length ?? 0;
  let quoteDepth = 0;
  while (line[offset] === ">") {
    quoteDepth += 1;
    offset += 1;
    if (line[offset] === " " || line[offset] === "\t") offset += 1;
    const indentation = line.slice(offset).match(/^ {0,3}/u)?.[0].length ?? 0;
    offset += indentation;
  }

  const listMarkerOffset = offset;
  const listMarker = line
    .slice(offset)
    .match(/^(?:[-+*]|\d{1,9}[.)])(?:[ \t]+|$)/u);
  if (listMarker) offset += listMarker[0].length;

  return {
    content: line.slice(offset),
    contentOffset: offset,
    hasListMarker: Boolean(listMarker),
    listMarkerOffset: listMarker ? listMarkerOffset : undefined,
    quoteDepth,
  };
}

function stripFencedCode(markdown) {
  let fence;
  return markdown
    .split("\n")
    .map((line) => {
      const parsed = parseContainerLine(line);
      const content = parsed.content.replace(/\r$/u, "");
      if (fence) {
        const containerEnded =
          parsed.quoteDepth < fence.quoteDepth ||
          (fence.listContentOffset !== undefined &&
            content.trim() &&
            (parsed.listMarkerOffset ?? parsed.contentOffset) <
              fence.listContentOffset);
        if (!containerEnded) {
          const close = content.match(/^(`{3,}|~{3,})[ \t]*$/u);
          if (
            close &&
            close[1][0] === fence.character &&
            close[1].length >= fence.length
          ) {
            fence = undefined;
          }
          return "";
        }
        fence = undefined;
      }
      const open = content.match(/^(`{3,}|~{3,})(.*)$/u);
      if (open && !(open[1][0] === "`" && open[2].includes("`"))) {
        fence = {
          character: open[1][0],
          length: open[1].length,
          listContentOffset: parsed.hasListMarker
            ? parsed.contentOffset
            : undefined,
          quoteDepth: parsed.quoteDepth,
        };
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

function decodeHtmlEntities(value) {
  return value.replace(
    /&(?:#(\d+)|#x([\da-f]+)|([a-z][a-z\d]+));/giu,
    (entity, decimal, hexadecimal, named) => {
      if (decimal !== undefined) {
        const codePoint = Number.parseInt(decimal, 10);
        return codePoint <= 0x10ffff ? String.fromCodePoint(codePoint) : entity;
      }
      if (hexadecimal !== undefined) {
        const codePoint = Number.parseInt(hexadecimal, 16);
        return codePoint <= 0x10ffff ? String.fromCodePoint(codePoint) : entity;
      }
      return HTML_ENTITIES.get(named.toLocaleLowerCase("en-US")) ?? entity;
    },
  );
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

function normalizeReferenceLabel(label) {
  return decodeHtmlEntities(label)
    .replace(/\\([!"#$%&'()*+,./:;<=>?@[\]^_`{|}~-])/gu, "$1")
    .trim()
    .replace(/[ \t\r\n]+/gu, " ")
    .toLocaleLowerCase("en-US");
}

function parseReferenceDestination(rest) {
  const value = rest.trim();
  if (!value) return undefined;
  let destination;
  let remainder;
  if (value.startsWith("<")) {
    const closing = value.indexOf(">");
    if (closing === -1) return undefined;
    destination = value.slice(1, closing);
    remainder = value.slice(closing + 1).trim();
  } else {
    const match = value.match(/^(\S+)(.*)$/u);
    if (!match) return undefined;
    destination = match[1];
    remainder = match[2].trim();
  }
  if (
    remainder &&
    !/^(?:"[^"\r\n]*"|'[^'\r\n]*'|\([^()\r\n]*\))$/u.test(remainder)
  ) {
    return undefined;
  }
  return destination;
}

function collectReferenceDefinitions(text) {
  const characters = text.split("");
  const definitions = new Map();
  let offset = 0;
  for (const line of text.split("\n")) {
    const content = parseContainerLine(line).content;
    const match = content.match(/^\[([^\]\r\n]+)\]:[ \t]*(.*)$/u);
    if (!match) {
      offset += line.length + 1;
      continue;
    }
    const destination = parseReferenceDestination(match[2]);
    if (destination !== undefined) {
      const label = normalizeReferenceLabel(match[1]);
      if (label && !definitions.has(label)) definitions.set(label, destination);
      for (let index = offset; index < offset + line.length; index += 1) {
        characters[index] = " ";
      }
    }
    offset += line.length + 1;
  }
  return { definitions, text: characters.join("") };
}

function blankRange(characters, start, end) {
  for (let index = start; index < end; index += 1) {
    if (characters[index] !== "\n") characters[index] = " ";
  }
}

export function collectMarkdownReferences(markdown, sourcePath) {
  const source = path.normalize(sourcePath.replaceAll("\\", "/"));
  const text = stripFencedCode(markdown);
  const references = [];
  const { definitions, text: definitionsMasked } = collectReferenceDefinitions(text);
  const linkCharacters = definitionsMasked.split("");

  const codeSpan = /(`+)([^`\n]*?)\1/g;
  for (const match of definitionsMasked.matchAll(codeSpan)) {
    const target = repositoryCodeTarget(match[2], source);
    if (target) {
      references.push({
        source,
        target,
        line: lineAt(text, match.index),
        kind: "backtick",
        offset: match.index,
      });
    }
    blankRange(linkCharacters, match.index, match.index + match[0].length);
  }

  const markdownLink = /!?\[[^\]\n]*\]\(\s*(<[^>\n]+>|[^\s)]+)(?:\s+(?:"[^"]*"|'[^']*'|\([^)]*\)))?\s*\)/g;
  for (const match of linkCharacters.join("").matchAll(markdownLink)) {
    const target = normalizeTarget(match[1], source, false);
    if (target) {
      references.push({
        source,
        target,
        line: lineAt(text, match.index),
        kind: "markdown",
        offset: match.index,
      });
    }
    blankRange(linkCharacters, match.index, match.index + match[0].length);
  }

  const referenceLink = /!?\[([^\]\n]+)\](?:\[([^\]\n]*)\])?/g;
  for (const match of linkCharacters.join("").matchAll(referenceLink)) {
    let escapes = 0;
    for (let index = match.index - 1; index >= 0 && text[index] === "\\"; index -= 1) {
      escapes += 1;
    }
    if (escapes % 2 === 1) continue;
    const label = normalizeReferenceLabel(match[2] || match[1]);
    const destination = definitions.get(label);
    if (destination === undefined) continue;
    const target = normalizeTarget(destination, source, false);
    if (!target) continue;
    references.push({
      source,
      target,
      line: lineAt(text, match.index),
      kind: "markdown",
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
  const text = stripFencedCode(markdown);
  const slugs = new Set();
  const occurrences = new Map();
  const lines = text.split("\n");

  function addHeading(rawLabel) {
    const label = decodeHtmlEntities(
      rawLabel
      .replace(/<[^>]*>/g, "")
      .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1")
      .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
      .replace(/!\[([^\]]*)\]\[[^\]]*\]/g, "$1")
      .replace(/\[([^\]]+)\]\[[^\]]*\]/g, "$1")
      .replace(/!\[([^\]]*)\]/g, "$1")
      .replace(/\[([^\]]+)\]/g, "$1")
      .replace(/[`*_~]/g, "")
      .replace(/\\([!"#$%&'()*+,./:;<=>?@[\]^_`{|}~-])/gu, "$1"),
    )
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

  for (let index = 0; index < lines.length; index += 1) {
    const current = parseContainerLine(lines[index]);
    const atx = current.content.match(/^#{1,6}(?:[ \t]+|$)(.*)$/u);
    if (atx) {
      addHeading(atx[1].replace(/[ \t]+#+[ \t]*$/u, ""));
      continue;
    }
    const next = index + 1 < lines.length
      ? parseContainerLine(lines[index + 1])
      : undefined;
    if (
      next &&
      current.content.trim() &&
      !current.hasListMarker &&
      !next.hasListMarker &&
      current.quoteDepth === next.quoteDepth &&
      /^(?:=+|-+)[ \t]*$/u.test(next.content)
    ) {
      addHeading(current.content.trim());
      index += 1;
    }
  }
  return slugs;
}
