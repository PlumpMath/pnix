#!/usr/bin/env node
"use strict";

const cljs = require("../dist/fixed-point/cljs-meta-fixed.js");

async function main() {
  const evaluated = await cljs.evaluate("(let [x 20] (+ x 22))");
  const compiled = await cljs.compile("(defn answer [] 42)");

  if (evaluated.outcome_kind !== "done" || evaluated.value !== 42) {
    throw new Error(`unexpected evaluation: ${JSON.stringify(evaluated)}`);
  }
  if (compiled.outcome_kind !== "done" ||
      typeof compiled.value !== "string") {
    throw new Error(`unexpected compilation: ${JSON.stringify(compiled)}`);
  }

  console.log(JSON.stringify({evaluated, compiled_bytes: compiled.value.length}));
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
