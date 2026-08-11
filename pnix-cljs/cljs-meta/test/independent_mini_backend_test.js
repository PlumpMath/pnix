const assert = require("node:assert/strict");
const cljsMeta = require("../dist/cljs-meta-module.js");
const mini = require("../independent_mini_backend.js");

// Trusting-Trust (Diverse Double-Compiling) witness: cross-check the
// self-hosted cljs.js-backed compiler against a from-scratch, independent
// tokenizer/reader + direct JS-text emitter (independent_mini_backend.js)
// that shares no code with cljs.js/cljs.compiler/cljs.analyzer. See
// STATUS.md's "Trusting-Trust defense roadmap" for the honest scope: this is
// a bounded fixture subset, not full ClojureScript, and behavior equivalence
// (not bit-identical JS text) is the bar.
const FIXTURES = [
  ["(let [x 20] (+ x 22))", 42],
  ["(if true :yes :no)", "yes"],
  ["(let [x 5 y 7] (if (< x y) (* (+ x 1) y) (- x y)))", 42],
  ["(+ 1 2)", 3],
  ["(- 50 8)", 42],
  ["(* 6 7)", 42],
  ["(if false 0 42)", 42],
  ["(let [x 41] (+ x 1))", 42],
];

(async () => {
  for (const [source, expected] of FIXTURES) {
    const hostResult = await cljsMeta.evaluate(source);
    assert.equal(hostResult.outcome_kind, "done", `host failed on: ${source}`);
    assert.equal(hostResult.value, expected, `host mismatch on: ${source}`);

    const miniResult = mini.compileAndEval(source);
    assert.equal(miniResult, expected, `mini backend mismatch on: ${source}`);
  }

  console.log(`independent mini backend DDC: PASS (${FIXTURES.length} fixtures)`);
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
