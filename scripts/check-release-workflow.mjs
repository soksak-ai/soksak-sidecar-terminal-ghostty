#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const read = (name) => fs.readFileSync(path.join(root, name), "utf8");
const workflow = read(".github/workflows/release.yml");
const cargo = read("Cargo.toml");
const manifest = JSON.parse(read("sidecar.json"));
if (manifest.processRole !== "sidecar-terminal-ghostty") throw new Error("Sidecar manifest must declare its project-independent processRole");
const dependency = JSON.parse(read("build-dependencies.json")).dependencies[0];
const targets = JSON.parse(read("release/targets.json"));
const makefile = read("Makefile");
const gate = read("scripts/gate.sh");
const stage = read("scripts/stage-built.sh");
const ownerPath = `soksak-sidecars/${manifest.id}`;
const requireText = (value, label) => {
  if (!workflow.includes(value)) throw new Error(`release workflow is missing ${label}: ${value}`);
};

if (!/^edition = "2024"$/m.test(cargo)) throw new Error("Rust packages must use edition 2024");
if (/\bpath\s*=\s*"\.\.\//.test(cargo)) throw new Error("Cargo dependencies must not require sibling checkouts");
if (!/^lock: preflight$/m.test(makefile) || !makefile.includes("cargo metadata --format-version 1")) throw new Error("Makefile must own Cargo lock regeneration");
if (!read("README.md").includes("make lock TARGET=")) throw new Error("README must document the owner lock target");
for (const target of ["require-tooling", "require-out", "release", "attest"]) if (!new RegExp(`^${target}:`, "m").test(makefile)) throw new Error(`Makefile target is missing: ${target}`);
if (!/^STAGE \?= dist$/m.test(makefile) || /^OUT \?= dist$/m.test(makefile)) throw new Error("Makefile must separate STAGE from release OUT");
for (const value of ["command -v soksak-sdk", "SDK_VERSION", "soksak-sdk pack-target", "soksak-sdk package", "soksak-sdk attest"]) if (!makefile.includes(value)) throw new Error(`Makefile release boundary is missing: ${value}`);
if (!read("README.md").includes("make attest TARGET=") || !read("README.md").includes("OUT=/absolute/")) throw new Error("README must document owner attestation");
if (workflow.includes(dependency.repository) || workflow.includes(dependency.commit) || workflow.includes(dependency.tools.zig)) {
  throw new Error("release workflow duplicates build-dependencies.json metadata");
}
for (const value of ["sdk_archive_url:", "sdk_archive_sha256:", "sdk_release_url:", "sdk_release_sha256:", "${{ inputs.sdk_archive_url }}", "${{ inputs.sdk_release_url }}", "$RUNNER_TEMP/soksak-sdk", "soksak-sdk prepare", ".dependencies/soksak-spec/release-template"]) requireText(value, "release-train input");
for (const obsolete of ["spec_url:", "spec_sha256:", ".dependency/spec-package"]) if (workflow.includes(obsolete)) throw new Error(`release workflow retains obsolete tooling: ${obsolete}`);
if (workflow.includes("repository: soksak-ai/soksak-spec")) throw new Error("workflow must not checkout spec source");
requireText("mlugg/setup-zig@d1434d08867e3ee9daa34448df10607b98908d29", "pinned verified Zig installer");
requireText("version: ${{ steps.build-dependency.outputs.zig }}", "manifest-owned Zig version");
requireText("use-cache: false", "clean transitive Zig dependency validation");
requireText('make verify TARGET="${{ matrix.target }}"', "owner Make verification");
requireText('make stage TARGET="${{ matrix.target }}" STAGE=dist', "owner Make staging");
requireText("build-dependency-receipt.json", "SDK provenance in the release archive");
requireText("release-template/sidecar/pack-target.mjs", "canonical target packer");
requireText("release-template/sidecar/build-release.mjs", "canonical release builder");
requireText("release-template/sidecar/validate-with-spec.mjs", "canonical release validator");
requireText("release-template/publish-canonical-release.mjs", "canonical immutable publisher");
requireText("choco install make --version=4.4.1", "addressed Windows Make environment");
requireText(`path: ${ownerPath}`, "owner checkout path");
requireText(`working-directory: ${ownerPath}`, "owner working directory");
requireText(`${ownerPath}/\${{ steps.archive.outputs.asset }}`, "artifact upload path");
requireText("GH_TOKEN: ${{ steps.release-token.outputs.token }}", "GitHub CLI release token");
if (!stage.includes("absolute candidate output")) throw new Error("stage-built does not permit isolated absolute output");
if (!/^benchmark:/m.test(makefile) || /--test bench|performance budget/.test(gate)) throw new Error("benchmark ownership is not separated from verification");
for (const value of ["staged binary conflicts", "staged manifest conflicts"]) {
  if (!stage.includes(value)) throw new Error(`stage-built is not idempotent: ${value}`);
}

for (const { target, runner } of targets) {
  requireText(`target: ${target}`, "release target");
  requireText(`runner: ${runner}`, "release runner");
}
for (const action of workflow.matchAll(/^\s*-?\s*uses:\s*([^\s#]+)/gm)) {
  if (!/^[^@\s]+@[a-f0-9]{40}$/.test(action[1])) throw new Error(`workflow action is not commit-pinned: ${action[1]}`);
}
for (const obsolete of ["stage.sh", "SOKSAK_GHOSTTY_VT_LIB", "scripts/install-zig.sh", "libghostty-vt-0.1.0-dev"]) {
  if (workflow.includes(obsolete)) throw new Error(`release workflow retains an obsolete build path: ${obsolete}`);
}

console.log("release workflow contract: passed");
