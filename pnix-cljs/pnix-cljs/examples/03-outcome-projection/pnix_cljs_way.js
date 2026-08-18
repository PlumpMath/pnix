const path = require("path");
const pnix = require(path.join(__dirname, "..", "..", "dist", "pnix-cljs-module.js"));

// evalSource 값은 BigInt일 수 있어 JSON.stringify 기본 replacer가 그냥 못
// 찍는다 (관측용 문자열 변환이지, 값 자체의 타입이 바뀌는 건 아니다).
const bigintSafe = (_key, value) => (typeof value === "bigint" ? value.toString() : value);

for (const src of ["20 + 22", "1 / 0", "missing_var"]) {
  const r = pnix.evalSource(src);
  console.log(JSON.stringify({ source: src, result: r }, bigintSafe));
}
