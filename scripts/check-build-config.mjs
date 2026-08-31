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
const cargo = read("Cargo.toml");
const engine = read("src/engine.rs");
const cargoConfig = read(".cargo/config.toml");

if (!cargo.includes('soksak-kit-sidecar-terminal = { git = "https://github.com/soksak-ai/soksak-kit-sidecar-terminal", rev = "20fb2d73d13e5bcde592380d3052c5d2204a592f"')) {
  throw new Error("terminal Kit must be pinned to the final v0.0.34 release commit");
}
for (const fact of [
  "mouse_x10: self.mode(dec_mode(9))",
  "mouse_highlight: false",
  "let mouse_reporting = modes.mouse_reporting();",
  "modes.reports_pointer(input.phase, input.button)",
]) {
  if (!engine.includes(fact)) throw new Error(`Ghostty tracking-mode contract is missing: ${fact}`);
}
for (const target of ["aarch64-apple-darwin", "x86_64-apple-darwin"]) {
  const section = `[target.${target}]`;
  const start = cargoConfig.indexOf(section);
  const next = cargoConfig.indexOf("[target.", start + section.length);
  const body = cargoConfig.slice(start, next < 0 ? undefined : next);
  if (start < 0 || !body.includes("link-arg=-Wl,-no_uuid")) {
    throw new Error(`Darwin release must suppress the nondeterministic Mach-O UUID: ${target}`);
  }
}

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
if (!makefile.includes("BUILD_DEPENDENCY_COMMIT := $(shell node -p")) {
  throw new Error("Makefile does not derive the Ghostty cache identity from build-dependencies.json");
}
if (!makefile.includes("BUILD_DEPENDENCY_ROOT := target/build-dependencies/ghostty-vt-sdk/$(BUILD_DEPENDENCY_COMMIT)")) {
  throw new Error("Ghostty SDK cache is not content-addressed by its source commit");
}
for (const command of [
  "soksak-sdk validate build-dependencies",
  "scripts/prepare-ghostty-sdk.sh",
  "SOKSAK_BUILD_DEPENDENCY_ROOT=",
  "soksak-sdk validate build-receipt",
]) {
  if (!makefile.includes(command)) throw new Error(`Makefile command is missing: ${command}`);
}
if (!prepare.includes("soksak-sdk validate build-receipt-create")) throw new Error("Ghostty prepare does not use the canonical receipt creator");
if (!workflow.includes('make stage TARGET="${{ matrix.target }}"')) throw new Error("release workflow does not call the owner Make target");
if (build.includes("SOKSAK_GHOSTTY_VT_LIB")) throw new Error("build.rs still exposes the raw SDK path input");
if (!build.includes("SOKSAK_BUILD_DEPENDENCY_ROOT")) throw new Error("build.rs does not consume the Make-owned SDK root");

console.log("build configuration contract: passed");
