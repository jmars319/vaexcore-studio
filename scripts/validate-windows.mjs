import { spawnSync } from "node:child_process";

const steps = [
  ["npm", ["run", "test:scripts"]],
  ["npm", ["run", "typecheck", "--workspaces", "--if-present"]],
  ["npm", ["run", "smoke:visual"]],
  ["cargo", ["fmt", "--all", "--", "--check"]],
  ["cargo", ["clippy", "--all-targets", "--", "-D", "warnings"]],
  ["cargo", ["test", "--workspace"], { windowsNoRunFallback: true }],
  ["npm", ["run", "app:build:windows"], { windowsOnly: true }],
];

let failed = false;

for (const [command, args, options = {}] of steps) {
  const label = [command, ...args].join(" ");
  if (options.windowsOnly && process.platform !== "win32") {
    console.log(`\n==> ${label}`);
    console.log("Skipped: run this packaging step on a Windows machine.");
    continue;
  }
  console.log(`\n==> ${label}`);
  const result = run(command, args);
  if (result.status === 0) continue;

  if (
    options.windowsNoRunFallback &&
    process.platform === "win32" &&
    looksLikeWindowsExecutionPolicyBlock(result)
  ) {
    console.error(
      "\nRust test binaries appear to be blocked by Windows application control.",
    );
    console.error("Compiling tests with --no-run so build validity is still checked.");
    const fallback = run("cargo", ["test", "--workspace", "--no-run"]);
    if (fallback.status === 0) {
      console.error(
        "\nPartial validation only: Rust tests compiled, but Windows blocked execution.",
      );
    }
  }

  failed = true;
  break;
}

if (failed) {
  process.exitCode = 1;
} else {
  console.log("\nWindows validation completed successfully.");
}

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: process.cwd(),
    encoding: "utf8",
    shell: process.platform === "win32",
    stdio: "pipe",
  });
  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  if (result.error) {
    console.error(result.error.message);
  }
  return result;
}

function looksLikeWindowsExecutionPolicyBlock(result) {
  const output = `${result.stdout ?? ""}\n${result.stderr ?? ""}`.toLowerCase();
  return (
    output.includes("windows defender application control") ||
    output.includes("this app has been blocked") ||
    output.includes("blocked by group policy") ||
    output.includes("operation did not complete successfully") ||
    output.includes("0xc0000364")
  );
}
