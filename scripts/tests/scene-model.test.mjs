import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import ts from "typescript";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const sharedTypesSource = resolve(scriptDir, "../../packages/shared-types/src/index.ts");

async function loadSharedTypesRuntime() {
  const source = readFileSync(sharedTypesSource, "utf8");
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ES2022,
      target: ts.ScriptTarget.ES2022,
    },
    fileName: "index.ts",
  });
  const encoded = Buffer.from(outputText, "utf8").toString("base64");
  return import(`data:text/javascript;base64,${encoded}`);
}

const sharedTypes = loadSharedTypesRuntime();

test("default scene collection is serializable and valid", async () => {
  const {
    createDefaultSceneCollection,
    createSceneCollectionBundle,
    normalizeSceneCollectionBundle,
    validateSceneCollection,
  } = await sharedTypes;
  const collection = createDefaultSceneCollection("2026-05-08T12:00:00.000Z");
  const scene = collection.scenes[0];
  const bundle = createSceneCollectionBundle(collection, "2026-05-08T13:00:00.000Z");
  const normalizedBundle = normalizeSceneCollectionBundle({
    collection: {
      name: "Imported Scenes",
      scenes: collection.scenes,
    },
  });

  assert.equal(collection.version, 1);
  assert.equal(collection.active_scene_id, scene.id);
  assert.equal(collection.active_transition_id, "transition-fade");
  assert.deepEqual(
    collection.transitions.map((transition) => transition.kind),
    ["cut", "fade"],
  );
  assert.equal(scene.canvas.width, 1920);
  assert.equal(scene.canvas.height, 1080);
  assert.deepEqual(
    new Set(scene.sources.map((source) => source.kind)),
    new Set(["display", "camera", "audio_meter", "browser_overlay", "text"]),
  );
  assert.equal(validateSceneCollection(collection).ok, true);
  assert.deepEqual(JSON.parse(JSON.stringify(collection)), collection);
  assert.equal(bundle.version, 1);
  assert.equal(bundle.exported_at, "2026-05-08T13:00:00.000Z");
  assert.deepEqual(bundle.collection, collection);
  assert.equal(normalizedBundle.version, 1);
  assert.equal(normalizedBundle.collection.name, "Imported Scenes");
  assert.equal(validateSceneCollection(normalizedBundle.collection).ok, true);
});

test("scene source defaults cover supported source kinds", async () => {
  const { createDefaultSceneSource, sceneSourceKindLabels } = await sharedTypes;
  const kinds = [
    "display",
    "window",
    "camera",
    "audio_meter",
    "image_media",
    "browser_overlay",
    "text",
    "group",
  ];

  assert.deepEqual(Object.keys(sceneSourceKindLabels).sort(), kinds.toSorted());

  for (const kind of kinds) {
    const source = createDefaultSceneSource(kind, {
      id: `source-${kind}`,
      name: sceneSourceKindLabels[kind],
    });
    assert.equal(source.kind, kind);
    assert.equal(source.id, `source-${kind}`);
    assert.equal(source.visible, true);
    assert.equal(source.locked, false);
    assert.equal(source.opacity, 1);
    assert.ok(source.config);
  }
});

test("scene collection validation catches duplicate ids and invalid transforms", async () => {
  const { cloneSceneCollection, createDefaultSceneCollection, validateSceneCollection } =
    await sharedTypes;
  const collection = cloneSceneCollection(createDefaultSceneCollection());
  const scene = collection.scenes[0];
  scene.sources[1].id = scene.sources[0].id;
  scene.sources[0].size.width = 0;
  scene.sources[0].opacity = 1.5;
  collection.active_scene_id = "missing-scene";
  collection.active_transition_id = "missing-transition";
  collection.transitions[0].id = collection.transitions[1].id;
  collection.transitions[0].duration_ms = 120;

  const result = validateSceneCollection(collection);

  assert.equal(result.ok, false);
  assert.match(
    result.issues.map((issue) => issue.message).join("\n"),
    /Duplicate source id/,
  );
  assert.match(
    result.issues.map((issue) => issue.path).join("\n"),
    /active_scene_id/,
  );
  assert.match(
    result.issues.map((issue) => issue.path).join("\n"),
    /active_transition_id/,
  );
  assert.match(
    result.issues.map((issue) => issue.message).join("\n"),
    /Duplicate transition id|Cut transitions/,
  );
  assert.match(
    result.issues.map((issue) => issue.path).join("\n"),
    /size\.width/,
  );
  assert.match(
    result.issues.map((issue) => issue.path).join("\n"),
    /opacity/,
  );
});

