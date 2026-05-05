import { existsSync, mkdirSync, statSync, unlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const baseUrl = "http://127.0.0.1:1420";
const outputDir = join(root, ".local", "visual-smoke");
const targets = [
  ["control-room", "/?section=dashboard"],
  ["broadcast-setup", "/?section=controls"],
  ["suite", "/?section=apps"],
  ["settings", "/?window=settings"]
];

const chrome = findChrome();
mkdirSync(outputDir, { recursive: true });

const server = spawn("npm", ["run", "dev", "-w", "apps/desktop"], {
  cwd: root,
  detached: true,
  stdio: "ignore"
});

try {
  await waitFor(baseUrl);
  for (const [name, path] of targets) {
    const screenshot = join(outputDir, `studio-${name}.png`);
    await capture(`${baseUrl}${path}`, screenshot);
    assertScreenshot(screenshot);
    console.log(`visual smoke: wrote ${screenshot}`);
  }
} finally {
  stop(server);
}

function findChrome() {
  const candidates = [
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"
  ];
  const found = candidates.find((candidate) => existsSync(candidate));
  if (!found) {
    throw new Error("Visual smoke requires Google Chrome, Chromium, or Microsoft Edge in /Applications.");
  }
  return found;
}

async function waitFor(url) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 30_000) {
    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch {
      await delay(500);
    }
  }
  throw new Error(`Timed out waiting for ${url}`);
}

async function capture(url, screenshot) {
  const userDataDir = join(tmpdir(), `vaexcore-studio-smoke-${Date.now()}`);
  if (existsSync(screenshot)) unlinkSync(screenshot);
  const child = spawn(chrome, [
    "--headless=new",
    "--disable-gpu",
    "--no-first-run",
    "--no-default-browser-check",
    "--hide-scrollbars",
    "--run-all-compositor-stages-before-draw",
    "--virtual-time-budget=2000",
    "--window-size=1440,1000",
    `--user-data-dir=${userDataDir}`,
    `--screenshot=${screenshot}`,
    url
  ], { cwd: root, detached: true, stdio: "ignore" });

  try {
    await Promise.race([
      waitForScreenshot(screenshot, 20_000),
      waitForExit(child).then((code) => {
        if (code !== 0) {
          throw new Error(`${chrome} exited with ${code}`);
        }
        return waitForScreenshot(screenshot, 1_000);
      })
    ]);
  } finally {
    stop(child);
  }
}

function assertScreenshot(path) {
  const size = statSync(path).size;
  if (size < 20_000) {
    throw new Error(`Screenshot ${path} is too small (${size} bytes).`);
  }
}

async function waitForScreenshot(path, timeoutMs) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    if (existsSync(path) && statSync(path).size >= 20_000) {
      const size = statSync(path).size;
      await delay(300);
      if (existsSync(path) && statSync(path).size === size) return;
    }
    await delay(250);
  }
  throw new Error(`Timed out waiting for screenshot ${path}`);
}

function waitForExit(child) {
  return new Promise((resolveExit, reject) => {
    child.on("error", reject);
    child.on("exit", (code) => resolveExit(code ?? 0));
  });
}

function stop(child) {
  if (child.pid) {
    try {
      process.kill(-child.pid, "SIGTERM");
    } catch {
      child.kill("SIGTERM");
    }
  }
}

function delay(ms) {
  return new Promise((resolveDelay) => setTimeout(resolveDelay, ms));
}
