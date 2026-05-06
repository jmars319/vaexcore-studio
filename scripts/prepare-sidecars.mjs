import { copyFileSync, chmodSync, existsSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { arch, platform } from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const rootDir = resolve(scriptDir, "..");
const tauriDir = resolve(rootDir, "apps/desktop/src-tauri");
const binDir = resolve(tauriDir, "binaries");
const targetTriple = process.env.TAURI_TARGET_TRIPLE || detectTargetTriple();
const extension = targetTriple.includes("windows") ? ".exe" : "";

const build = spawnSync("cargo", ["build", "-p", "vaexcore-media-runner", "--release"], {
  cwd: rootDir,
  stdio: "inherit",
  shell: platform === "win32"
});

if (build.status !== 0) {
  process.exit(build.status ?? 1);
}

const source = resolve(rootDir, "target/release", `media-runner${extension}`);
const destination = resolve(binDir, `media-runner-${targetTriple}${extension}`);

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

function detectTargetTriple() {
  const rustc = spawnSync("rustc", ["-Vv"], {
    encoding: "utf8",
    shell: platform === "win32"
  });
  const hostLine = rustc.stdout
    ?.split(/\r?\n/)
    .find((line) => line.startsWith("host:"));
  const hostTriple = hostLine?.replace("host:", "").trim();
  if (hostTriple) {
    return hostTriple;
  }

  if (platform === "win32") {
    return arch === "arm64" ? "aarch64-pc-windows-msvc" : "x86_64-pc-windows-msvc";
  }
  if (platform === "darwin") {
    return arch === "arm64" ? "aarch64-apple-darwin" : "x86_64-apple-darwin";
  }
  if (platform === "linux") {
    return arch === "arm64" ? "aarch64-unknown-linux-gnu" : "x86_64-unknown-linux-gnu";
  }

  throw new Error("failed to determine Rust target triple");
}
