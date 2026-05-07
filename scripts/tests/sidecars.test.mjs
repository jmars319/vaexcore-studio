import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import {
  detectTargetTriple,
  sidecarBinaryName,
  sidecarExtension,
  studioSidecarPaths,
  targetTripleFromEnvironment,
} from "../lib/sidecars.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const checkSidecarsScript = resolve(scriptDir, "../check-sidecars.mjs");

test("sidecar names include the Rust target triple", () => {
  assert.equal(sidecarBinaryName("aarch64-apple-darwin"), "media-runner-aarch64-apple-darwin");
  assert.equal(sidecarBinaryName("x86_64-pc-windows-msvc"), "media-runner-x86_64-pc-windows-msvc.exe");
});

test("sidecar extensions only use exe for Windows targets", () => {
  assert.equal(sidecarExtension("aarch64-apple-darwin"), "");
  assert.equal(sidecarExtension("x86_64-unknown-linux-gnu"), "");
  assert.equal(sidecarExtension("x86_64-pc-windows-msvc"), ".exe");
});

test("target triple detection prefers rustc host output", () => {
  assert.equal(
    detectTargetTriple({
      rustcOutput: "rustc 1.91.0\nhost: aarch64-apple-darwin\nrelease: 1.91.0\n",
      platform: "darwin",
      arch: "x64",
    }),
    "aarch64-apple-darwin"
  );
});

test("target triple detection falls back to platform and arch", () => {
  assert.equal(detectTargetTriple({ rustcOutput: "", platform: "darwin", arch: "x64" }), "x86_64-apple-darwin");
  assert.equal(detectTargetTriple({ rustcOutput: "", platform: "linux", arch: "arm64" }), "aarch64-unknown-linux-gnu");
  assert.equal(detectTargetTriple({ rustcOutput: "", platform: "win32", arch: "x64" }), "x86_64-pc-windows-msvc");
});

test("sidecar target triple prefers TAURI_TARGET_TRIPLE", () => {
  assert.equal(
    targetTripleFromEnvironment({ TAURI_TARGET_TRIPLE: "x86_64-pc-windows-msvc" }, () => "aarch64-apple-darwin"),
    "x86_64-pc-windows-msvc"
  );
});

test("studio sidecar paths point at Tauri binaries", () => {
  const rootDir = resolve("/repo");
  const paths = studioSidecarPaths(rootDir, "aarch64-apple-darwin");

  assert.equal(paths.binDir, resolve(rootDir, "apps/desktop/src-tauri/binaries"));
  assert.equal(paths.source, resolve(rootDir, "target/release/media-runner"));
  assert.equal(paths.destination, resolve(rootDir, "apps/desktop/src-tauri/binaries/media-runner-aarch64-apple-darwin"));
});

test("sidecar preflight reports a missing prepared binary", () => {
  const rootDir = mkdtempSync(join(tmpdir(), "vaexcore-studio-sidecar-"));
  try {
    const result = spawnSync(process.execPath, [checkSidecarsScript], {
      encoding: "utf8",
      env: {
        ...process.env,
        TAURI_TARGET_TRIPLE: "aarch64-apple-darwin",
        VAEXCORE_STUDIO_ROOT_DIR: rootDir,
      },
    });

    assert.equal(result.status, 1);
    assert.match(result.stderr, /expected prepared media-runner sidecar not found/);
    assert.match(result.stderr, /Run npm run prepare:sidecars -w apps\/desktop/);
  } finally {
    rmSync(rootDir, { recursive: true, force: true });
  }
});
