// 문자열·버전 helpers.
const path = require("path");
const pnix = require(path.join(__dirname, "..", "..", "dist", "pnix-cljs-module.js"));

function show(source) {
  const r = pnix.evalSource(source);
  console.log(source, "=>", r.outcome_kind, r.value);
}

[
  "builtins.substring 1 2 \"abcd\"",
  "builtins.concatStringsSep \",\" [\"a\" \"b\"]",
  "builtins.splitVersion \"1.2.3\"",
  "builtins.toString 42",
  "toString true",
  "builtins.hasAttr \"a\" { a = 1; }",
].forEach(show);
