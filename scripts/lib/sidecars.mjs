import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import { arch as currentArch, platform as currentPlatform } from "node:process";

export function detectTargetTriple({
  platform = currentPlatform,
  arch = currentArch,
  rustcOutput,
  spawn = spawnSync,
} = {}) {
  const output =
    rustcOutput ??
    spawn("rustc", ["-Vv"], {
      encoding: "utf8",
      shell: platform === "win32",
    }).stdout;

  const hostLine = output
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

export function sidecarExtension(targetTriple) {
  return targetTriple.includes("windows") ? ".exe" : "";
}

export function sidecarBinaryName(targetTriple) {
  return `media-runner-${targetTriple}${sidecarExtension(targetTriple)}`;
}

export function studioSidecarPaths(rootDir, targetTriple) {
  const extension = sidecarExtension(targetTriple);
  const tauriDir = resolve(rootDir, "apps/desktop/src-tauri");
  const binDir = resolve(tauriDir, "binaries");

  return {
    binDir,
    source: resolve(rootDir, "target/release", `media-runner${extension}`),
    destination: resolve(binDir, sidecarBinaryName(targetTriple)),
  };
}
