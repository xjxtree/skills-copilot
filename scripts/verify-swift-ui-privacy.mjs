#!/usr/bin/env node

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const repoRoot = dirname(dirname(scriptPath));
const swiftSourceRoot = join(repoRoot, "apps/macos/Sources/SkillsCopilot/Views");

const safePathRenderers = [
  "DisplayText.privacyPath",
  "DisplayText.configPathSummary",
  "DisplayText.redactLocalPath",
  "AgentConfigDisplay.pathSummary",
];

const userVisibleSinkPattern =
  /(?:\b(?:Text|Label|SummaryChip)\s*|\.(?:help|accessibilityLabel|accessibilityValue|append)\s*)\(/g;
const directPathMemberPattern =
  /\.(?:displayPath|rootPath|projectRoot|localPath|sourcePath|targetPath|path)\b/;
const configTargetPathPattern =
  /\b(?:document|snapshot|snapshotToRollback|configDocument|configSnapshot|preview\.snapshot)\??\.target\b/;
const directPathValuePattern =
  /\b(?:displayPath|rootPath|projectRoot|localPath|sourcePath|targetPath|path)\b(?!\s*:)/;
const appendedPathAliasPattern = /\b(?:project|root|target|source)\b/;

export function findUnsafeSwiftUIPathSinks(source, relativePath = "<memory>") {
  const masked = maskSwiftNonCode(source);
  const violations = [];

  for (const match of masked.matchAll(userVisibleSinkPattern)) {
    const openParen = match.index + match[0].lastIndexOf("(");
    const closeParen = findMatchingParen(masked, openParen);
    if (closeParen === -1) {
      continue;
    }

    const sinkName = match[0].slice(0, match[0].lastIndexOf("(")).trim();
    const argument = masked.slice(openParen + 1, closeParen);
    const uncheckedArgument = blankSafeRendererCalls(argument);
    const hasDirectPath =
      directPathMemberPattern.test(uncheckedArgument)
      || configTargetPathPattern.test(uncheckedArgument)
      || directPathValuePattern.test(uncheckedArgument)
      || (sinkName.endsWith("append") && appendedPathAliasPattern.test(uncheckedArgument));

    if (hasDirectPath) {
      violations.push({
        relativePath,
        line: lineNumberAt(source, match.index),
        sink: sinkName,
        message: "user-visible path data must pass through a privacy-safe path renderer",
      });
    }
  }

  if (relativePath.includes("/Views/") || relativePath.startsWith("Views/")) {
    const collapsePattern = /\bDisplayText\.collapsePath\s*\(/g;
    for (const match of masked.matchAll(collapsePattern)) {
      violations.push({
        relativePath,
        line: lineNumberAt(source, match.index),
        sink: "DisplayText.collapsePath",
        message: "collapsePath shortens paths but does not redact private local roots",
      });
    }
  }

  return deduplicateViolations(violations);
}

export function verifySwiftUIPrivacy(root = swiftSourceRoot) {
  const violations = [];
  for (const file of swiftFilesIn(root)) {
    const source = readFileSync(file, "utf8");
    const relativePath = relative(repoRoot, file);
    violations.push(...findUnsafeSwiftUIPathSinks(source, relativePath));
  }
  return violations;
}

function blankSafeRendererCalls(argument) {
  const characters = [...argument];
  for (const renderer of safePathRenderers) {
    const pattern = new RegExp(`\\b${escapeRegExp(renderer)}\\s*\\(`, "g");
    for (const match of argument.matchAll(pattern)) {
      const openParen = match.index + match[0].lastIndexOf("(");
      const closeParen = findMatchingParen(argument, openParen);
      if (closeParen === -1) {
        continue;
      }
      for (let index = match.index; index <= closeParen; index += 1) {
        characters[index] = " ";
      }
    }
  }
  return characters.join("");
}

function maskSwiftNonCode(source) {
  const output = [...source].map((character) => (character === "\n" ? "\n" : " "));
  const modes = [{ type: "code", interpolationDepth: null }];
  let index = 0;

  while (index < source.length) {
    const mode = modes[modes.length - 1];

    if (mode.type === "string") {
      const closingDelimiter = mode.multiline ? '"""' : '"';
      if (source.startsWith(closingDelimiter, index)) {
        index += closingDelimiter.length;
        modes.pop();
        continue;
      }
      if (source[index] === "\\" && source[index + 1] === "(") {
        output[index + 1] = "(";
        modes.push({ type: "code", interpolationDepth: 1 });
        index += 2;
        continue;
      }
      if (!mode.multiline && source[index] === "\\") {
        index += Math.min(2, source.length - index);
        continue;
      }
      index += 1;
      continue;
    }

    if (source.startsWith("//", index)) {
      index = skipLineComment(source, index, output);
      continue;
    }
    if (source.startsWith("/*", index)) {
      index = skipBlockComment(source, index, output);
      continue;
    }
    if (source.startsWith('"""', index)) {
      modes.push({ type: "string", multiline: true });
      index += 3;
      continue;
    }
    if (source[index] === '"') {
      modes.push({ type: "string", multiline: false });
      index += 1;
      continue;
    }

    const character = source[index];
    output[index] = character;
    if (mode.interpolationDepth !== null) {
      if (character === "(") {
        mode.interpolationDepth += 1;
      } else if (character === ")") {
        mode.interpolationDepth -= 1;
        if (mode.interpolationDepth === 0) {
          modes.pop();
        }
      }
    }
    index += 1;
  }

  return output.join("");
}

function skipLineComment(source, start, output) {
  let index = start;
  while (index < source.length && source[index] !== "\n") {
    index += 1;
  }
  if (index < source.length) {
    output[index] = "\n";
    index += 1;
  }
  return index;
}

function skipBlockComment(source, start, output) {
  let index = start + 2;
  let depth = 1;
  while (index < source.length && depth > 0) {
    if (source.startsWith("/*", index)) {
      depth += 1;
      index += 2;
      continue;
    }
    if (source.startsWith("*/", index)) {
      depth -= 1;
      index += 2;
      continue;
    }
    if (source[index] === "\n") {
      output[index] = "\n";
    }
    index += 1;
  }
  return index;
}

function findMatchingParen(source, openParen) {
  let depth = 0;
  for (let index = openParen; index < source.length; index += 1) {
    if (source[index] === "(") {
      depth += 1;
    } else if (source[index] === ")") {
      depth -= 1;
      if (depth === 0) {
        return index;
      }
    }
  }
  return -1;
}

function lineNumberAt(source, index) {
  return source.slice(0, index).split("\n").length;
}

function swiftFilesIn(root) {
  return readdirSync(root, { withFileTypes: true }).flatMap((entry) => {
    const path = join(root, entry.name);
    if (entry.isDirectory()) {
      return swiftFilesIn(path);
    }
    return entry.isFile() && entry.name.endsWith(".swift") ? [path] : [];
  });
}

function deduplicateViolations(violations) {
  const seen = new Set();
  return violations.filter((violation) => {
    const key = `${violation.relativePath}:${violation.line}:${violation.sink}:${violation.message}`;
    if (seen.has(key)) {
      return false;
    }
    seen.add(key);
    return true;
  });
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function run() {
  const violations = verifySwiftUIPrivacy();
  if (violations.length === 0) {
    console.log("Swift UI privacy verification passed: no raw path values reach guarded UI sinks");
    return;
  }

  console.error("Swift UI privacy verification failed:");
  for (const violation of violations) {
    console.error(
      `  - ${violation.relativePath}:${violation.line} ${violation.sink}: ${violation.message}`,
    );
  }
  process.exitCode = 1;
}

if (process.argv[1] && resolve(process.argv[1]) === scriptPath) {
  run();
}
