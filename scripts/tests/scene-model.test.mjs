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
    buildSceneTransitionPreviewPlan,
    createDefaultSceneCollection,
    createSceneCollectionBundle,
    normalizeSceneCollectionBundle,
    validateSceneTransitionPreviewPlan,
    validateSceneCollection,
  } = await sharedTypes;
  const collection = createDefaultSceneCollection("2026-05-08T12:00:00.000Z");
  const scene = collection.scenes[0];
  const bundle = createSceneCollectionBundle(collection, "2026-05-08T13:00:00.000Z");
  const transitionPreview = buildSceneTransitionPreviewPlan(
    collection,
    scene.id,
    scene.id,
    60,
  );
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
  assert.equal(transitionPreview.transition.id, "transition-fade");
  assert.equal(transitionPreview.frame_count, 18);
  assert.equal(transitionPreview.sample_frames.length, 3);
  assert.equal(validateSceneTransitionPreviewPlan(transitionPreview).ready, true);
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
    assert.equal(source.bounds_mode, "stretch");
    assert.deepEqual(source.filters, []);
    assert.ok(source.config);
  }
});

test("scene validation accepts current video media source configs", async () => {
  const {
    cloneSceneCollection,
    createDefaultSceneCollection,
    createDefaultSceneSource,
    validateSceneCollection,
  } = await sharedTypes;
  const collection = cloneSceneCollection(createDefaultSceneCollection("2026-05-08T12:00:00.000Z"));
  const scene = collection.scenes[0];
  scene.sources.push(
    createDefaultSceneSource("image_media", {
      id: "source-video-media",
      name: "Local Video",
      position: { x: 80, y: 60 },
      size: { width: 640, height: 360 },
      z_index: 20,
      config: {
        asset_uri: "/tmp/vaexcore-preview.mp4",
        media_type: "video",
        loop: true,
        availability: {
          state: "available",
          detail: "Video preview source selected.",
          checked_at: "2026-05-08T12:00:00.000Z",
        },
      },
    }),
  );

  assert.equal(validateSceneCollection(collection).ok, true);
});

test("scene validation accepts current browser overlay source configs", async () => {
  const {
    cloneSceneCollection,
    createDefaultSceneCollection,
    createDefaultSceneSource,
    validateSceneCollection,
  } = await sharedTypes;
  const collection = cloneSceneCollection(createDefaultSceneCollection("2026-05-08T12:00:00.000Z"));
  const scene = collection.scenes[0];
  scene.sources.push(
    createDefaultSceneSource("browser_overlay", {
      id: "source-browser-runtime",
      name: "Browser Runtime",
      position: { x: 120, y: 80 },
      size: { width: 640, height: 360 },
      z_index: 24,
      config: {
        url: "https://example.com/overlay",
        viewport: { width: 1280, height: 720 },
        custom_css: "body { background: transparent; }",
        availability: {
          state: "available",
          detail: "Browser overlay URL configured.",
          checked_at: "2026-05-08T12:00:00.000Z",
        },
      },
    }),
  );

  assert.equal(validateSceneCollection(collection).ok, true);
});

