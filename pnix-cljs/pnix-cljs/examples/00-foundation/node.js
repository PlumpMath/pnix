// 00-foundation — dist 모듈로 seed 폼을 돌린다.
const pnix = require("../../dist/pnix-cljs-module.js");

const cases = [
  "20 + 22",
  "let double = x: x * 2; in double 21",
  "if true then { answer = 42; }.answer else 0",
  "rec { answer = base + 2; base = 40; }.answer",
  "1 / 0",
];

for (const source of cases) {
  console.log(source, pnix.evalSource(source));
}
