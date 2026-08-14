// with / assert / ++ / //
const path = require("path");
const pnix = require(path.join(__dirname, "..", "..", "dist", "pnix-cljs-module.js"));

function show(source) {
  const r = pnix.evalSource(source);
  let v = r.value;
  if (typeof v === "bigint") v = v.toString();
  console.log(source, "=>", r.outcome_kind, v);
}

[
  "with { a = 1; }; a",
  "assert true; 9",
  "[1 2] ++ [3]",
  "{ a = 1; } // { b = 2; }",
  "/* 주석 */ 1 + 1",
].forEach(show);
