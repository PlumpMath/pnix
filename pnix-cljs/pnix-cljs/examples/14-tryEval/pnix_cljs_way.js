// tryEval — throw 는 catch, 성공 경로도 확인.
const path = require("path");
const pnix = require(path.join(__dirname, "..", "..", "dist", "pnix-cljs-module.js"));

function show(source) {
  const r = pnix.evalSource(source);
  console.log(source, "=>", r.outcome_kind, r.value);
}

[
  "builtins.tryEval (1 + 1)",
  "builtins.tryEval (throw \"x\")",
].forEach(show);
