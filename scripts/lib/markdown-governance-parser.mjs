import { HTML_CHARACTER_ENTITIES } from "./html-character-entities.mjs";

const ESCAPABLE_PUNCTUATION = /^[!"#$%&'()*+,./:;<=>?@[\]^_`{|}~-]$/u;

function measureIndent(line, start, maximum = Number.POSITIVE_INFINITY) {
  let columns = 0;
  let offset = start;
  while (offset < line.length) {
    let width;
    if (line[offset] === " ") {
      width = 1;
    } else if (line[offset] === "\t") {
      width = 4 - (columns % 4);
    } else {
      break;
    }
    if (columns + width > maximum) break;
    columns += width;
    offset += 1;
  }
  return { columns, offset };
}

function consumeRequiredIndent(line, start, required) {
  let columns = 0;
  let offset = start;
  while (offset < line.length && columns < required) {
    let width;
    if (line[offset] === " ") {
      width = 1;
    } else if (line[offset] === "\t") {
      width = 4 - (columns % 4);
    } else {
      break;
    }
    columns += width;
    offset += 1;
  }
  return columns >= required ? offset : undefined;
}

function parseQuotePrefix(line, start) {
  const indent = measureIndent(line, start, 3);
  if (line[indent.offset] !== ">") return undefined;
  let offset = indent.offset + 1;
  if (line[offset] === " " || line[offset] === "\t") offset += 1;
  return offset;
}

function parseListMarker(line, start) {
  const indent = measureIndent(line, start, 3);
  let offset = indent.offset;
  let markerWidth;
  if (/[-+*]/u.test(line[offset] ?? "")) {
    markerWidth = 1;
  } else {
    const ordered = line.slice(offset).match(/^\d{1,9}[.)]/u);
    if (!ordered) return undefined;
    markerWidth = ordered[0].length;
  }
  const afterMarker = offset + markerWidth;
  if (afterMarker < line.length && !/[ \t]/u.test(line[afterMarker])) {
    return undefined;
  }
  if (afterMarker === line.length) {
    return {
      container: {
        type: "list",
        contentIndent: indent.columns + markerWidth + 1,
      },
      offset: afterMarker,
    };
  }

  const padding = measureIndent(line, afterMarker);
  const paddingColumns = padding.columns <= 4 ? padding.columns : 1;
  const paddingOffset = consumeRequiredIndent(
    line,
    afterMarker,
    paddingColumns,
  );
  return {
    container: {
      type: "list",
      contentIndent: indent.columns + markerWidth + paddingColumns,
    },
    offset: paddingOffset,
  };
}

function matchContainers(line, containers) {
  let offset = 0;
  let matched = 0;
  for (const container of containers) {
    if (container.type === "quote") {
      const next = parseQuotePrefix(line, offset);
      if (next === undefined) break;
      offset = next;
    } else {
      if (!line.slice(offset).trim()) {
        offset = line.length;
        matched += 1;
        continue;
      }
      const next = consumeRequiredIndent(
        line,
        offset,
        container.contentIndent,
      );
      if (next === undefined) break;
      offset = next;
    }
    matched += 1;
  }
  return { matched, offset };
}

function openAdditionalContainers(line, start, containers) {
  let offset = start;
  let openedList = false;
  while (offset < line.length) {
    const quoteOffset = parseQuotePrefix(line, offset);
    if (quoteOffset !== undefined) {
      containers.push({ type: "quote" });
      offset = quoteOffset;
      continue;
    }
    const list = parseListMarker(line, offset);
    if (list) {
      containers.push(list.container);
      offset = list.offset;
      openedList = true;
      continue;
    }
    break;
  }
  return { offset, openedList };
}

function fenceOpening(content) {
  const match = content.match(/^ {0,3}(`{3,}|~{3,})(.*)$/u);
  if (!match || (match[1][0] === "`" && match[2].includes("`"))) {
    return undefined;
  }
  return { character: match[1][0], length: match[1].length };
}

function isFenceClosing(content, fence) {
  const match = content.match(/^ {0,3}(`{3,}|~{3,})[ \t]*\r?$/u);
  return Boolean(
    match &&
      match[1][0] === fence.character &&
      match[1].length >= fence.length,
  );
}