test("transition preview frames are deterministic for supported transition kinds", async () => {
  const {
    buildSceneTransitionPreviewFrame,
    buildSceneTransitionPreviewPlan,
    cloneSceneCollection,
    createDefaultSceneCollection,
  } = await sharedTypes;
  const base = createDefaultSceneCollection("2026-05-08T12:00:00.000Z");
  const secondScene = {
    ...base.scenes[0],
    id: "scene-next",
    name: "Next Scene",
    sources: base.scenes[0].sources.map((source) => ({
      ...source,
      id: `${source.id}-next`,
    })),
  };

  for (const transition of [
    {
      id: "transition-cut",
      name: "Cut",
      kind: "cut",
      duration_ms: 0,
      easing: "linear",
      config: {},
    },
    {
      id: "transition-fade-test",
      name: "Fade",
      kind: "fade",
      duration_ms: 300,
      easing: "linear",
      config: { color: "#000000" },
    },
    {
      id: "transition-swipe-test",
      name: "Swipe",
      kind: "swipe",
      duration_ms: 450,
      easing: "linear",
      config: { direction: "left", edge_softness: 0.12 },
    },
    {
      id: "transition-stinger-test",
      name: "Stinger",
      kind: "stinger",
      duration_ms: 1200,
      easing: "linear",
      config: { asset_uri: "/tmp/stinger.webm", trigger_time_ms: 500 },
    },
  ]) {
    const collection = cloneSceneCollection(base);
    collection.scenes.push(secondScene);
    collection.transitions = [transition];
    collection.active_transition_id = transition.id;
    const plan = buildSceneTransitionPreviewPlan(
      collection,
      base.scenes[0].id,
      secondScene.id,
      60,
    );
    const midpoint = buildSceneTransitionPreviewFrame(
      plan,
      Math.floor(plan.frame_count / 2),
      640,
      360,
    );
    const repeated = buildSceneTransitionPreviewFrame(
      plan,
      Math.floor(plan.frame_count / 2),
      640,
      360,
    );

    assert.equal(midpoint.transition_kind, transition.kind);
    assert.equal(midpoint.validation.ready, true);
    assert.equal(midpoint.checksum, repeated.checksum);
    assert.ok(midpoint.layers.length >= 2);

    if (transition.kind === "fade") {
      assert.ok(midpoint.layers[0].opacity < 1);
      assert.ok(midpoint.layers[1].opacity > 0);
    }
    if (transition.kind === "swipe") {
      assert.notEqual(midpoint.layers[0].offset_x, 0);
      assert.notEqual(midpoint.layers[1].offset_x, 0);
    }
    if (transition.kind === "stinger") {
      assert.equal(midpoint.layers.some((layer) => layer.role === "stinger"), true);
      assert.match(
        midpoint.layers.find((layer) => layer.role === "stinger").label,
        /stinger\.webm/,
      );
    }
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
  scene.sources[0].filters = [
    {
      id: "filter-duplicate",
      name: "Color",
      kind: "color_correction",
      enabled: true,
      order: 0,
      config: { brightness: 0.1, contrast: 1, saturation: 1, gamma: 1 },
    },
    {
      id: "filter-duplicate",
      name: "Chroma",
      kind: "chroma_key",
      enabled: false,
      order: 10,
      config: { key_color: "#00ff00", similarity: 0.25, smoothness: 0.08 },
    },
    {
      id: "filter-invalid-config",
      name: "Hot Gain",
      kind: "audio_gain",
      enabled: true,
      order: 20,
      config: { gain_db: 99 },
    },
  ];
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
  assert.match(
    result.issues.map((issue) => issue.message).join("\n"),
    /Duplicate source filter/,
  );
  assert.match(
    result.issues.map((issue) => issue.message).join("\n"),
    /gain_db/,
  );
});

test("scene filter validation accepts alpha mask blend and rejects malformed filter config", async () => {
  const { cloneSceneCollection, createDefaultSceneCollection, validateSceneCollection } =
    await sharedTypes;
  const collection = cloneSceneCollection(createDefaultSceneCollection());
  const scene = collection.scenes[0];
  scene.sources[0].filters = [
    {
      id: "filter-alpha-mask",
      name: "Alpha Mask",
      kind: "mask_blend",
      enabled: true,
      order: 0,
      config: { mask_uri: null, blend_mode: "alpha" },
    },
    {
      id: "filter-lut",
      name: "LUT",
      kind: "lut",
      enabled: true,
      order: 10,
      config: { lut_uri: null, strength: 0.5 },
    },
  ];

  assert.equal(validateSceneCollection(collection).ok, true);

  scene.sources[0].filters[0].config.blend_mode = "difference";
  scene.sources[0].filters[1].config.strength = 2;
  const invalid = validateSceneCollection(collection);

  assert.equal(invalid.ok, false);
  assert.match(
    invalid.issues.map((issue) => issue.message).join("\n"),
    /blend_mode/,
  );
  assert.match(
    invalid.issues.map((issue) => issue.message).join("\n"),
    /strength/,
  );
});

test("scene groups validate children and apply parent transforms", async () => {
  const {
    buildCompositorGraph,
    buildCompositorRenderPlan,
    createDefaultSceneCollection,
    createDefaultSceneSource,
    evaluateCompositorFrame,
    validateSceneCollection,
  } = await sharedTypes;
  const collection = createDefaultSceneCollection("2026-05-08T12:00:00.000Z");
  const scene = collection.scenes[0];
  const camera = scene.sources.find((source) => source.id === "source-camera-placeholder");
  assert.ok(camera);
  camera.position = { x: 20, y: 30 };
  camera.opacity = 0.5;
  camera.rotation_degrees = 5;
  const group = createDefaultSceneSource("group", {
    id: "source-group",
    name: "Camera Group",
    position: { x: 100, y: 50 },
    size: { width: 640, height: 360 },
    rotation_degrees: 10,
    opacity: 0.8,
    z_index: 5,
    config: { child_source_ids: [camera.id] },
  });
  scene.sources.push(group);

  const graph = buildCompositorGraph(scene);
  const cameraNode = graph.nodes.find((node) => node.source_id === camera.id);
  const plan = buildCompositorRenderPlan(graph, [
    {
      id: "program",
      name: "Program",
      kind: "program",
      width: 1920,
      height: 1080,
      framerate: 60,
      enabled: true,
      frame_format: "bgra8",
      scale_mode: "fit",
    },
  ]);
  const frame = evaluateCompositorFrame(plan, 0);
  const cameraFrameNode = frame.targets[0].nodes.find((node) => node.source_id === camera.id);

  assert.equal(validateSceneCollection(collection).ok, true);
  assert.equal(cameraNode.parent_source_id, "source-group");
  assert.equal(cameraNode.group_depth, 1);
  assert.equal(cameraFrameNode.rect.x, 120);
  assert.equal(cameraFrameNode.rect.y, 80);
  assert.equal(cameraFrameNode.rotation_degrees, 15);
  assert.equal(cameraFrameNode.opacity, 0.4);

  group.config.child_source_ids = [camera.id, camera.id, "missing-source", group.id];
  const invalid = validateSceneCollection(collection);
  assert.equal(invalid.ok, false);
  assert.match(
    invalid.issues.map((issue) => issue.message).join("\n"),
    /Duplicate group child|does not exist|Group cannot contain itself/,
  );
});

test("source bounds modes are reflected in compositor frame evaluation", async () => {
  const {
    buildCompositorGraph,
    buildCompositorRenderPlan,
    createDefaultSceneCollection,
    evaluateCompositorFrame,
  } = await sharedTypes;
  const scene = createDefaultSceneCollection("2026-05-08T12:00:00.000Z").scenes[0];
  const camera = scene.sources.find((source) => source.id === "source-camera-placeholder");
  assert.ok(camera);
  camera.position = { x: 0, y: 0 };
  camera.size = { width: 300, height: 300 };
  camera.bounds_mode = "fit";

  const graph = buildCompositorGraph(scene);
  const cameraNode = graph.nodes.find((node) => node.source_id === camera.id);
  const plan = buildCompositorRenderPlan(graph, [
    {
      id: "program",
      name: "Program",
      kind: "program",
      width: 1920,
      height: 1080,
      framerate: 60,
      enabled: true,
      frame_format: "bgra8",
      scale_mode: "fit",
    },
  ]);
  const frame = evaluateCompositorFrame(plan, 0);
  const cameraFrameNode = frame.targets[0].nodes.find((node) => node.source_id === camera.id);

  assert.equal(cameraNode.scale_mode, "fit");
  assert.equal(cameraFrameNode.rect.x, 0);
  assert.equal(cameraFrameNode.rect.y, 65.625);
  assert.equal(cameraFrameNode.rect.width, 300);
  assert.equal(cameraFrameNode.rect.height, 168.75);
});

test("compositor graph builder preserves source order and warnings", async () => {
  const {
    buildCompositorGraph,
    buildCompositorRenderPlan,
    buildDefaultCompositorRenderTargets,
    buildOutputPreflightPlan,
    buildPerformanceTelemetryPlan,
    createDefaultSceneCollection,
    evaluateCompositorFrame,
    validateCompositorGraph,
    validateCompositorRenderPlan,
    validateOutputPreflightPlan,
    validatePerformanceTelemetryPlan,
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
  const performancePlan = buildPerformanceTelemetryPlan(renderPlan);
  const performanceValidation = validatePerformanceTelemetryPlan(performancePlan);
  const recordingProfile = {
    id: "profile-test",
    name: "Local Test",
    output_folder: "~/Movies/vaexcore studio",
    filename_pattern: "{date}-{time}-{profile}",
    container: "mkv",
    resolution: { width: 1920, height: 1080 },
    framerate: 60,
    bitrate_kbps: 12000,
    encoder_preference: "auto",
    created_at: "2026-05-08T12:00:00.000Z",
    updated_at: "2026-05-08T12:00:00.000Z",
  };
  const outputPreflight = buildOutputPreflightPlan(
    "recording",
    scene,
    renderPlan,
    recordingProfile,
  );
  const outputValidation = validateOutputPreflightPlan(outputPreflight);

  assert.equal(graph.version, 1);
  assert.equal(graph.scene_id, scene.id);
  assert.equal(graph.output.width, scene.canvas.width);
  assert.equal(graph.nodes.length, scene.sources.length);
  assert.deepEqual(graph.nodes[0].filters, scene.sources[0].filters);
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
  assert.equal(performanceValidation.ready, true);
  assert.equal(outputValidation.ready, true);
  assert.equal(outputPreflight.recording_target.render_target_id, "target-recording");
  assert.ok(outputPreflight.render_targets.some((target) => target.kind === "recording"));
  assert.deepEqual(
    renderPlan.targets.map((target) => target.kind),
    ["preview", "program", "recording"],
  );
  assert.equal(performancePlan.scene_id, scene.id);
  assert.equal(performancePlan.targets.length, 3);
  assert.equal(performancePlan.targets[0].frame_budget_nanos, 16_666_666);
  assert.equal(performancePlan.targets[0].max_dropped_frames_per_minute, 18);
  assert.equal(frame.clock.framerate, 60);
  assert.equal(frame.targets.length, 3);
  assert.equal(frame.targets[0].nodes[0].rect.width, 1920);
});

test("scene runtime contracts validate preview, render, binding, and transition payloads", async () => {
  const {
    buildCompositorGraph,
    buildCompositorRenderPlan,
    buildDefaultCompositorRenderTargets,
    buildRuntimeAudioSourceBindingContract,
    buildRuntimeCaptureSourceBindingContract,
    createCompositorRenderRequest,
    createCompositorRenderResponse,
    createDefaultSceneCollection,
    createPreviewFrameRequest,
    createPreviewFrameResponse,
    createSceneActivationRequest,
    createSceneActivationResponse,
    createSceneRuntimeCommand,
    createSceneRuntimeStateUpdateRequest,
    createSceneRuntimeStateUpdateResponse,
    createTransitionExecutionRequest,
    createTransitionExecutionResponse,
    createTransitionPreviewFrameRequest,
    evaluateCompositorFrame,
    validateCompositorRenderRequest,
    validateCompositorRenderResponse,
    validatePreviewFrameRequest,
    validatePreviewFrameResponse,
    validateRuntimeAudioSourceBindingContract,
    validateRuntimeCaptureSourceBindingContract,
    validateSceneActivationRequest,
    validateSceneActivationResponse,
    validateSceneRuntimeCommand,
    validateSceneRuntimeStateUpdateRequest,
    validateSceneRuntimeStateUpdateResponse,
    validateTransitionExecutionRequest,
    validateTransitionPreviewFrameRequest,
  } = await sharedTypes;
  const collection = createDefaultSceneCollection("2026-05-08T12:00:00.000Z");
  const scene = collection.scenes[0];
  const requestedAt = "2026-05-08T12:15:00.000Z";

  const activation = createSceneActivationRequest(collection, scene.id, {
    requestId: "scene-activation-test",
    requestedAt,
    reason: "test",
  });
  const activationCommand = createSceneRuntimeCommand("activate_scene", activation, {
    commandId: "runtime-command-test",
    requestedAt,
  });

  assert.equal(validateSceneActivationRequest(activation, collection).ready, true);
  assert.equal(validateSceneRuntimeCommand(activationCommand).ready, true);
  const activationResponse = createSceneActivationResponse(activation, collection, {
    previousSceneId: scene.id,
    activatedAt: "2026-05-08T12:15:00.010Z",
  });
  assert.equal(activationResponse.status, "accepted");
  assert.equal(validateSceneActivationResponse(activationResponse, collection).ready, true);

  const stateUpdate = createSceneRuntimeStateUpdateRequest(
    collection,
    { active_scene_id: scene.id, preview_enabled: true },
    { requestId: "scene-state-test", requestedAt },
  );
  assert.equal(validateSceneRuntimeStateUpdateRequest(stateUpdate, collection).ready, true);
  const stateUpdateResponse = createSceneRuntimeStateUpdateResponse(
    stateUpdate,
    collection,
    { updatedAt: "2026-05-08T12:15:00.012Z" },
  );
  assert.equal(stateUpdateResponse.status, "active");
  assert.equal(
    validateSceneRuntimeStateUpdateResponse(stateUpdateResponse, collection).ready,
    true,
  );

  const previewRequest = createPreviewFrameRequest(scene, {
    request_id: "preview-frame-test",
    width: 1280,
    height: 720,
    framerate: 30,
    requested_at: requestedAt,
  });
  assert.equal(validatePreviewFrameRequest(previewRequest).ready, true);

  const graph = buildCompositorGraph(scene);
  const renderPlan = buildCompositorRenderPlan(
    graph,
    buildDefaultCompositorRenderTargets("recording", graph, null),
  );
  const renderRequest = createCompositorRenderRequest(renderPlan, {
    requestId: "compositor-render-test",
    requestedAt,
    frameIndex: 4,
  });
  const renderedFrame = evaluateCompositorFrame(renderPlan, 4);
  const renderResponse = createCompositorRenderResponse(renderRequest, renderedFrame, {
    renderedAt: "2026-05-08T12:15:00.020Z",
    renderTimeMs: 2.25,
  });
  const previewResponse = createPreviewFrameResponse(previewRequest, renderedFrame, {
    checksum: "sha256:test",
    generatedAt: "2026-05-08T12:15:00.030Z",
    renderTimeMs: 2.5,
  });

  assert.equal(validateCompositorRenderRequest(renderRequest).ready, true);
  assert.equal(validateCompositorRenderResponse(renderResponse).ready, true);
  assert.equal(validatePreviewFrameResponse(previewResponse).ready, true);

  const captureContract = buildRuntimeCaptureSourceBindingContract(scene);
  const captureValidation = validateRuntimeCaptureSourceBindingContract(captureContract);
  assert.equal(captureValidation.ready, true);
  assert.ok(captureContract.bindings.some((binding) => binding.media_kind === "video"));
  assert.ok(captureContract.bindings.some((binding) => binding.media_kind === "audio"));
  assert.ok(captureValidation.warnings.length >= 1);

  const audioContract = buildRuntimeAudioSourceBindingContract(scene);
  const audioValidation = validateRuntimeAudioSourceBindingContract(audioContract);
  assert.equal(audioValidation.ready, true);
  assert.ok(audioContract.buses.some((bus) => bus.kind === "master"));
  assert.ok(audioValidation.warnings.length >= 1);

  const transition = createTransitionExecutionRequest(collection, scene.id, scene.id, {
    requestId: "transition-test",
    requestedAt,
    transitionId: "transition-cut",
    framerate: 60,
  });
  const transitionResponse = createTransitionExecutionResponse(transition, collection, {
    startedAt: "2026-05-08T12:15:00.040Z",
  });

  assert.equal(validateTransitionExecutionRequest(transition, collection).ready, true);
  assert.equal(transitionResponse.preview_plan.transition.id, "transition-cut");
  assert.equal(transitionResponse.validation.ready, true);

  const toScene = {
    ...scene,
    id: "scene-to",
    name: "To Scene",
    canvas: { ...scene.canvas, background_color: "#123824" },
  };
  const stingerCollection = {
    ...collection,
    active_transition_id: "transition-stinger",
    scenes: [...collection.scenes, toScene],
    transitions: [
      ...collection.transitions,
      {
        id: "transition-stinger",
        name: "Stinger",
        kind: "stinger",
        duration_ms: 1000,
        easing: "linear",
        config: {
          asset_uri: null,
          trigger_time_ms: 500,
        },
      },
    ],
  };
  const transitionPreview = createTransitionPreviewFrameRequest(
    stingerCollection,
    scene.id,
    toScene.id,
    {
      request_id: "transition-preview-test",
      frame_index: 15,
      requested_at: requestedAt,
    },
  );
  const transitionPreviewCommand = createSceneRuntimeCommand(
    "request_transition_preview_frame",
    transitionPreview,
    { commandId: "runtime-transition-preview-command-test", requestedAt },
  );
  assert.equal(
    validateTransitionPreviewFrameRequest(transitionPreview, stingerCollection).ready,
    true,
  );
  assert.equal(validateSceneRuntimeCommand(transitionPreviewCommand).ready, true);
  assert.equal(
    validateTransitionPreviewFrameRequest(
      { ...transitionPreview, transition_id: "transition-fade" },
      stingerCollection,
    ).ready,
    false,
  );

  const invalidActivation = {
    ...activation,
    target_scene_id: "missing-scene",
  };
  assert.equal(validateSceneActivationRequest(invalidActivation, collection).ready, false);
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

test("audio graph runtime reports ordered audio filter diagnostics", async () => {
  const {
    buildAudioGraphRuntimeSnapshot,
    createDefaultSceneCollection,
    validateAudioGraphRuntimeSnapshot,
    validateSceneCollection,
  } = await sharedTypes;
  const collection = createDefaultSceneCollection("2026-05-08T12:00:00.000Z");
  const scene = collection.scenes[0];
  const source = scene.sources.find((sceneSource) => sceneSource.kind === "audio_meter");
  assert.ok(source);
  source.config.device_id = "microphone:default";
  source.config.availability = {
    state: "available",
    detail: "Default microphone is available.",
  };
  source.filters = [
    {
      id: "filter-compressor",
      name: "Compressor",
      kind: "compressor",
      enabled: true,
      order: 20,
      config: {
        threshold_db: -24,
        ratio: 3,
        attack_ms: 8,
        release_ms: 120,
        makeup_gain_db: 0,
      },
    },
    {
      id: "filter-gain",
      name: "Audio Gain",
      kind: "audio_gain",
      enabled: true,
      order: 10,
      config: { gain_db: 6 },
    },
    {
      id: "filter-gate",
      name: "Noise Gate",
      kind: "noise_gate",
      enabled: false,
      order: 30,
      config: {
        close_threshold_db: -45,
        open_threshold_db: -35,
        attack_ms: 10,
        release_ms: 120,
      },
    },
  ];

  const sceneValidation = validateSceneCollection(collection);
  assert.equal(sceneValidation.ok, true);

  const snapshot = buildAudioGraphRuntimeSnapshot(
    scene,
    3,
    "2026-05-08T12:00:00.000Z",
  );
  const validation = validateAudioGraphRuntimeSnapshot(snapshot);
  const runtimeSource = snapshot.sources[0];

  assert.equal(validation.ready, true);
  assert.equal(runtimeSource.filters.length, 3);
  assert.deepEqual(
    runtimeSource.filters.map((filter) => filter.id),
    ["filter-gain", "filter-compressor", "filter-gate"],
  );
  assert.equal(runtimeSource.filters[0].status, "applied");
  assert.equal(runtimeSource.filters[2].status, "skipped");
  assert.notEqual(runtimeSource.level_db, runtimeSource.pre_filter_level_db);
  assert.equal(runtimeSource.level_db, runtimeSource.post_filter_level_db);
  assert.equal(runtimeSource.linear_level, runtimeSource.post_filter_linear_level);
});

test("audio graph runtime reports malformed filters without mutating levels", async () => {
  const { buildAudioGraphRuntimeSnapshot, createDefaultSceneCollection } = await sharedTypes;
  const scene = createDefaultSceneCollection("2026-05-08T12:00:00.000Z").scenes[0];
  const source = scene.sources.find((sceneSource) => sceneSource.kind === "audio_meter");
  assert.ok(source);
  source.filters = [
    {
      id: "filter-hot-gain",
      name: "Hot Gain",
      kind: "audio_gain",
      enabled: true,
      order: 0,
      config: { gain_db: 99 },
    },
  ];

  const snapshot = buildAudioGraphRuntimeSnapshot(
    scene,
    2,
    "2026-05-08T12:00:00.000Z",
  );
  const runtimeSource = snapshot.sources[0];

  assert.equal(runtimeSource.filters[0].status, "error");
  assert.match(runtimeSource.filters[0].status_detail, /between/);
  assert.equal(runtimeSource.level_db, runtimeSource.pre_filter_level_db);
});

test("capture inventory binding updates scene source availability", async () => {
  const {
    bindSceneCollectionCaptureInventory,
    createDefaultSceneCollection,
  } = await sharedTypes;
  const collection = createDefaultSceneCollection("2026-05-08T12:00:00.000Z");
  const camera = collection.scenes[0].sources.find((source) => source.kind === "camera");
  const microphone = collection.scenes[0].sources.find(
    (source) => source.kind === "audio_meter",
  );
  assert.ok(camera);
  assert.ok(microphone);
  camera.config.device_id = "camera:facecam";
  microphone.config.device_id = "system:loopback";
  microphone.config.channel = "system";

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
      {
        id: "camera:facecam",
        kind: "camera",
        name: "Facecam",
        available: true,
        notes: null,
      },
      {
        id: "system:loopback",
        kind: "system_audio",
        name: "Desktop Audio",
        available: false,
        notes: "System audio capture is unavailable.",
      },
    ],
    selected: [],
  });

  const display = bound.scenes[0].sources.find((source) => source.kind === "display");
  const boundCamera = bound.scenes[0].sources.find((source) => source.kind === "camera");
  const boundMicrophone = bound.scenes[0].sources.find(
    (source) => source.kind === "audio_meter",
  );

  assert.equal(display.config.availability.state, "available");
  assert.equal(boundCamera.config.availability.state, "available");
  assert.equal(boundMicrophone.config.availability.state, "unavailable");
  assert.match(boundMicrophone.config.availability.detail, /System audio capture/);
  assert.equal(collection.scenes[0].sources[0].config.availability.state, "permission_required");
});
