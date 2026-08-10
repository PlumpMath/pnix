#!/usr/bin/env node
"use strict";

const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");
const {spawnSync} = require("node:child_process");

const root = path.resolve(process.argv[2] || path.join(__dirname, ".."));
const metaRoot = path.join(root, "cljs-meta");
const outputRoot = path.join(metaRoot, "dist", "fixed-point");
const bootstrapPath = path.join(metaRoot, "dist", "cljs-meta-module.js");
const runtimePath = path.join(metaRoot, "dist", "cljs-meta-stage-runtime.js");
const coreMacrosPath = path.join(
  metaRoot, "target", "cljs-meta-module", "cljs", "core$macros.js"
);
const coreAnalysisPath = path.join(
  metaRoot, "target", "cljs-meta-module", "cljs", "core.cljs.cache.edn"
);
const stageDriver = path.join(root, "bin", "fixed-point-stage.js");
const minimumStages = Number.parseInt(process.env.PNIX_CLJS_MIN_STAGES || "15", 10);
const maximumStages = Number.parseInt(process.env.PNIX_CLJS_MAX_STAGES || "32", 10);
if (!Number.isInteger(minimumStages) || !Number.isInteger(maximumStages) ||
    minimumStages < 2 || maximumStages < minimumStages) {
  throw new Error("invalid PNIX_CLJS_MIN_STAGES/PNIX_CLJS_MAX_STAGES");
}
const bootstrap = fs.readFileSync(bootstrapPath, "utf8");
const runtime = fs.readFileSync(runtimePath, "utf8");
const coreMacros = fs.readFileSync(coreMacrosPath, "utf8");
const coreAnalysis = fs.readFileSync(coreAnalysisPath, "utf8");
const coreAnalysisPrelude =
  `globalThis.PNIX_CLJS_CORE_ANALYSIS_CACHE_EDN = ${JSON.stringify(coreAnalysis)};\n`;
const coreMacrosPayload =
  `cljs.core$macros = cljs.core$macros || {};\n${coreMacros}`;
