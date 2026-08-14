// 리스트 고차 builtins (seed). BigInt 값은 문자열로 찍는다.
const path = require("path");
const pnix = require(path.join(__dirname, "..", "..", "dist", "pnix-cljs-module.js"));

function show(source) {
  const r = pnix.evalSource(source);
  let v = r.value;
  if (typeof v === "bigint") v = v.toString();
  else if (Array.isArray(v)) v = v.map((x) => (typeof x === "bigint" ? x.toString() : x));
  console.log(source, "=>", r.outcome_kind, v);
}

[
  "builtins.map (x: x + 1) [1 2 3]",
  "builtins.filter (x: x > 1) [1 2 3]",
  "builtins.genList (i: i * 2) 3",
  "builtins.concatLists [[1] [2 3]]",
  "builtins.head [9 8]",
  "builtins.elem 2 [1 2 3]",
].forEach(show);
