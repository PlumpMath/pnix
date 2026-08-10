#!/usr/bin/env node
"use strict";

const crypto = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "..");
const receiptPath = path.join(root, "cljs-meta", "dist", "meta-build-receipt.json");
const fixedReceiptPath = path.join(root, "cljs-meta", "dist", "fixed-point", "receipt.json");
const mode = process.argv[2];

const sourceRoots = [
  "bin/build-fixed-point.js",
  "bin/fixed-point-stage.js",
  "bin/meta-artifact-identity.js",
  "cljs-meta/deps.edn",
  "cljs-meta/build-cli.edn",
  "cljs-meta/build-module.edn",
  "cljs-meta/build-stage-runtime.edn",
  "cljs-meta/fixed-cli.js",
  "cljs-meta/package.json",
  "cljs-meta/src",
  // The ClojureScript compiler substrate is no longer vendored: it is the
  // pinned org.clojure/clojurescript jar, and deps-lock.json fixes its exact
  // content hash. Hashing the lock therefore pins the substrate identity.
  "deps-lock.json"
];

const artifacts = [
  "cljs-meta/dist/cljs-meta.js",
  "cljs-meta/dist/cljs-meta-module.js",
  "cljs-meta/dist/cljs-meta-stage-runtime.js",
  "cljs-meta/dist/fixed-point/cljs-meta-fixed.js",
  "cljs-meta/dist/fixed-point/cljs-meta-fixed-cli.js",
  "cljs-meta/dist/fixed-point/receipt.json"
];

function filesBelow(relative) {
  const absolute = path.join(root, relative);
  const stat = fs.lstatSync(absolute);
  if (!stat.isDirectory()) return [relative];
  return fs.readdirSync(absolute).sort().flatMap((name) =>
    filesBelow(path.posix.join(relative, name))
  );
}

function digestFiles(roots) {
  const files = roots.flatMap(filesBelow).sort();
  const hash = crypto.createHash("sha256");
  for (const relative of files) {
    hash.update(relative);
    hash.update(Buffer.from([0]));
    hash.update(fs.readFileSync(path.join(root, relative)));
    hash.update(Buffer.from([0]));
  }
  return {files, digest: hash.digest("hex")};
}

function assertFixedPoint() {
  const receipt = JSON.parse(fs.readFileSync(fixedReceiptPath, "utf8"));
  if (receipt.schema !== "pnix.cljs-meta.fixed-point.v1" ||
      receipt.fixed_point !== true ||
      receipt.source_closure_equal !== true ||
      receipt.stage0_compiler_embedded !== false ||
      receipt.compiler_payload_self_hosted !== true ||
      receipt.minimum_stage_count < 15 ||
      receipt.stage_count < receipt.minimum_stage_count ||
      receipt.converged_at_stage !== receipt.stage_count) {
    throw new Error("invalid cljs-meta fixed-point receipt");
  }
}

function currentReceipt() {
  assertFixedPoint();
  const source = digestFiles(sourceRoots);
  const artifact = digestFiles(artifacts);
  return {
    schema: "pnix.cljs-meta-build-identity.v1",
    source_digest: source.digest,
    artifact_digest: artifact.digest,
    source_files: source.files,
    artifact_files: artifact.files
  };
}

if (mode === "--write") {
  const receipt = currentReceipt();
  fs.writeFileSync(receiptPath, JSON.stringify(receipt, null, 2) + "\n");
  console.log("cljs-meta artifact identity: WROTE " + receiptPath);
} else if (mode === "--check") {
  const expected = JSON.parse(fs.readFileSync(receiptPath, "utf8"));
  const actual = currentReceipt();
  if (JSON.stringify(expected) !== JSON.stringify(actual)) {
    console.error("cljs-meta artifact identity: source/artifact drift");
    process.exit(1);
  }
  console.log("cljs-meta artifact identity: PASS " + actual.artifact_digest);
} else {
  console.error("usage: meta-artifact-identity.js --write|--check");
  process.exit(2);
}
