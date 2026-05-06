import assert from "node:assert/strict";
import test from "node:test";
import {
  detectTargetTriple,
  sidecarBinaryName,
  sidecarExtension,
  studioSidecarPaths,
} from "../lib/sidecars.mjs";

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

test("studio sidecar paths point at Tauri binaries", () => {
  const paths = studioSidecarPaths("/repo", "aarch64-apple-darwin");

  assert.equal(paths.binDir, "/repo/apps/desktop/src-tauri/binaries");
  assert.equal(paths.source, "/repo/target/release/media-runner");
  assert.equal(paths.destination, "/repo/apps/desktop/src-tauri/binaries/media-runner-aarch64-apple-darwin");
});
