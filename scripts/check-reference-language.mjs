import { readFileSync } from "node:fs";

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
const documents = new Map([
  ["README.md", read("README.md")],
  ["docs/CHANGELOG.md", read("docs/CHANGELOG.md")],
  ["docs/TERMINAL-PRESENTATION.md", read("docs/TERMINAL-PRESENTATION.md")],
]);

const forbidden = [
  /not against another engine/i,
  /unlike an engine/i,
  /unlike the vt100 unit/i,
  /\bpinned fork\b/i,
  /\bfork commit\b/i,
  /\bforked libghostty-vt\b/i,
];

for (const [path, text] of documents) {
  for (const pattern of forbidden) {
    if (pattern.test(text)) throw new Error(`${path} retains comparison/provenance prose: ${pattern}`);
  }
}

const readme = documents.get("README.md");
for (const required of [
  "## Graded against the declared reference state",
  "Ghostty exposes reply callbacks for terminal queries.",
  "the SDK revision declared in `build-dependencies.json` emits canonical Darwin archive metadata and member order",
  "DEC Special Graphics designation and invocation are available at this engine boundary",
]) {
  if (!readme.includes(required)) throw new Error(`README.md is missing self-description: ${required}`);
}

if (!documents.get("docs/CHANGELOG.md").includes("Pinned the Ghostty SDK source revision")) {
  throw new Error("docs/CHANGELOG.md does not describe the pinned SDK revision directly");
}
if (!documents.get("docs/TERMINAL-PRESENTATION.md").includes("The pinned libghostty-vt C API exposes")) {
  throw new Error("docs/TERMINAL-PRESENTATION.md does not name the pinned API capability");
}

const dependencies = read("build-dependencies.json");
for (const required of [
  '"repository": "https://github.com/min-median-max/ghostty.git"',
  '"commit": "fbfb4c510321379d63ff8e83ab40410f8656cefe"',
]) {
  if (!dependencies.includes(required)) throw new Error(`build dependency identity changed: ${required}`);
}