export function containerStacksEqual(left, right) {
  return (
    left.length === right.length &&
    left.every(
      (container, index) =>
        container.type === right[index].type &&
        container.contentIndent === right[index].contentIndent,
    )
  );
}

export function parseMarkdownBlocks(markdown) {
  const rawLines = markdown.split("\n");
  const lines = [];
  const maskedLines = [];
  let openContainers = [];
  let fence;
  let lineStart = 0;

  for (const raw of rawLines) {
    if (fence) {
      const matched = matchContainers(raw, fence.containers);
      if (matched.matched === fence.containers.length) {
        const content = raw.slice(matched.offset);
        const closes = isFenceClosing(content, fence);
        lines.push({
          raw,
          lineStart,
          content: "",
          contentOffset: raw.length,
          containers: fence.containers,
          masked: true,
          openedList: false,
        });
        maskedLines.push(" ".repeat(raw.length));
        openContainers = fence.containers;
        if (closes) fence = undefined;
        lineStart += raw.length + 1;
        continue;
      }
      fence = undefined;
    }

    const matched = matchContainers(raw, openContainers);
    const containers = openContainers.slice(0, matched.matched);
    const opened = openAdditionalContainers(raw, matched.offset, containers);
    const content = raw.slice(opened.offset);
    const opening = fenceOpening(content);
    const masked = Boolean(opening);
    lines.push({
      raw,
      lineStart,
      content: masked ? "" : content,
      contentOffset: opened.offset,
      containers,
      masked,
      openedList: opened.openedList,
    });
    maskedLines.push(masked ? " ".repeat(raw.length) : raw);
    openContainers = containers;
    if (opening) fence = { ...opening, containers };
    lineStart += raw.length + 1;
  }

  return { lines, maskedText: maskedLines.join("\n") };
}

function decodeNumericReference(decimal, hexadecimal) {
  const codePoint = Number.parseInt(decimal ?? hexadecimal, decimal ? 10 : 16);
  if (
    !Number.isInteger(codePoint) ||
    codePoint === 0 ||
    codePoint > 0x10ffff ||
    (codePoint >= 0xd800 && codePoint <= 0xdfff)
  ) {
    return "\ufffd";
  }
  return String.fromCodePoint(codePoint);
}

