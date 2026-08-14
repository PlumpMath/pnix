// pnix-cljs 게스트 평가 — 구조화된 결과 (ambient Node eval 아님).
const path = require("path");
const pnix = require(path.join(__dirname, "..", "..", "dist", "pnix-cljs-module.js"));

const pure = pnix.evalSource("1 + 2 * 3");
console.log("순수:", pure);

const div0 = pnix.evalSource("1 / 0");
console.log("div0:", div0);

// 게스트 소스는 평가기 API 로 Node require/fs 에 닿지 않는다.
// (hy/rs 수준의 전체 effect 게이트를 주장하지 않음 — admitted seed 만.)
