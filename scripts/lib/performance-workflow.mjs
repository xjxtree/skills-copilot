export function workflowJobBody(workflow, jobName) {
  const escapedName = jobName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const jobPattern = new RegExp(`^  ${escapedName}:\\s*$`, "m");
  const match = jobPattern.exec(workflow);
  if (!match) {
    throw new Error(`workflow job not found: ${jobName}`);
  }

  const start = match.index;
  const remainder = workflow.slice(start + match[0].length);
  const nextJob = /^  [A-Za-z0-9_-]+:\s*$/m.exec(remainder);
  const end = nextJob ? start + match[0].length + nextJob.index : workflow.length;
  return workflow.slice(start, end);
}
