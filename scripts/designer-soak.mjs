import { existsSync, mkdirSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { delimiter, join, resolve } from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const baseUrl = "http://127.0.0.1:1420/?section=designer";
const outputDir = join(root, ".local", "designer-soak");
const durationMs = Number.parseInt(process.env.VAEXCORE_DESIGNER_SOAK_MS ?? "10000", 10);
const chrome = findChrome();

mkdirSync(outputDir, { recursive: true });

const server = spawn("npm", ["run", "dev", "-w", "apps/desktop"], {
  cwd: root,
  detached: true,
  stdio: "ignore",
});

let browser;
let cdp;

try {
  await waitFor("http://127.0.0.1:1420");
  const port = 9800 + Math.floor(Math.random() * 300);
  browser = spawn(chrome, [
    "--headless=new",
    "--disable-gpu",
    "--no-first-run",
    "--no-default-browser-check",
    "--hide-scrollbars",
    "--run-all-compositor-stages-before-draw",
    "--window-size=1440,1000",
    `--remote-debugging-port=${port}`,
    `--user-data-dir=${join(tmpdir(), `vaexcore-designer-soak-${Date.now()}`)}`,
    "about:blank",
  ], { cwd: root, detached: true, stdio: "ignore" });

  const target = await waitForCdpTarget(port);
  cdp = await createCdpClient(target.webSocketDebuggerUrl);
  await cdp.send("Page.enable");
  await cdp.send("Runtime.enable");
  await cdp.send("Page.navigate", { url: baseUrl });
  await waitForExpression(
    cdp,
    'Boolean(document.querySelector("[data-testid=\\"designer-readiness-panel\\"]"))',
    15_000,
    "Designer readiness panel did not render during soak startup.",
  );

  const startedAt = Date.now();
  let cycles = 0;
  let lastStatus = null;
  while (Date.now() - startedAt < durationMs) {
    await clickIfPresent(cdp, '[aria-label="Request runtime preview frame"]');
    await clickIfPresent(cdp, '[aria-label="Request program preview frame"]');
    await clickIfPresent(cdp, '[aria-label="Refresh Scene Designer readiness report"]');
    await delay(700);
    lastStatus = await evaluate(
      cdp,
      `(() => {
        const readiness = document.querySelector("[data-testid='designer-readiness-panel']");
        const canvas = document.querySelector(".designer-preview-render-canvas");
        const sourceBoxes = document.querySelectorAll("[data-testid='designer-preview-source']");
        return {
          readinessText: readiness?.textContent ?? "",
          canvasWidth: canvas?.width ?? 0,
          canvasHeight: canvas?.height ?? 0,
          sourceCount: sourceBoxes.length,
          runtimeVisible: readiness?.textContent?.includes("Runtime Session") ?? false,
          bodyLength: document.body?.textContent?.length ?? 0
        };
      })()`,
    );
    cycles += 1;
  }

  if (!lastStatus?.runtimeVisible) {
    throw new Error("Designer soak did not observe runtime session readiness text.");
  }
  if (lastStatus.canvasWidth < 100 || lastStatus.canvasHeight < 100) {
    throw new Error(
      `Designer soak canvas dimensions were too small: ${lastStatus.canvasWidth}x${lastStatus.canvasHeight}.`,
    );
  }
  if (lastStatus.sourceCount < 1) {
    throw new Error("Designer soak did not observe any preview source boxes.");
  }
  if (lastStatus.bodyLength < 1_000) {
    throw new Error("Designer soak page body looked incomplete.");
  }

  const screenshot = join(outputDir, "studio-designer-soak.png");
  const result = await cdp.send("Page.captureScreenshot", {
    captureBeyondViewport: false,
    format: "png",
  });
  writeFileSync(screenshot, Buffer.from(result.data, "base64"));
  if (statSync(screenshot).size < 50_000) {
    throw new Error(`Designer soak screenshot is too small: ${screenshot}`);
  }

  console.log(
    `designer soak: ${cycles} cycle(s), ${lastStatus.sourceCount} source box(es), screenshot ${screenshot}`,
  );
} finally {
  cdp?.close();
  stop(browser);
  stop(server);
}

function findChrome() {
  const windows = process.platform === "win32";
  const pathCandidates = (process.env.PATH ?? "")
    .split(delimiter)
    .filter(Boolean)
    .flatMap((dir) =>
      ["google-chrome", "chrome", "chromium", "chromium-browser", "msedge"].map(
        (bin) => join(dir, windows ? `${bin}.exe` : bin),
      ),
    );
  const candidates = [
    process.env.VAEXCORE_CHROME_PATH,
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
    process.env.LOCALAPPDATA
      ? join(process.env.LOCALAPPDATA, "Google/Chrome/Application/chrome.exe")
      : null,
    process.env.PROGRAMFILES
      ? join(process.env.PROGRAMFILES, "Google/Chrome/Application/chrome.exe")
      : null,
    process.env["PROGRAMFILES(X86)"]
      ? join(process.env["PROGRAMFILES(X86)"], "Google/Chrome/Application/chrome.exe")
      : null,
    process.env.PROGRAMFILES
      ? join(process.env.PROGRAMFILES, "Microsoft/Edge/Application/msedge.exe")
      : null,
    "/usr/bin/google-chrome",
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
    "/usr/bin/microsoft-edge",
    ...pathCandidates,
  ].filter(Boolean);
  const found = candidates.find((candidate) => existsSync(candidate));
  if (!found) {
    throw new Error(
      "Designer soak requires Google Chrome, Chromium, or Microsoft Edge. Set VAEXCORE_CHROME_PATH if it is installed in a custom location.",
    );
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

async function waitForCdpTarget(port) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 15_000) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json/list`);
      if (response.ok) {
        const targets = await response.json();
        const page = targets.find((item) => item.type === "page");
        if (page?.webSocketDebuggerUrl) return page;
      }
    } catch {
      await delay(200);
    }
  }
  throw new Error(`Timed out waiting for Chrome DevTools on ${port}`);
}

async function createCdpClient(webSocketDebuggerUrl) {
  const socket = new WebSocket(webSocketDebuggerUrl);
  const pending = new Map();
  let nextId = 1;

  await new Promise((resolveOpen, reject) => {
    socket.addEventListener("open", resolveOpen, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });

  socket.addEventListener("message", (event) => {
    const message = JSON.parse(String(event.data));
    if (!message.id) return;
    const request = pending.get(message.id);
    if (!request) return;
    pending.delete(message.id);
    clearTimeout(request.timeout);
    if (message.error) {
      request.reject(new Error(message.error.message));
    } else {
      request.resolve(message.result ?? {});
    }
  });

  return {
    close() {
      for (const request of pending.values()) {
        clearTimeout(request.timeout);
        request.reject(new Error("CDP socket closed before a response arrived."));
      }
      pending.clear();
      socket.close();
    },
    send(method, params = {}) {
      const id = nextId;
      nextId += 1;
      socket.send(JSON.stringify({ id, method, params }));
      return new Promise((resolveSend, reject) => {
        const timeout = setTimeout(() => {
          pending.delete(id);
          reject(new Error(`Timed out waiting for CDP response: ${method}`));
        }, 10_000);
        pending.set(id, { resolve: resolveSend, reject, timeout });
      });
    },
  };
}

async function clickIfPresent(cdp, selector) {
  await evaluate(
    cdp,
    `(() => {
      const element = document.querySelector(${JSON.stringify(selector)});
      if (!element) return false;
      element.scrollIntoView({ block: "center", inline: "center" });
      element.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
      return true;
    })()`,
  );
}

async function waitForExpression(cdp, expression, timeoutMs, message) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    if (await evaluate(cdp, expression)) return;
    await delay(100);
  }
  throw new Error(message ?? `Timed out waiting for expression: ${expression}`);
}

async function evaluate(cdp, expression) {
  const response = await cdp.send("Runtime.evaluate", {
    awaitPromise: true,
    expression,
    returnByValue: true,
  });
  if (response.exceptionDetails) {
    throw new Error(response.exceptionDetails.text ?? "CDP evaluation failed");
  }
  return response.result?.value;
}

function stop(child) {
  if (!child?.pid) return;
  try {
    process.kill(-child.pid, "SIGTERM");
  } catch {
    child.kill("SIGTERM");
  }
}

function delay(ms) {
  return new Promise((resolveDelay) => setTimeout(resolveDelay, ms));
}
