#!/usr/bin/env node
import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const args = new Map();
for (let index = 2; index < process.argv.length; index += 2) {
  const name = process.argv[index];
  const value = process.argv[index + 1];
  if (!["--resolution", "--target", "--output-root", "--out"].includes(name) || !value || args.has(name)) {
    throw new Error("usage: write-build-receipt.mjs --resolution <json> --target <triple> --output-root <dir> --out <json>");
  }
  args.set(name, value);
}
if (args.size !== 4) throw new Error("all receipt options are required");

const resolution = JSON.parse(fs.readFileSync(args.get("--resolution"), "utf8"));
const target = args.get("--target");
if (resolution.target !== target || !Array.isArray(resolution.outputs) || resolution.outputs.length === 0) {
  throw new Error("resolved dependency does not match the receipt target");
}
const root = path.resolve(args.get("--output-root"));
const outputs = resolution.outputs.map((output) => {
  if (output.type !== "file") throw new Error(`Ghostty SDK output must be a file: ${output.path}`);
  const relative = output.path;
  const absolute = path.resolve(root, ...relative.split("/"));
  if (!absolute.startsWith(`${root}${path.sep}`)) throw new Error(`output escapes root: ${relative}`);
  const stat = fs.lstatSync(absolute);
  if (!stat.isFile() || stat.isSymbolicLink() || fs.realpathSync(absolute) !== absolute) {
    throw new Error(`output must be a regular file: ${relative}`);
  }
  const bytes = fs.readFileSync(absolute);
  return { path: relative, type: "file", size: bytes.length, sha256: createHash("sha256").update(bytes).digest("hex") };
});
const receipt = {
  schema: "soksak-build-dependency-receipt-v1",
  dependency: resolution.id,
  target,
  repository: resolution.repository,
  commit: resolution.commit,
  tools: resolution.tools,
  outputs,
};
fs.writeFileSync(args.get("--out"), `${JSON.stringify(receipt, null, 2)}\n`, { flag: "wx", mode: 0o644 });
