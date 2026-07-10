import { Buffer } from "node:buffer";
import { statSync } from "node:fs";
import { resolve } from "node:path";

export function filesystemIdentity(path) {
  try {
    const stats = statSync(path, { bigint: true });
    return `stat:${stats.dev}:${stats.ino}`;
  } catch {
    return `path:${Buffer.from(resolve(path)).toString("hex")}`;
  }
}

export function sameFilesystemEntry(left, right) {
  return filesystemIdentity(left) === filesystemIdentity(right);
}