export function decodeHtmlEntities(value) {
  return value.replace(
    /&(?:#(\d+)|#x([\da-fA-F]+)|([A-Za-z][A-Za-z\d]+));/gu,
    (entity, decimal, hexadecimal, named) => {
      if (decimal !== undefined || hexadecimal !== undefined) {
        return decodeNumericReference(decimal, hexadecimal);
      }
      return HTML_CHARACTER_ENTITIES.get(named) ?? entity;
    },
  );
}

function isEscaped(text, index) {
  let backslashes = 0;
  for (let cursor = index - 1; cursor >= 0 && text[cursor] === "\\"; cursor -= 1) {
    backslashes += 1;
  }
  return backslashes % 2 === 1;
}

function normalizeCodeSpanContent(content) {
  let normalized = content.replace(/\r\n?|\n/gu, " ");
  if (
    normalized.startsWith(" ") &&
    normalized.endsWith(" ") &&
    /[^ ]/u.test(normalized)
  ) {
    normalized = normalized.slice(1, -1);
  }
  return normalized;
}

function collectCodeSpans(text) {
  const spans = [];
  let offset = 0;
  while (offset < text.length) {
    if (text[offset] !== "`" || isEscaped(text, offset)) {
      offset += 1;
      continue;
    }
    const start = offset;
    while (text[offset] === "`") offset += 1;
    const delimiterLength = offset - start;
    let cursor = offset;
    let closed = false;
    while (cursor < text.length) {
      const next = text.indexOf("`", cursor);
      if (next === -1) break;
      let end = next;
      while (text[end] === "`") end += 1;
      if (end - next === delimiterLength) {
        spans.push({
          type: "code",
          start,
          end,
          content: normalizeCodeSpanContent(text.slice(offset, next)),
        });
        offset = end;
        closed = true;
        break;
      }
      cursor = end;
    }
    if (!closed) offset = start + delimiterLength;
  }
  return spans;
}

function blankRanges(text, ranges) {
  const characters = text.split("");
  for (const range of ranges) {
    for (let index = range.start; index < range.end; index += 1) {
      if (characters[index] !== "\n") characters[index] = " ";
    }
  }
  return characters.join("");
}

function findClosingBracket(text, opening) {
  let depth = 1;
  for (let index = opening + 1; index < text.length; index += 1) {
    if (isEscaped(text, index)) continue;
    if (text[index] === "[") depth += 1;
    if (text[index] === "]") {
      depth -= 1;
      if (depth === 0) return index;
    }
  }
  return undefined;
}

function skipInlineWhitespace(text, start) {
  let offset = start;
  while (/[ \t\r\n]/u.test(text[offset] ?? "")) offset += 1;
  return offset;
}

function parseInlineTitle(text, start) {
  const opening = text[start];
  const closing = opening === "(" ? ")" : opening;
  if (!['"', "'", "("].includes(opening)) return undefined;
  for (let offset = start + 1; offset < text.length; offset += 1) {
    if (text[offset] === closing && !isEscaped(text, offset)) {
      return { end: offset + 1 };
    }
    if (text[offset] === "\n" && opening !== "(") return undefined;
  }
  return undefined;
}

function parseInlineDestination(text, opening) {
  let offset = skipInlineWhitespace(text, opening + 1);
  let destination;
  if (text[offset] === "<") {
    const start = offset + 1;
    offset = start;
    while (
      offset < text.length &&
      text[offset] !== ">" &&
      text[offset] !== "\n"
    ) {
      offset += isEscaped(text, offset) ? 2 : 1;
    }
    if (text[offset] !== ">") return undefined;
    destination = text.slice(start, offset);
    offset += 1;
  } else {
    const start = offset;
    let depth = 0;
    while (offset < text.length) {
      if (isEscaped(text, offset)) {
        offset += 2;
        continue;
      }
      const character = text[offset];
      if (character === "(") {
        depth += 1;
      } else if (character === ")") {
        if (depth === 0) break;
        depth -= 1;
      } else if (/\s/u.test(character) && depth === 0) {
        break;
      }
      offset += 1;
    }
    if (depth !== 0 || offset === start) return undefined;
    destination = text.slice(start, offset);
  }

  offset = skipInlineWhitespace(text, offset);
  if (text[offset] !== ")") {
    const title = parseInlineTitle(text, offset);
    if (!title) return undefined;
    offset = skipInlineWhitespace(text, title.end);
  }
  if (text[offset] !== ")") return undefined;
  return { destination, end: offset + 1 };
}

export function normalizeReferenceLabel(label) {
  return decodeHtmlEntities(label)
    .replace(/\\([!"#$%&'()*+,./:;<=>?@[\]^_`{|}~-])/gu, "$1")
    .trim()
    .replace(/[ \t\r\n]+/gu, " ")
    .toLocaleLowerCase("en-US");
}

export function collectInlineNodes(text, definitions = new Map()) {
  const codeSpans = collectCodeSpans(text);
  const working = blankRanges(text, codeSpans);
  const links = [];
  let offset = 0;
  while (offset < working.length) {
    const image = working[offset] === "!" && working[offset + 1] === "[";
    const opening = image ? offset + 1 : offset;
    if (working[opening] !== "[" || isEscaped(working, opening)) {
      offset += 1;
      continue;
    }
    const closing = findClosingBracket(working, opening);
    if (closing === undefined) {
      offset = opening + 1;
      continue;
    }
    const label = text.slice(opening + 1, closing);
    const after = closing + 1;
    let node;
    if (working[after] === "(") {
      const inline = parseInlineDestination(working, after);
      if (inline) {
        node = {
          type: "link",
          start: offset,
          end: inline.end,
          label,
          destination: inline.destination,
        };
      }
    } else if (working[after] === "[") {
      const labelClosing = findClosingBracket(working, after);
      if (labelClosing !== undefined) {
        const explicit = text.slice(after + 1, labelClosing);
        const destination = definitions.get(
          normalizeReferenceLabel(explicit || label),
        );
        if (destination !== undefined) {
          node = {
            type: "link",
            start: offset,
            end: labelClosing + 1,
            label,
            destination,
          };
        }
      }
    } else {
      const destination = definitions.get(normalizeReferenceLabel(label));
      if (destination !== undefined) {
        node = {
          type: "link",
          start: offset,
          end: closing + 1,
          label,
          destination,
        };
      }
    }
    if (node) {
      links.push(node);
      offset = node.end;
    } else {
      offset = closing + 1;
    }
  }
  return { codeSpans, links };
}

function parseReferenceDestination(rest) {
  const value = rest.trimStart();
  if (!value) return undefined;
  let destination;
  let offset;
  if (value.startsWith("<")) {
    const closing = value.indexOf(">");
    if (closing === -1 || value.slice(1, closing).includes("\n")) {
      return undefined;
    }
    destination = value.slice(1, closing);
    offset = closing + 1;
  } else {
    let depth = 0;
    offset = 0;
    while (offset < value.length) {
      if (isEscaped(value, offset)) {
        offset += 2;
        continue;
      }
      if (/\s/u.test(value[offset]) && depth === 0) break;
      if (value[offset] === "(") depth += 1;
      if (value[offset] === ")") {
        if (depth === 0) return undefined;
        depth -= 1;
      }
      offset += 1;
    }
    if (depth !== 0 || offset === 0) return undefined;
    destination = value.slice(0, offset);
  }
  return { destination, remainder: value.slice(offset).trim() };
}

function isReferenceTitle(value) {
  return /^(?:"[^"\r\n]*"|'[^'\r\n]*'|\([^()\r\n]*\))$/u.test(value);
}

function continuedLineContent(line, requireIndent) {
  const measured = measureIndent(line.content, 0);
  if (measured.columns > 3 || (requireIndent && measured.columns === 0)) {
    return undefined;
  }
  return line.content.slice(measured.offset).replace(/\r$/u, "");
}

function maskLine(characters, line) {
  for (
    let offset = line.lineStart;
    offset < line.lineStart + line.raw.length;
    offset += 1
  ) {
    characters[offset] = " ";
  }
}

export function collectReferenceDefinitions(document) {
  const characters = document.maskedText.split("");
  const definitions = new Map();
  for (let index = 0; index < document.lines.length; index += 1) {
    const line = document.lines[index];
    if (line.masked) continue;
    const match = line.content.match(/^ {0,3}\[([^\]\r\n]+)\]:[ \t]*(.*)\r?$/u);
    if (!match) continue;

    let destinationLine = index;
    let parsed = parseReferenceDestination(match[2]);
    if (!parsed && !match[2].trim()) {
      const continuation = document.lines[index + 1];
      if (
        continuation &&
        !continuation.masked &&
        containerStacksEqual(line.containers, continuation.containers)
      ) {
        const content = continuedLineContent(continuation, true);
        if (content !== undefined) {
          parsed = parseReferenceDestination(content);
          if (parsed) destinationLine = index + 1;
        }
      }
    }
    if (!parsed) continue;
    if (parsed.remainder && !isReferenceTitle(parsed.remainder)) continue;

    let lastLine = destinationLine;
    if (!parsed.remainder) {
      const titleLine = document.lines[destinationLine + 1];
      if (
        titleLine &&
        !titleLine.masked &&
        containerStacksEqual(line.containers, titleLine.containers)
      ) {
        const title = continuedLineContent(titleLine, false);
        if (title !== undefined && isReferenceTitle(title.trim())) {
          lastLine = destinationLine + 1;
        }
      }
    }

    const label = normalizeReferenceLabel(match[1]);
    if (label && !definitions.has(label)) {
      definitions.set(label, parsed.destination);
    }
    for (let consumed = index; consumed <= lastLine; consumed += 1) {
      maskLine(characters, document.lines[consumed]);
    }
    index = lastLine;
  }
  return { definitions, text: characters.join("") };
}

function stripHtmlTags(value) {
  let output = "";
  let offset = 0;
  while (offset < value.length) {
    if (value[offset] !== "<") {
      output += value[offset];
      offset += 1;
      continue;
    }
    const closing = value.indexOf(">", offset + 1);
    const candidate = closing === -1 ? "" : value.slice(offset, closing + 1);
    if (/^<\/?[A-Za-z][^>]*>$/u.test(candidate) || /^<!--[\s\S]*-->$/u.test(candidate)) {
      offset = closing + 1;
    } else {
      output += value[offset];
      offset += 1;
    }
  }
  return output;
}

function punctuation(character) {
  return Boolean(character && /[\p{P}\p{S}]/u.test(character));
}

function whitespace(character) {
  return !character || /\s/u.test(character);
}

function stripEmphasisDelimiters(value) {
  const characters = value.split("");
  const stacks = new Map([["*", []], ["_", []], ["~", []]]);
  for (let offset = 0; offset < value.length; ) {
    const marker = value[offset];
    if (!stacks.has(marker) || isEscaped(value, offset)) {
      offset += 1;
      continue;
    }
    let end = offset;
    while (value[end] === marker) end += 1;
    const length = end - offset;
    if (marker === "~" && length < 2) {
      offset = end;
      continue;
    }
    const previous = value[offset - 1];
    const next = value[end];
    const leftFlanking =
      !whitespace(next) && (!punctuation(next) || whitespace(previous) || punctuation(previous));
    const rightFlanking =
      !whitespace(previous) && (!punctuation(previous) || whitespace(next) || punctuation(next));
    const canOpen = marker === "_"
      ? leftFlanking && (!rightFlanking || punctuation(previous))
      : leftFlanking;
    const canClose = marker === "_"
      ? rightFlanking && (!leftFlanking || punctuation(next))
      : rightFlanking;
    const stack = stacks.get(marker);
    if (canClose && stack.length > 0) {
      const opening = stack.pop();
      for (let index = opening.start; index < opening.end; index += 1) {
        characters[index] = "";
      }
      for (let index = offset; index < end; index += 1) characters[index] = "";
    } else if (canOpen) {
      stack.push({ start: offset, end });
    }
    offset = end;
  }
  return characters.join("");
}

function unescapePunctuation(value) {
  let output = "";
  for (let offset = 0; offset < value.length; offset += 1) {
    if (
      value[offset] === "\\" &&
      ESCAPABLE_PUNCTUATION.test(value[offset + 1] ?? "")
    ) {
      offset += 1;
    }
    output += value[offset];
  }
  return output;
}

export function renderInlineText(text, definitions = new Map()) {
  const literals = [];
  function renderWithPlaceholders(value) {
    const { codeSpans, links } = collectInlineNodes(value, definitions);
    const spans = [...codeSpans, ...links].sort(
      (left, right) => left.start - right.start || right.end - left.end,
    );
    let output = "";
    let offset = 0;
    for (const span of spans) {
      if (span.start < offset) continue;
      output += value.slice(offset, span.start);
      if (span.type === "code") {
        const literalIndex = literals.push(span.content) - 1;
        output += `\u0001${literalIndex}\u0002`;
      } else {
        output += renderWithPlaceholders(span.label);
      }
      offset = span.end;
    }
    return output + value.slice(offset);
  }

  let rendered = renderWithPlaceholders(text);
  rendered = stripHtmlTags(rendered);
  rendered = decodeHtmlEntities(rendered);
  rendered = stripEmphasisDelimiters(rendered);
  rendered = unescapePunctuation(rendered);
  return rendered.replace(/\u0001(\d+)\u0002/gu, (_placeholder, index) =>
    literals[Number(index)] ?? ""
  );
}
