// seed builtins 샘플 — 전면 Nix 패리티를 주장하지 않는다.
const path = require("path");
const pnix = require(path.join(__dirname, "..", "..", "dist", "pnix-cljs-module.js"));

const cases = [
  "builtins.typeOf 1",
  "builtins.typeOf \"hi\"",
  "builtins.attrNames { b = 2; a = 1; }",
  "builtins.getAttr \"a\" { a = 42; }",
  "builtins.length [1 2 3]",
];

for (const source of cases) {
  console.log(source, "=>", pnix.evalSource(source));
}
