import { existsSync, mkdirSync, readFileSync, statSync, unlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const baseUrl = "http://127.0.0.1:1420";
const outputDir = join(root, ".local", "visual-smoke");
const targets = [
  {
    name: "control-room",
    path: "/?section=dashboard",
  },
  {
    name: "designer",
    path: "/?section=designer",
    minBytes: 50_000,
  },
  {
    name: "broadcast-setup",
    path: "/?section=controls",
  },
  {
    name: "suite",
    path: "/?section=apps",
  },
  {
    name: "settings",
    path: "/?window=settings",
  },
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
  for (const target of targets) {
    const url = `${baseUrl}${target.path}`;
    const screenshot = join(outputDir, `studio-${target.name}.png`);
    await capture(url, screenshot);
    assertScreenshot(screenshot, target);
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

function assertScreenshot(path, target) {
  const size = statSync(path).size;
  const minBytes = target.minBytes ?? 20_000;
  if (size < minBytes) {
    throw new Error(`Screenshot ${path} is too small (${size} bytes).`);
  }
  const { width, height } = readPngSize(path);
  if (width < 1000 || height < 700) {
    throw new Error(`Screenshot ${path} has unexpected dimensions ${width}x${height}.`);
  }
}

function readPngSize(path) {
  const buffer = readFileSync(path);
  if (buffer.subarray(0, 8).toString("hex") !== "89504e470d0a1a0a") {
    throw new Error(`Screenshot ${path} is not a PNG file.`);
  }
  return {
    width: buffer.readUInt32BE(16),
    height: buffer.readUInt32BE(20),
  };
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
