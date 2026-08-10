const assert = require("node:assert/strict");
const cljsMeta = require("../dist/cljs-meta-module.js");

(async () => {
  const arithmetic = await cljsMeta.evaluate("(let [x 20] (+ x 22))");
  assert.equal(arithmetic.outcome_kind, "done");
  assert.equal(arithmetic.value, 42);

  const conditional = await cljsMeta.evaluate("(if true :yes :no)");
  assert.equal(conditional.outcome_kind, "done");
  assert.equal(conditional.value, "yes");

  const invalid = await cljsMeta.evaluate("(+ 1");
  assert.equal(invalid.outcome_kind, "failed");
  assert.equal(invalid.error.class, "clojurescript-evaluation-error");

  console.log("cljs-meta self-host matrix: PASS");
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
