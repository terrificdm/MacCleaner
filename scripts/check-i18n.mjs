// Checks that zh-CN.json and en.json have the same keys (plural suffixes
// normalised) and that every literal t("...") key used in src/ exists.
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const root = new URL("../src/", import.meta.url).pathname;
const load = (f) => JSON.parse(readFileSync(join(root, "i18n", f), "utf8"));
const flat = (o, p = "") =>
  Object.entries(o).flatMap(([k, v]) => (typeof v === "object" ? flat(v, `${p}${k}.`) : [`${p}${k}`]));
const norm = (k) => k.replace(/_(zero|one|two|few|many|other)$/, "");

const zh = new Set(flat(load("zh-CN.json")).map(norm));
const en = new Set(flat(load("en.json")).map(norm));
let bad = 0;
for (const k of zh) if (!en.has(k)) (console.log(`missing in en: ${k}`), bad++);
for (const k of en) if (!zh.has(k)) (console.log(`missing in zh-CN: ${k}`), bad++);

const files = [];
const walk = (d) => {
  for (const f of readdirSync(d)) {
    const p = join(d, f);
    if (statSync(p).isDirectory()) walk(p);
    else if (/\.tsx?$/.test(f)) files.push(p);
  }
};
walk(root);
const used = new Set();
for (const f of files) {
  const src = readFileSync(f, "utf8");
  for (const m of src.matchAll(/\bt\(\s*["']([a-zA-Z0-9_.]+)["']/g)) used.add(m[1]);
}
for (const k of used) {
  if (!zh.has(k)) (console.log(`used but undefined: ${k}`), bad++);
}

// Dynamic keys built from known ids.
const dyn = {
  "nav.": ["smart", "space", "system", "dev", "large", "apps", "leftovers", "trash", "history", "settings"],
  "modules.*.subtitle": ["smart", "space", "system", "dev", "large", "apps", "leftovers", "trash", "history", "settings"],
  "modules.*.heroTitle": ["smart", "space", "system", "dev", "large", "leftovers", "trash"],
  "modules.*.heroHint": ["smart", "space", "system", "dev", "large", "leftovers", "trash"],
  "errors.": ["notAbsolute", "notFound", "protected", "whitelisted", "inTrash", "outsideAllowed", "tooShallow", "appRunning", "changed", "finderFailed", "commandFailed", "brewFailed", "appRefusedQuit", "cancelled", "noScan", "unknown", "insidePackage", "self", "inUse", "installNotAllowed", "oldRunning", "oldNotMoved", "copyFailed", "locked", "restoreExists", "restoreNotInTrash", "restoreManual", "restoreFailed"],
  "install.title.": ["fresh", "upgrade", "reinstall", "downgrade", "conflict"],
  "install.desc.": ["fresh", "upgrade", "reinstall", "downgrade", "conflict"],
  "install.action.": ["fresh", "upgrade", "reinstall", "downgrade"],
  "history.method.": ["trash", "finder", "command", "dryRun", "emptyTrash", "restored"],
  "smart.cards.": ["system", "dev", "leftovers", "downloads", "large", "trash"],
};
for (const [pat, ids] of Object.entries(dyn)) {
  for (const id of ids) {
    const k = pat.includes("*") ? pat.replace("*", id) : pat + id;
    if (!zh.has(k)) (console.log(`dynamic key undefined: ${k}`), bad++);
  }
}

console.log(bad ? `\n${bad} problem(s)` : `i18n OK: ${zh.size} keys, ${used.size} literal keys used`);
process.exit(bad ? 1 : 0);
