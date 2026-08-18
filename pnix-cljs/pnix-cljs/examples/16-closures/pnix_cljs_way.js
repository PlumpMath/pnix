// 클로저: 바깥 바인딩 캡처 + 다회 재호출. BigInt 값은 문자열로 찍는다.
const path = require("path");
const pnix = require(path.join(__dirname, "..", "..", "dist", "pnix-cljs-module.js"));

function show(source) {
  const r = pnix.evalSource(source);
  let v = r.value;
  if (typeof v === "bigint") v = v.toString();
  else if (Array.isArray(v)) v = v.map((x) => (typeof x === "bigint" ? x.toString() : x));
  console.log(source, "=>", r.outcome_kind, v);
}

show("let make_adder = n: (x: x + n); add5 = make_adder 5; in add5 10");
show(
  "let counter_maker = start: (step: start + step); c = counter_maker 100; in [(c 1) (c 2) (c 3)]"
);
