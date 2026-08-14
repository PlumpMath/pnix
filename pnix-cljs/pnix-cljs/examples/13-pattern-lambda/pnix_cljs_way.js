// 패턴 람다 (attrset formal).
const path = require("path");
const pnix = require(path.join(__dirname, "..", "..", "dist", "pnix-cljs-module.js"));

function show(source) {
  const r = pnix.evalSource(source);
  let v = r.value;
  if (typeof v === "bigint") v = v.toString();
  console.log(source, "=>", r.outcome_kind, v);
}

[
  "({ a }: a) { a = 7; }",
  "({ a ? 2 }: a) {}",
  "({ a, b ? a }: b) { a = 3; }",
  "let f = x: y: x + y; in f 1 2",
].forEach(show);