const shebangMatch = runtime.match(/^#![^\n]*\n/);
const shebang = shebangMatch ? shebangMatch[0] : "";
const runtimeBody = shebang ? runtime.slice(shebang.length) : runtime;
const runtimePrefix = `${shebang}${coreAnalysisPrelude}${runtimeBody}`;
const stableRuntime = `${runtimePrefix}\n${coreMacrosPayload}`;

fs.rmSync(outputRoot, {recursive: true, force: true});
fs.mkdirSync(outputRoot, {recursive: true});

function sha256(value) {
  return crypto.createHash("sha256").update(value).digest("hex");
}

function compileStage(inputArtifact, stageNumber) {
  const compilerInput = fs.readFileSync(inputArtifact, "utf8");
  const resultPath = path.join(outputRoot, `stage-${stageNumber}.json`);
  const child = spawnSync(
    process.execPath,
    [stageDriver, inputArtifact, root, resultPath],
    {encoding: "utf8", stdio: ["ignore", "pipe", "pipe"]}
  );

  if (child.status !== 0) {
    process.stderr.write(child.stdout || "");
    process.stderr.write(child.stderr || "");
    process.exit(child.status || 1);
  }

  const result = JSON.parse(fs.readFileSync(resultPath, "utf8"));
  const artifact = `${runtimePrefix}\n${result.payload}\n${coreMacrosPayload}`;
  const artifactPath = path.join(outputRoot, `cljs-meta-stage-${stageNumber}.js`);
  fs.writeFileSync(artifactPath, artifact);
  return {
    artifact,
    artifactPath,
    artifactSha256: sha256(artifact),
    compilerInputSha256: sha256(compilerInput),
    payloadSha256: result.payload_sha256,
    sourceClosure: result.source_closure
  };
}

const stages = [];
let compilerPath = bootstrapPath;
let convergedAt = null;
for (let stageNumber = 1; stageNumber <= maximumStages; stageNumber += 1) {
  const stage = compileStage(compilerPath, stageNumber);
  stages.push(stage);
  const previous = stages.at(-2);
  if (stageNumber >= minimumStages && previous &&
      previous.artifactSha256 === stage.artifactSha256 &&
      previous.artifact === stage.artifact) {
    convergedAt = stageNumber;
    break;
  }
  compilerPath = stage.artifactPath;
}

const finalStage = stages.at(-1);
const previousStage = stages.at(-2);
const stage1 = stages[0];
const stage2 = stages[1];
const stage3 = stages[2];
const fixed = convergedAt !== null;

const previousClosure = JSON.stringify(previousStage.sourceClosure);
const finalClosure = JSON.stringify(finalStage.sourceClosure);
const bootstrapOnlyMarkers = [
  "pnix.cljs_meta.core.compiler_state",
  "pnix.cljs_meta.module"
];
const stage0CompilerEmbedded =
  finalStage.artifact.includes(bootstrap) ||
  bootstrapOnlyMarkers.some((marker) => finalStage.artifact.includes(marker));
const compilerPayloadSelfHosted =
  stages.slice(1).every((stage, index) =>
    stage.compilerInputSha256 === stages[index].artifactSha256
  );

const receipt = {
  schema: "pnix.cljs-meta.fixed-point.v1",
  compiler_namespace: "cljs.js",
  bootstrap_artifact_sha256: sha256(bootstrap),
  stable_runtime_artifact_sha256: sha256(stableRuntime),
  core_analysis_cache_sha256: sha256(coreAnalysis),
  stage1_artifact_sha256: stage1.artifactSha256,
  stage2_artifact_sha256: stage2.artifactSha256,
  stage3_artifact_sha256: stage3.artifactSha256,
  stage2_compiler_input_sha256: stage2.compilerInputSha256,
  stage3_compiler_input_sha256: stage3.compilerInputSha256,
  stage2_payload_sha256: stage2.payloadSha256,
  stage3_payload_sha256: stage3.payloadSha256,
  source_closure_sha256: sha256(finalClosure),
  source_closure_equal: previousClosure === finalClosure,
  minimum_stage_count: minimumStages,
  stage_count: stages.length,
  converged_at_stage: convergedAt,
  stage_artifact_sha256: stages.map((stage) => stage.artifactSha256),
  fixed_point: fixed,
  stage0_compiler_embedded: stage0CompilerEmbedded,
  bootstrap_only_markers_absent: !stage0CompilerEmbedded,
  compiler_payload_self_hosted: compilerPayloadSelfHosted,
  criterion: `at least ${minimumStages} stages and final two artifact bytes equal`,
  bootstrap_trust_root: [
    "cljs.core runtime",
    "cljs.core macro bootstrap kernel",
    "cljs.reader and cljs.tools.reader runtime",
    "Google Closure runtime",
    "Node.js",
    "fixed-point stage harness"
  ]
};

fs.writeFileSync(
  path.join(outputRoot, "receipt.json"),
  `${JSON.stringify(receipt, null, 2)}\n`
);

if (!receipt.fixed_point ||
    !receipt.source_closure_equal ||
    receipt.stage0_compiler_embedded ||
    !receipt.compiler_payload_self_hosted) {
  console.error(JSON.stringify(receipt, null, 2));
  process.exit(1);
}

fs.copyFileSync(finalStage.artifactPath, path.join(outputRoot, "cljs-meta-fixed.js"));
fs.copyFileSync(
  path.join(metaRoot, "fixed-cli.js"),
  path.join(outputRoot, "cljs-meta-fixed-cli.js")
);
fs.chmodSync(path.join(outputRoot, "cljs-meta-fixed-cli.js"), 0o755);

assert.equal(previousStage.artifact, finalStage.artifact);
console.log(`cljs-meta fixed point: PASS stage=${convergedAt} ${finalStage.artifactSha256}`);
