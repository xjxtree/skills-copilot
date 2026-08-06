export const validationBlockerCodes = [
  "locked-session",
  "window-not-found",
  "activation-failed",
  "stale-bundle",
  "tool-layer-unknown",
];

const classifierRules = [
  {
    code: "locked-session",
    patterns: [/locked-session/i, /CGSSessionScreenIsLocked\s*=?\s*Yes/i, /macOS session is locked/i],
  },
  {
    code: "stale-bundle",
    patterns: [
      /stale-bundle/i,
      /older than source inputs/i,
      /stale app/i,
      /stale same-bundle/i,
      /running .* from different bundle path/i,
      /bundle path mismatch/i,
    ],
  },
  {
    code: "window-not-found",
    patterns: [
      /cgWindowNotFound/i,
      /window-not-found/i,
      /No visible .*app window found/i,
      /timed out waiting for visible .* window/i,
      /duplicate bundle id/i,
      /window ambiguity/i,
      /multiple visible .* windows/i,
    ],
  },
  {
    code: "activation-failed",
    patterns: [/activation error/i, /failed to activate/i, /activate.*failed/i, /unable to activate/i],
  },
];

export function classifyValidationBlocker(input) {
  const text = String(input ?? "").trim();
  for (const code of validationBlockerCodes) {
    if (text.startsWith(`${code}:`)) {
      return code;
    }
  }
  for (const rule of classifierRules) {
    if (rule.patterns.some((pattern) => pattern.test(text))) {
      return rule.code;
    }
  }
  return "tool-layer-unknown";
}

export function formatValidationBlocker(input, fallback = "validation blocked") {
  const text = String(input ?? "").trim();
  const code = classifyValidationBlocker(text || fallback);
  if (text.startsWith(`${code}:`)) {
    return text;
  }
  return `${code}: ${text || fallback}`;
}
