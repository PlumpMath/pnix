const assert = require("node:assert/strict");
const fs = require("node:fs");

const receipt = JSON.parse(
  fs.readFileSync("cljs-meta/dist/fixed-point/receipt.json", "utf8")
);
const fixed = require("../dist/fixed-point/cljs-meta-fixed.js");

assert.equal(receipt.schema, "pnix.cljs-meta.fixed-point.v1");
assert.equal(receipt.fixed_point, true);
assert.equal(receipt.source_closure_equal, true);
assert.equal(receipt.stage2_artifact_sha256, receipt.stage3_artifact_sha256);
assert.equal(receipt.stage0_compiler_embedded, false);
assert.equal(receipt.bootstrap_only_markers_absent, true);
assert.equal(receipt.compiler_payload_self_hosted, true);
assert.ok(receipt.minimum_stage_count >= 15);
assert.ok(receipt.stage_count >= receipt.minimum_stage_count);
assert.equal(receipt.converged_at_stage, receipt.stage_count);
assert.equal(receipt.stage2_compiler_input_sha256,
             receipt.stage1_artifact_sha256);
assert.equal(receipt.stage3_compiler_input_sha256,
             receipt.stage2_artifact_sha256);

Promise.all([
  fixed.evaluate("(let [x 20] (+ x 22))"),
  fixed.compile("(defn answer [] 42)")
]).then(([evaluated, compiled]) => {
  assert.equal(evaluated.outcome_kind, "done");
  assert.equal(evaluated.value, 42);
  assert.equal(compiled.outcome_kind, "done");
  assert.equal(typeof compiled.value, "string");
  assert.ok(compiled.value.includes("answer"));
  console.log("cljs-meta fixed-point runtime: PASS");
}).catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
