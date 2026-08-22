const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const pnix = require("../dist/pnix-cljs-module.js");

function done(source, expected) {
  const result = pnix.evalSource(source);
  assert.equal(result.outcome_kind, "done", source);
  assert.deepEqual(result.value, expected, source);
}

function failed(source, expectedClass) {
  const result = pnix.evalSource(source);
  assert.equal(result.outcome_kind, "failed", source);
  assert.equal(result.error.class, expectedClass, source);
}

done("20 + 22", 42n);
done("5 / 2", 2n);
done("let double = x: x * 2; in double 21", 42n);
done("if true then 42 else missing", 42n);
done("let unused = unused; in 42", 42n);
done("rec { answer = base + 2; base = 40; }.answer", 42n);
done("{ answer = 42; }.answer", 42n);
done("# comment\n20 + 22", 42n);
done("9223372036854775807", 9223372036854775807n);
done("-9223372036854775808", -9223372036854775808n);
done('{ outcome_kind = "done"; }', {outcome_kind: "done"});
done('builtins.toJSON { outPath = "/x"; other = 1; }', '"/x"');
done('builtins.toJSON { __toString = self: self.name; name = "n"; outPath = "/x"; }', '"n"');

failed("1 / 0", "division-by-zero");
failed("9223372036854775807 + 1", "integer-overflow");
failed("missing", "unknown-variable");
failed("{}.missing", "attribute-missing");
failed("1 2", "not-callable");
failed("true + 1", "type-error");
failed("let value = value; in value", "cycle-detected");
failed("let x = ; in x", "syntax-error");
failed("@", "syntax-error");
failed("{ a = 1; a = 2; }", "duplicate-attrset-binding");
failed('builtins.toJSON { __toString = self: 42; }', "type-error");
done("(x: x) == (x: x)", false);

const fixture = fs.readFileSync(
  path.join(__dirname, "../examples/00-foundation/program.px"),
  "utf8"
);
const fixtureResult = pnix.evalSource(fixture);
assert.equal(fixtureResult.outcome_kind, "done");
assert.deepEqual(fixtureResult.value, {answer: 42n, selected: 42n});
assert.equal(
  pnix.evalSourceJson("9223372036854775807"),
  '{"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":9223372036854775807}'
);

const readinessLibrary = path.join(
  __dirname,
  "../examples/production-readiness/library.px"
);
assert.equal(pnix.callFileValueJson(readinessLibrary, "double", "[21]"), "42");
assert.equal(
  pnix.callFileValueJson(readinessLibrary, "mapDouble", "[[1,2,3]]"),
  "[2,4,6]"
);
assert.deepEqual(
  pnix.callFile(readinessLibrary, "summarize", "[[1,2,3,4]]"),
  {
    schema: "pnix.machine.host-outcome.v1",
    outcome_kind: "done",
    value: {count: 4n, total: 10n},
  }
);

console.log("pnix-cljs JavaScript runtime matrix: PASS");
