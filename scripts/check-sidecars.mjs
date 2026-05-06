#!/usr/bin/env node
import { existsSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { platform } from "node:process";
import { fileURLToPath } from "node:url";
import { detectTargetTriple, studioSidecarPaths } from "./lib/sidecars.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const rootDir = resolve(scriptDir, "..");
const targetTriple = process.env.TAURI_TARGET_TRIPLE || detectTargetTriple();
const { destination } = studioSidecarPaths(rootDir, targetTriple);

if (!existsSync(destination)) {
  console.error(`expected prepared media-runner sidecar not found: ${destination}`);
  console.error("Run npm run prepare:sidecars -w apps/desktop before testing the Tauri desktop crate.");
  process.exit(1);
}

const stats = statSync(destination);
if (!stats.isFile() || stats.size === 0) {
  console.error(`prepared media-runner sidecar is not a non-empty file: ${destination}`);
  process.exit(1);
}

if (platform !== "win32" && (stats.mode & 0o111) === 0) {
  console.error(`prepared media-runner sidecar is not executable: ${destination}`);
  process.exit(1);
}

console.log(`sidecar preflight passed: ${destination}`);
