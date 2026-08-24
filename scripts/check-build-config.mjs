#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const read = (name) => fs.readFileSync(path.join(root, name), "utf8");
const manifest = JSON.parse(read("build-dependencies.json"));
const makefile = read("Makefile");
const workflow = read(".github/workflows/release.yml");
const build = read("build.rs");
const prepare = read("scripts/prepare-ghostty-sdk.sh");

if (manifest.schema !== "soksak-build-dependencies-v1" || !Array.isArray(manifest.dependencies) || manifest.dependencies.length !== 1) {
  throw new Error("build-dependencies.json must declare one external SDK dependency");
}
const dependency = manifest.dependencies[0];
const exactKeys = (value, keys, label) => {
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error(`${label} keys differ: ${actual.join(",")}`);
};
exactKeys(dependency, ["id", "repository", "commit", "tools", "targets"], "Ghostty dependency");
if (dependency.id !== "ghostty-vt-sdk") throw new Error("Ghostty dependency id is not stable");
if (!/^https:\/\/[^/]+\/(?:[^/]+\/)+[^/]+[.]git$/.test(dependency.repository)) throw new Error("Ghostty repository must be an HTTPS Git URL");
if (!/^[a-f0-9]{40}$/.test(dependency.commit)) throw new Error("Ghostty source must use an exact commit");
exactKeys(dependency.tools, ["zig"], "Ghostty tools");
if (!/^\d+[.]\d+[.]\d+$/.test(dependency.tools.zig)) throw new Error("Ghostty Zig version must be exact");

const targetRows = JSON.parse(read("release/targets.json"));
const releaseTargets = targetRows.map(({ target }) => target).sort();
const dependencyTargets = Object.keys(dependency.targets).sort();
if (JSON.stringify(releaseTargets) !== JSON.stringify(dependencyTargets)) {
  throw new Error("release and build dependency target sets differ");
}
for (const target of dependencyTargets) {
  const archive = target.includes("windows") ? "ghostty-vt-static.lib" : "libghostty-vt.a";
  const expected = [
    { path: `targets/${target}/lib/${archive}`, type: "file" },
    { path: `targets/${target}/source-commit.txt`, type: "file" },
    { path: `targets/${target}/zig-version.txt`, type: "file" },
  ];
  if (JSON.stringify(dependency.targets[target].outputs) !== JSON.stringify(expected)) {
    throw new Error(`Ghostty outputs differ for ${target}`);
  }
}

for (const [name, source] of [["Makefile", makefile], ["release workflow", workflow], ["build.rs", build]]) {
  for (const duplicated of [dependency.repository, dependency.commit, dependency.tools.zig]) {
    if (source.includes(duplicated)) throw new Error(`${name} duplicates build-dependencies.json metadata`);
  }
}
for (const target of ["preflight", "prepare", "build", "verify", "stage"]) {
  if (!new RegExp(`^${target}:`, "m").test(makefile)) throw new Error(`Makefile target is missing: ${target}`);
}
if (!makefile.includes("TARGET")) throw new Error("Makefile does not require an explicit TARGET");
for (const command of [
  "soksak-validate build-dependencies",
  "scripts/prepare-ghostty-sdk.sh",
  "SOKSAK_BUILD_DEPENDENCY_ROOT=",
  "soksak-validate build-receipt",
]) {
  if (!makefile.includes(command)) throw new Error(`Makefile command is missing: ${command}`);
}
if (!prepare.includes("soksak-validate build-receipt-create")) throw new Error("Ghostty prepare does not use the canonical receipt creator");
if (!workflow.includes('make stage TARGET="${{ matrix.target }}"')) throw new Error("release workflow does not call the owner Make target");
if (build.includes("SOKSAK_GHOSTTY_VT_LIB")) throw new Error("build.rs still exposes the raw SDK path input");
if (!build.includes("SOKSAK_BUILD_DEPENDENCY_ROOT")) throw new Error("build.rs does not consume the Make-owned SDK root");

console.log("build configuration contract: passed");
