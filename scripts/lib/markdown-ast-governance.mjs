import { fromMarkdown } from "mdast-util-from-markdown";

function visit(node, visitor) {
  const stack = [{ node, parent: undefined }];
  while (stack.length > 0) {
    const current = stack.pop();
    visitor(current.node, current.parent);
    const children = current.node.children ?? [];
    for (let index = children.length - 1; index >= 0; index -= 1) {
      stack.push({ node: children[index], parent: current.node });
    }
  }
}

function renderInline(node) {
  let rendered = "";
  const stack = [node];
  while (stack.length > 0) {
    const current = stack.pop();
    switch (current.type) {
      case "text":
      case "inlineCode":
        rendered += current.value ?? "";
        break;
      case "break":
        rendered += "\n";
        break;
      case "image":
      case "imageReference":
        rendered += current.alt ?? "";
        break;
      case "html":
        break;
      default: {
        const children = current.children ?? [];
        for (let index = children.length - 1; index >= 0; index -= 1) {
          stack.push(children[index]);
        }
      }
    }
  }
  return rendered;
}

const INLINE_HTML_PARENTS = new Set([
  "delete",
  "emphasis",
  "heading",
  "link",
  "linkReference",
  "paragraph",
  "strong",
]);

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
  const excludedBlockRanges = [];
  let order = 0;

  visit(document, (node, parent) => {
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
    } else if (
      node.type === "code" ||
      (node.type === "html" && !INLINE_HTML_PARENTS.has(parent?.type))
    ) {
      const start = node.position?.start?.line;
      const end = node.position?.end?.line;
      if (Number.isInteger(start) && Number.isInteger(end)) {
        excludedBlockRanges.push({ start, end });
      }
    }
  });

  return { references, headings, excludedBlockRanges };
}
