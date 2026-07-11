import { HTML_CHARACTER_ENTITIES_A_TO_F } from "./html-character-entities-a-f.mjs";
import { HTML_CHARACTER_ENTITIES_G_TO_M } from "./html-character-entities-g-m.mjs";
import { HTML_CHARACTER_ENTITIES_N_TO_Z } from "./html-character-entities-n-z.mjs";

// CommonMark accepts only semicolon-terminated character references. The
// generated shards contain all 2,125 such names, with the semicolon removed.
export const HTML_CHARACTER_ENTITIES = new Map([
  ...HTML_CHARACTER_ENTITIES_A_TO_F,
  ...HTML_CHARACTER_ENTITIES_G_TO_M,
  ...HTML_CHARACTER_ENTITIES_N_TO_Z,
]);
