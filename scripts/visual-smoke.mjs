import {
  existsSync,
  mkdirSync,
  readFileSync,
  statSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
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
    name: "designer-foundation-controls",
    path: "/?section=designer",
    minBytes: 50_000,
    interactions: [
      {
        type: "assert",
        expression: 'Boolean(document.querySelector("[data-testid=\\"designer-source-create-panel\\"]"))',
        message: "Designer source creation panel did not render.",
      },
      {
        type: "assert",
        expression: 'Boolean(document.querySelector("[data-testid=\\"designer-command-bar\\"]"))',
        message: "Designer command bar did not render.",
      },
      {
        type: "assert",
        expression: 'Boolean(document.querySelector("[data-testid=\\"designer-validation-panel\\"]"))',
        message: "Designer validation panel did not render.",
      },
      {
        type: "assert",
        expression: 'Boolean(document.querySelector("[data-testid=\\"designer-output-preflight\\"]"))',
        message: "Designer output preflight panel did not render.",
      },
      {
        type: "assert",
        expression:
          'Boolean(document.querySelector("[data-testid=\\"designer-shortcuts-panel\\"]"))',
        message: "Designer shortcuts panel did not render.",
      },
      {
        type: "assert",
        expression: 'Boolean(document.querySelector(".designer-preview-toolbar"))',
        message: "Designer preview toolbar did not render.",
      },
      {
        type: "click",
        selector: '[data-testid="designer-command-bar"] button',
      },
      {
        type: "assert",
        expression: 'document.querySelectorAll(".designer-source-box.selected").length >= 5',
        message: "Designer select-all command did not select the visible sources.",
      },
      {
        type: "click",
        selector: '[aria-label="Zoom preview in"]',
      },
      {
        type: "assert",
        expression: 'Number.parseFloat(document.querySelector(".designer-preview-canvas")?.style.width ?? "0") > 100',
        message: "Designer preview zoom control did not enlarge the canvas.",
      },
      {
        type: "click",
        selector: '[data-testid="designer-open-source-modal"]',
      },
      {
        type: "assert",
        expression: 'Boolean(document.querySelector("[data-testid=\\"designer-source-add-modal\\"]"))',
        message: "Designer source add modal did not render.",
      },
      {
        type: "click",
        selector: '[data-testid="designer-source-preset"]',
      },
      {
        type: "assert",
        expression: 'document.querySelectorAll("[data-testid=\\"designer-source-stack-item\\"]").length >= 6',
        message: "Designer source preset did not create a source.",
      },
    ],
  },
  {
    name: "designer-multi-select",
    path: "/?section=designer",
    minBytes: 50_000,
    interactions: [
      {
        type: "click",
        selector: '[data-testid="designer-source-select"][data-source-id="source-main-display"]',
      },
      {
        type: "click",
        selector: '[data-testid="designer-source-select"][data-source-id="source-camera-placeholder"]',
        shiftKey: true,
      },
      {
        type: "assert",
        expression: 'Boolean(document.querySelector("[data-testid=\\"designer-selection-tools\\"]"))',
        message: "Designer multi-select tools did not render.",
      },
    ],
  },
  {
    name: "designer-image-asset-runtime",
    path: "/?section=designer",
    minBytes: 50_000,
    interactions: [
      {
        type: "click",
        selector: '[data-testid="designer-open-source-modal"]',
      },
      {
        type: "assert",
        expression: 'Boolean(document.querySelector("[data-testid=\\"designer-source-add-modal\\"]"))',
        message: "Designer source add modal did not render for image source smoke.",
      },
      {
        type: "click",
        selector:
          '[data-testid="designer-source-preset"][data-source-kind="image_media"]',
      },
      {
        type: "assert",
        expression: 'Boolean(document.querySelector("[data-testid=\\"designer-asset-picker\\"]"))',
        message: "Designer image/media asset picker did not render.",
      },
      {
        type: "assert",
        expression: 'Boolean(document.querySelector("[data-testid=\\"designer-image-asset-runtime\\"]"))',
        message: "Designer image asset runtime panel did not render.",
      },
    ],
  },
  {
    name: "designer-text-runtime",
    path: "/?section=designer",
    minBytes: 50_000,
    interactions: [
      {
        type: "click",
        selector: '[data-testid="designer-source-select"][data-source-id="source-title-text"]',
      },
      {
        type: "assert",
        expression: 'Boolean(document.querySelector("[data-testid=\\"designer-text-runtime\\"]"))',
        message: "Designer text runtime panel did not render.",
      },
    ],
  },
  {
    name: "designer-filter-runtime",
    path: "/?section=designer",
    minBytes: 50_000,
    interactions: [
      {
        type: "click",
        selector: '[data-testid="designer-source-select"][data-source-id="source-title-text"]',
      },
      {
        type: "click",
        selector: '[data-testid="designer-add-source-filter"]',
      },
      {
        type: "wait",
        ms: 1200,
      },
      {
        type: "assert",
        expression: 'Boolean(document.querySelector("[data-testid=\\"designer-filter-runtime\\"]"))',
        message: "Designer filter runtime panel did not render.",
      },
      {
        type: "assert",
        expression: 'document.querySelector("[data-testid=\\"designer-filter-runtime\\"]")?.textContent?.includes("Color Correction")',
        message: "Designer filter runtime panel did not show the added visual filter.",
      },
    ],
  },
  {
    name: "designer-filter-mask-blend",
    path: "/?section=designer",
    minBytes: 50_000,
    interactions: [
      {
        type: "click",
        selector: '[data-testid="designer-source-select"][data-source-id="source-title-text"]',
      },
      {
        type: "select",
        selector: '[data-testid="designer-new-source-filter-kind"]',
        value: "mask_blend",
      },
      {
        type: "click",
        selector: '[data-testid="designer-add-source-filter"]',
      },
      {
        type: "assert",
        expression: 'Boolean(document.querySelector("[data-testid=\\"designer-filter-mask-picker\\"]"))',
        message: "Designer mask/blend picker did not render.",
      },
      {
        type: "assert",
        expression: 'document.querySelector("[data-testid=\\"designer-filter-runtime\\"]")?.textContent?.includes("Mask / Blend")',
        message: "Designer filter runtime panel did not show mask/blend filter.",
      },
    ],
  },
  {
    name: "designer-filter-lut",
    path: "/?section=designer",
    minBytes: 50_000,
    interactions: [
      {
        type: "click",
        selector: '[data-testid="designer-source-select"][data-source-id="source-title-text"]',
      },
      {
        type: "select",
        selector: '[data-testid="designer-new-source-filter-kind"]',
        value: "lut",
      },
      {
        type: "click",
        selector: '[data-testid="designer-add-source-filter"]',
      },
      {
        type: "assert",
        expression: 'Boolean(document.querySelector("[data-testid=\\"designer-filter-lut-picker\\"]"))',
        message: "Designer LUT picker did not render.",
      },
      {
        type: "assert",
        expression: 'document.querySelector("[data-testid=\\"designer-filter-runtime\\"]")?.textContent?.includes("LUT")',
        message: "Designer filter runtime panel did not show LUT filter.",
      },
    ],
  },
  {
    name: "designer-grouping",
    path: "/?section=designer",
    minBytes: 50_000,
    interactions: [
      {
        type: "click",
        selector: '[data-testid="designer-source-select"][data-source-id="source-main-display"]',
      },
      {
        type: "click",
        selector: '[data-testid="designer-source-select"][data-source-id="source-camera-placeholder"]',
        shiftKey: true,
      },
      {
        type: "click",
        selector: '[data-testid="designer-group-selection"]',
      },
      {
        type: "assert",
        expression: 'Boolean(document.querySelector("[data-testid=\\"designer-source-stack-item\\"][data-source-kind=\\"group\\"]"))',
        message: "Designer grouping did not create a group source.",
      },
      {
        type: "assert",
        expression: 'Boolean(document.querySelector("[data-testid=\\"designer-group-child-manager\\"]"))',
        message: "Designer group child manager did not render.",
      },
    ],
  },
  {
    name: "designer-selection-transform",
    path: "/?section=designer",
    minBytes: 50_000,
    interactions: [
      {
        type: "click",
        selector: '[data-testid="designer-source-select"][data-source-id="source-title-text"]',
      },
      {
        type: "click",
        selector: '[data-testid="designer-source-select"][data-source-id="source-alert-overlay"]',
        shiftKey: true,
      },
      {
        type: "assert",
        expression: 'Boolean(document.querySelector("[data-testid=\\"designer-selection-bounds\\"]"))',
        message: "Designer selection bounds did not render.",
      },
      {
        type: "drag",
        selector: '[data-testid="designer-preview-source"][data-source-id="source-title-text"]',
        deltaX: 90,
        deltaY: 45,
      },
      {
        type: "assert",
        expression: 'Number.parseFloat(document.querySelector("[data-testid=\\"designer-preview-source\\"][data-source-id=\\"source-title-text\\"]")?.style.left ?? "0") > 34',
        message: "Designer multi-source move did not update the selected source position.",
      },
      {
        type: "drag",
        selector: '[data-testid="designer-selection-resize-handle"]',
        deltaX: 80,
        deltaY: 55,
      },
      {
        type: "assert",
        expression: 'Number.parseFloat(document.querySelector("[data-testid=\\"designer-preview-source\\"][data-source-id=\\"source-title-text\\"]")?.style.width ?? "0") > 34',
        message: "Designer multi-source resize did not scale the selected source.",
      },
    ],
  },
  {
    name: "designer-transition-preview",
    path: "/?section=designer",
    minBytes: 50_000,
    interactions: [
      {
        type: "click",
        selector: '[data-testid="transition-preview-button"]',
      },
      {
        type: "assert",
        expression: 'Boolean(document.querySelector("[data-testid=\\"transition-preview-stage\\"]"))',
        message: "Transition preview stage did not render.",
      },
      {
        type: "wait",
        ms: 240,
      },
      {
        type: "assert",
        expression: 'Number(document.querySelector(".transition-preview-track")?.getAttribute("aria-valuenow") ?? 0) > 0',
        message: "Transition preview progress did not advance.",
      },
      {
        type: "assert",
        expression: 'document.querySelectorAll(".transition-preview-layer").length >= 2',
        message: "Transition preview frame layers did not render.",
      },
    ],
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
    await capture(url, screenshot, target);
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

async function capture(url, screenshot, target) {
  if (target?.interactions?.length) {
    await captureWithInteractions(url, screenshot, target);
    return;
  }

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

async function captureWithInteractions(url, screenshot, target) {
  const userDataDir = join(tmpdir(), `vaexcore-studio-smoke-${Date.now()}`);
  const port = 9300 + Math.floor(Math.random() * 400);
  if (existsSync(screenshot)) unlinkSync(screenshot);
  const child = spawn(chrome, [
    "--headless=new",
    "--disable-gpu",
    "--no-first-run",
    "--no-default-browser-check",
    "--hide-scrollbars",
    "--run-all-compositor-stages-before-draw",
    "--window-size=1440,1000",
    `--remote-debugging-port=${port}`,
    `--user-data-dir=${userDataDir}`,
    "about:blank"
  ], { cwd: root, detached: true, stdio: "ignore" });

  let cdp;
  try {
    const targetInfo = await waitForCdpTarget(port);
    cdp = await createCdpClient(targetInfo.webSocketDebuggerUrl);
    await cdp.send("Page.enable");
    await cdp.send("Runtime.enable");
    await cdp.send("Page.navigate", { url });
    await waitForCdpExpression(
      cdp,
      'Boolean(document.querySelector(".designer-grid"))',
      10_000,
    );

    for (const interaction of target.interactions) {
      await runInteraction(cdp, interaction);
    }
    await delay(300);

    const result = await cdp.send("Page.captureScreenshot", {
      captureBeyondViewport: false,
      format: "png",
    });
    writeFileSync(screenshot, Buffer.from(result.data, "base64"));
  } finally {
    cdp?.close();
    stop(child);
  }
}

async function waitForCdpTarget(port) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 15_000) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json/list`);
      if (response.ok) {
        const targets = await response.json();
        const page = targets.find((target) => target.type === "page");
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
  const listeners = new Map();
  let nextId = 1;

  await new Promise((resolveOpen, reject) => {
    socket.addEventListener("open", resolveOpen, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });

  socket.addEventListener("message", (event) => {
    const message = JSON.parse(String(event.data));
    if (message.id) {
      const request = pending.get(message.id);
      if (!request) return;
      pending.delete(message.id);
      clearTimeout(request.timeout);
      if (message.error) {
        request.reject(new Error(message.error.message));
      } else {
        request.resolve(message.result ?? {});
      }
      return;
    }

    if (message.method) {
      const queue = listeners.get(message.method) ?? [];
      listeners.set(message.method, []);
      queue.forEach((listener) => listener(message.params ?? {}));
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
    waitForEvent(method, timeoutMs) {
      return new Promise((resolveEvent, reject) => {
        const timeout = setTimeout(
          () => reject(new Error(`Timed out waiting for ${method}`)),
          timeoutMs,
        );
        const queue = listeners.get(method) ?? [];
        queue.push((params) => {
          clearTimeout(timeout);
          resolveEvent(params);
        });
        listeners.set(method, queue);
      });
    },
  };
}

async function runInteraction(cdp, interaction) {
  if (interaction.type === "wait") {
    await delay(interaction.ms);
    return;
  }

  if (interaction.type === "assert") {
    await waitForCdpExpression(cdp, interaction.expression, 5_000, interaction.message);
    return;
  }

  if (interaction.type === "click") {
    await waitForCdpExpression(
      cdp,
      `Boolean(document.querySelector(${JSON.stringify(interaction.selector)}))`,
      5_000,
      `Could not find ${interaction.selector}`,
    );
    await evaluateCdp(
      cdp,
      `(() => {
        const element = document.querySelector(${JSON.stringify(interaction.selector)});
        element.scrollIntoView({ block: "center", inline: "center" });
        element.dispatchEvent(new MouseEvent("click", {
          bubbles: true,
          cancelable: true,
          shiftKey: ${Boolean(interaction.shiftKey)},
          metaKey: ${Boolean(interaction.metaKey)},
          ctrlKey: ${Boolean(interaction.ctrlKey)}
        }));
        return true;
      })()`,
    );
    await delay(interaction.delayMs ?? 160);
    return;
  }

  if (interaction.type === "select") {
    await waitForCdpExpression(
      cdp,
      `Boolean(document.querySelector(${JSON.stringify(interaction.selector)}))`,
      5_000,
      `Could not find ${interaction.selector}`,
    );
    await evaluateCdp(
      cdp,
      `(() => {
        const element = document.querySelector(${JSON.stringify(interaction.selector)});
        element.scrollIntoView({ block: "center", inline: "center" });
        element.value = ${JSON.stringify(interaction.value)};
        element.dispatchEvent(new Event("input", { bubbles: true }));
        element.dispatchEvent(new Event("change", { bubbles: true }));
        return true;
      })()`,
    );
    await delay(interaction.delayMs ?? 160);
    return;
  }

  if (interaction.type === "drag") {
    await waitForCdpExpression(
      cdp,
      `Boolean(document.querySelector(${JSON.stringify(interaction.selector)}))`,
      5_000,
      `Could not find ${interaction.selector}`,
    );
    const start = await elementCenter(cdp, interaction.selector);
    const deltaX = interaction.deltaX ?? 0;
    const deltaY = interaction.deltaY ?? 0;
    await cdp.send("Input.dispatchMouseEvent", {
      button: "none",
      type: "mouseMoved",
      x: start.x,
      y: start.y,
    });
    await cdp.send("Input.dispatchMouseEvent", {
      button: "left",
      buttons: 1,
      clickCount: 1,
      type: "mousePressed",
      x: start.x,
      y: start.y,
    });
    for (let step = 1; step <= 6; step += 1) {
      await cdp.send("Input.dispatchMouseEvent", {
        button: "left",
        buttons: 1,
        type: "mouseMoved",
        x: start.x + (deltaX * step) / 6,
        y: start.y + (deltaY * step) / 6,
      });
      await delay(40);
    }
    await cdp.send("Input.dispatchMouseEvent", {
      button: "left",
      buttons: 0,
      type: "mouseReleased",
      x: start.x + deltaX,
      y: start.y + deltaY,
    });
    await delay(interaction.delayMs ?? 220);
    return;
  }

  throw new Error(`Unknown visual smoke interaction: ${interaction.type}`);
}

async function elementCenter(cdp, selector) {
  return evaluateCdp(
    cdp,
    `(() => {
      const element = document.querySelector(${JSON.stringify(selector)});
      if (!element) return null;
      element.scrollIntoView({ block: "center", inline: "center" });
      const rect = element.getBoundingClientRect();
      return {
        x: Math.round(rect.left + rect.width / 2),
        y: Math.round(rect.top + rect.height / 2)
      };
    })()`,
  );
}

async function waitForCdpExpression(cdp, expression, timeoutMs, message) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    const result = await evaluateCdp(cdp, expression);
    if (result) return;
    await delay(100);
  }
  throw new Error(message ?? `Timed out waiting for expression: ${expression}`);
}

async function evaluateCdp(cdp, expression) {
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
