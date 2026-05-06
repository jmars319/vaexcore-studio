import { copyFileSync, chmodSync, existsSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { platform } from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { detectTargetTriple, studioSidecarPaths } from "./lib/sidecars.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const rootDir = resolve(scriptDir, "..");
const targetTriple = process.env.TAURI_TARGET_TRIPLE || detectTargetTriple();
const { binDir, source, destination } = studioSidecarPaths(rootDir, targetTriple);

const build = spawnSync("cargo", ["build", "-p", "vaexcore-media-runner", "--release"], {
  cwd: rootDir,
  stdio: "inherit",
  shell: platform === "win32"
});

if (build.status !== 0) {
  process.exit(build.status ?? 1);
}

if (!existsSync(source)) {
  throw new Error(`expected sidecar binary not found: ${source}`);
}

mkdirSync(binDir, { recursive: true });
copyFileSync(source, destination);
try {
  chmodSync(destination, 0o755);
} catch {
  // Windows does not need POSIX execute bits.
}

console.log(`prepared sidecar: ${destination}`);
