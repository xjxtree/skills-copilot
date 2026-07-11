import { fromMarkdown } from "mdast-util-from-markdown";

function visit(node, visitor) {
  visitor(node);
  for (const child of node.children ?? []) visit(child, visitor);
}

function renderInline(node) {
  switch (node.type) {
    case "text":
    case "inlineCode":
      return node.value ?? "";
    case "break":
      return "\n";
    case "image":
    case "imageReference":
      return node.alt ?? "";
    case "html":
      return "";
    default:
      return (node.children ?? []).map(renderInline).join("");
  }
}

export function parseGovernanceMarkdown(markdown) {
  const document = fromMarkdown(markdown);
  const definitions = new Map();

  visit(document, (node) => {
    if (node.type === "definition" && !definitions.has(node.identifier)) {
      definitions.set(node.identifier, node.url);
    }
  });

  const references = [];
  const headings = [];
  const codeBlockRanges = [];
  let order = 0;

  visit(document, (node) => {
    const line = node.position?.start?.line;
    if (node.type === "heading") {
      headings.push(renderInline(node));
    } else if (node.type === "inlineCode" && Number.isInteger(line)) {
      references.push({
        kind: "backtick",
        value: node.value ?? "",
        line,
        order,
      });
      order += 1;
    } else if (
      (node.type === "link" || node.type === "image") &&
      Number.isInteger(line)
    ) {
      references.push({
        kind: "markdown",
        value: node.url ?? "",
        line,
        order,
      });
      order += 1;
    } else if (
      (node.type === "linkReference" || node.type === "imageReference") &&
      Number.isInteger(line)
    ) {
      const destination = definitions.get(node.identifier);
      if (destination !== undefined) {
        references.push({
          kind: "markdown",
          value: destination,
          line,
          order,
        });
        order += 1;
      }
    } else if (node.type === "code") {
      const start = node.position?.start?.line;
      const end = node.position?.end?.line;
      if (Number.isInteger(start) && Number.isInteger(end)) {
        codeBlockRanges.push({ start, end });
      }
    }
  });

  return { references, headings, codeBlockRanges };
}
