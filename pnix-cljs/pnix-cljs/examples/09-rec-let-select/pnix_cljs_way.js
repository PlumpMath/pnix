// rec / let / select 핵심 문법 스모크.
const path = require("path");
const pnix = require(path.join(__dirname, "..", "..", "dist", "pnix-cljs-module.js"));

const cases = [
  "let x = 40; in x + 2",
  "rec { a = b + 1; b = 41; }.a",
  "{ answer = 42; }.answer",
  "if false then 0 else 7",
  "let s = { a = { b = 1; }; }; in s.a.b",
];

for (const source of cases) {
  console.log(source, "=>", pnix.evalSource(source));
}