test("compositor graph builder preserves source order and warnings", async () => {
  const {
    buildCompositorGraph,
    buildCompositorRenderPlan,
    buildDefaultCompositorRenderTargets,
    createDefaultSceneCollection,
    evaluateCompositorFrame,
    validateCompositorGraph,
    validateCompositorRenderPlan,
  } = await sharedTypes;
  const scene = createDefaultSceneCollection("2026-05-08T12:00:00.000Z").scenes[0];
  const graph = buildCompositorGraph(scene);
  const renderPlan = buildCompositorRenderPlan(
    graph,
    buildDefaultCompositorRenderTargets("recording", graph, null),
  );
  const validation = validateCompositorGraph(graph);
  const renderValidation = validateCompositorRenderPlan(renderPlan);
  const frame = evaluateCompositorFrame(renderPlan, 2);

  assert.equal(graph.version, 1);
  assert.equal(graph.scene_id, scene.id);
  assert.equal(graph.output.width, scene.canvas.width);
  assert.equal(graph.nodes.length, scene.sources.length);
  assert.deepEqual(
    graph.nodes.map((node) => node.source_id),
    [
      "source-main-display",
      "source-camera-placeholder",
      "source-mic-meter",
      "source-alert-overlay",
      "source-title-text",
    ],
  );
  assert.equal(validation.ready, true);
  assert.ok(validation.warnings.length >= 1);
  assert.equal(renderValidation.ready, true);
  assert.deepEqual(
    renderPlan.targets.map((target) => target.kind),
    ["preview", "program", "recording"],
  );
  assert.equal(frame.clock.framerate, 60);
  assert.equal(frame.targets.length, 3);
  assert.equal(frame.targets[0].nodes[0].rect.width, 1920);
});

test("capture frame plan maps scene sources to video and audio bindings", async () => {
  const {
    buildCaptureFramePlan,
    createDefaultSceneCollection,
    validateCaptureFramePlan,
  } = await sharedTypes;
  const scene = createDefaultSceneCollection("2026-05-08T12:00:00.000Z").scenes[0];
  const plan = buildCaptureFramePlan(scene);
  const validation = validateCaptureFramePlan(plan);

  assert.equal(plan.version, 1);
  assert.equal(plan.scene_id, scene.id);
  assert.equal(plan.bindings.length, 3);
  assert.equal(validation.ready, true);
  assert.ok(validation.warnings.some((warning) => warning.includes("capture permission")));

  const display = plan.bindings.find(
    (binding) => binding.scene_source_id === "source-main-display",
  );
  const audio = plan.bindings.find(
    (binding) => binding.scene_source_id === "source-mic-meter",
  );

  assert.equal(display.capture_kind, "display");
  assert.equal(display.media_kind, "video");
  assert.equal(display.width, 1920);
  assert.equal(display.height, 1080);
  assert.equal(display.format, "bgra8");
  assert.equal(display.transport, "unavailable");
  assert.equal(audio.capture_kind, "microphone");
  assert.equal(audio.media_kind, "audio");
  assert.equal(audio.sample_rate, 48000);
  assert.equal(audio.channels, 2);
});

test("audio mixer plan maps audio meter sources to buses", async () => {
  const {
    buildAudioMixerPlan,
    createDefaultSceneCollection,
    validateAudioMixerPlan,
  } = await sharedTypes;
  const scene = createDefaultSceneCollection("2026-05-08T12:00:00.000Z").scenes[0];
  const plan = buildAudioMixerPlan(scene);
  const validation = validateAudioMixerPlan(plan);

  assert.equal(plan.version, 1);
  assert.equal(plan.scene_id, scene.id);
  assert.equal(plan.sample_rate, 48000);
  assert.equal(plan.channels, 2);
  assert.equal(plan.sources.length, 1);
  assert.deepEqual(
    plan.buses.map((bus) => bus.kind),
    ["master", "monitor", "recording", "stream"],
  );
  assert.equal(validation.ready, true);
  assert.ok(validation.warnings.some((warning) => warning.includes("audio input")));

  const source = plan.sources[0];
  assert.equal(source.scene_source_id, "source-mic-meter");
  assert.equal(source.capture_kind, "microphone");
  assert.equal(source.gain_db, 0);
  assert.equal(source.meter_enabled, true);
});

test("capture inventory binding updates scene source availability", async () => {
  const {
    bindSceneCollectionCaptureInventory,
    createDefaultSceneCollection,
  } = await sharedTypes;
  const collection = createDefaultSceneCollection("2026-05-08T12:00:00.000Z");
  const bound = bindSceneCollectionCaptureInventory(collection, {
    candidates: [
      {
        id: "display:main",
        kind: "display",
        name: "Main Display",
        available: true,
        notes: null,
      },
      {
        id: "microphone:default",
        kind: "microphone",
        name: "Default Microphone",
        available: false,
        notes: "Microphone permission is required.",
      },
    ],
    selected: [],
  });

  const display = bound.scenes[0].sources.find((source) => source.kind === "display");
  const microphone = bound.scenes[0].sources.find((source) => source.kind === "audio_meter");

  assert.equal(display.config.availability.state, "available");
  assert.equal(microphone.config.availability.state, "unavailable");
  assert.equal(collection.scenes[0].sources[0].config.availability.state, "permission_required");
});
