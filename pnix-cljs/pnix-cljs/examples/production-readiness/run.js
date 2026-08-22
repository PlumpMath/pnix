#!/usr/bin/env node
const fs = require("fs");
const path = require("path");

const here = __dirname;
const repo = path.resolve(here, "../../..");
const pnix = require(path.join(repo, "pnix-cljs/dist/pnix-cljs-module.js"));
const cljsMeta = require(path.join(repo, "cljs-meta/dist/fixed-point/cljs-meta-fixed.js"));
const receipt = JSON.parse(
  fs.readFileSync(path.join(repo, "cljs-meta/dist/fixed-point/receipt.json"), "utf8")
);

function done(name) {
  const result = pnix.evalFile(path.join(here, name));
  if (result.outcome_kind !== "done") {
    throw new Error(`${name}: ${JSON.stringify(result)}`);
  }
  return result.value;
}

async function main() {
  const direct = done("direct.px");
  const consumer = done("consumer.px");
  const selfHosted = done("self_interpreter.px");

  // Host import returns native JavaScript objects/arrays and exact BigInts.
  if (direct.mode !== "direct-runtime" || direct.value !== 42n) throw new Error("direct");
  if (consumer.answer !== 42n || consumer.total !== 10n) throw new Error("consumer");
  if (consumer.mapped.join(",") !== "2,4,6") throw new Error("consumer map");
  if (selfHosted.mode !== "pnix-in-pnix" || selfHosted.value !== 42n) throw new Error("self");

  // Host -> exported PNIX stdlib-style function, with exact JSON-safe data.
  const library = path.join(here, "library.px");
  if (pnix.callFileValueJson(library, "double", "[21]") !== "42") {
    throw new Error("callFile double");
  }
  if (pnix.callFileValueJson(library, "mapDouble", "[[1,2,3]]") !== "[2,4,6]") {
    throw new Error("callFile mapDouble");
  }

  if (!receipt.fixed_point || receipt.stage_count < 15) throw new Error("fixed point receipt");
  const metaResult = await cljsMeta.evaluate("(let [x 20] (+ x 22))");
  if (metaResult.outcome_kind !== "done" || metaResult.value !== 42) {
    throw new Error(`cljs-meta: ${JSON.stringify(metaResult)}`);
  }
  console.log("PASS pnix-cljs production-readiness");
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
