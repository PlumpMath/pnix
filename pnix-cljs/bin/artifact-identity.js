#!/usr/bin/env node
"use strict";

const crypto = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "..");
const receiptPath = path.join(root, "build-receipt.json");
const mode = process.argv[2];

const sourceRoots = [
  "bin/build-cljs",
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
  "pnix-cljs/deps.edn",
  "pnix-cljs/build-cli.edn",
  "pnix-cljs/build-module.edn",
  "pnix-cljs/build-test.edn",
  "pnix-cljs/package.json",
  "pnix-cljs/src",
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
  "cljs-meta/dist/fixed-point/receipt.json",
  "cljs-meta/dist/meta-build-receipt.json",
  "pnix-cljs/dist/pnix-cljs.js",
  "pnix-cljs/dist/pnix-cljs-module.js",
  "pnix-cljs/dist/pnix-cljs-self-test.js"
];

function sha256(parts) {
  const hash = crypto.createHash("sha256");
  for (const part of parts) hash.update(part);
  return hash.digest("hex");
}

function filesBelow(relative) {
  const absolute = path.join(root, relative);
  const stat = fs.lstatSync(absolute);
  if (!stat.isDirectory()) return [relative];
  return fs.readdirSync(absolute).sort().flatMap((name) =>
    filesBelow(path.posix.join(relative, name))
  );
}

function digestFiles(relativePaths) {
  const files = relativePaths.flatMap(filesBelow).sort();
  return {
    files,
    digest: sha256(files.flatMap((relative) => [
      Buffer.from(relative),
      Buffer.from([0]),
      fs.readFileSync(path.join(root, relative)),
      Buffer.from([0])
    ]))
  };
}

function currentReceipt() {
  const source = digestFiles(sourceRoots);
  const artifact = digestFiles(artifacts);
  return {
    schema: "pnix.cljs-build-identity.v1",
    source_digest: source.digest,
    artifact_digest: artifact.digest,
    source_files: source.files,
    artifact_files: artifact.files
  };
}

if (mode === "--write") {
  fs.writeFileSync(receiptPath, JSON.stringify(currentReceipt(), null, 2) + "\n");
  console.log("pnix-cljs artifact identity: WROTE " + receiptPath);
} else if (mode === "--check") {
  const expected = JSON.parse(fs.readFileSync(receiptPath, "utf8"));
  const actual = currentReceipt();
  if (JSON.stringify(expected) !== JSON.stringify(actual)) {
    console.error("pnix-cljs artifact identity: source/artifact drift");
    process.exit(1);
  }
  console.log("pnix-cljs artifact identity: PASS " + actual.artifact_digest);
} else {
  console.error("usage: artifact-identity.js --write|--check");
  process.exit(2);
}
