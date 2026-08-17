const assert = require("node:assert/strict");
const cljsMeta = require("../dist/cljs-meta-module.js");
const mini = require("../independent_mini_backend.js");

// Trusting-Trust (Diverse Double-Compiling) witness: cross-check the
// self-hosted cljs.js-backed compiler against a from-scratch, independent
// tokenizer/reader + direct JS-text emitter (independent_mini_backend.js)
// that shares no code with cljs.js/cljs.compiler/cljs.analyzer. See
// STATUS.md's "Trusting-Trust defense roadmap" for the honest scope: this is
// a bounded fixture subset, not full ClojureScript, and behavior equivalence
// (not bit-identical JS text) is the bar.
const FIXTURES = [
  ["(let [x 20] (+ x 22))", 42],
  ["(if true :yes :no)", "yes"],
  ["(let [x 5 y 7] (if (< x y) (* (+ x 1) y) (- x y)))", 42],
  ["(+ 1 2)", 3],
  ["(- 50 8)", 42],
  ["(* 6 7)", 42],
  ["(if false 0 42)", 42],
  ["(let [x 41] (+ x 1))", 42],
  ['"hello"', "hello"],
  ["(do 1 2 3)", 3],
  ["(let [a 1 b 2] [a b])", [1, 2]],
  ["((fn [x] (* x x)) 6)", 36],
  ["((fn fact [n] (if (<= n 1) 1 (* n (fact (- n 1))))) 6)", 720],
  ["((fn fib [n] (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) 10)", 55],
  ["{:a 1 :b 2}", { a: 1, b: 2 }],
  ["(get {:a 1 :b 2} :a)", 1],
  ["(count [1 2 3])", 3],
  ["(conj [1 2] 3)", [1, 2, 3]],
  ["(nth [10 20 30] 1)", 20],
  ["(let [[a b] [20 22]] (+ a b))", 42],
  ["(let [[a b c] [20 22]] (if (nil? c) (+ a b) 0))", 42],
  ["(let [[[a b] c] [[1 2] 3]] (+ (+ a b) c))", 6],
  ["(let [[a [b c] d] [1 [2 3] 4]] (+ (+ a b) (+ c d)))", 10],
  ["#{1 2 3}", [1, 2, 3]],
  ["(count #{1 2 3})", 3],
  ["(assoc {:a 1} :b 2)", { a: 1, b: 2 }],
  ["(assoc {:a 1} :b 2 :c 3)", { a: 1, b: 2, c: 3 }],
  ["(update {:a 1} :a (fn [x] (+ x 1)))", { a: 2 }],
  ["(when true 42)", 42],
  ["(when false 42)", null],
  ["(when (> 5 3) (+ 1 2))", 3],
  ["(cond false 1 true 2 :else 3)", 2],
  ["(cond false 1 false 2 :else 99)", 99],
  ["(-> 1 (+ 2) (* 3))", 9],
  ["(loop [i 0 acc 0] (if (< i 10) (recur (+ i 1) (+ acc i)) acc))", 45],
  ["(loop [i 5 acc 1] (if (<= i 1) acc (recur (- i 1) (* acc i))))", 120],
  ["(loop [a 1 b 2 n 3] (if (= n 0) a (recur b a (- n 1))))", 2],
  ["(let [square (fn [x] (* x x))] (+ (square 3) (square 4)))", 25],
  ["(let [addA (fn [x] (+ x 5))] (- (addA 10) (addA 1)))", 9],
  ["(let [addAB (fn [a b] (+ a b))] (addAB 14 (addAB 14 14)))", 42],
  [
    "(let [base (fn [x] (+ x 3)) scaled (fn [y] (* (base y) 2))] (+ (scaled 1) (scaled 2)))",
    18,
  ],
];

(async () => {
  for (const [source, expected] of FIXTURES) {
    const hostResult = await cljsMeta.evaluate(source);
    assert.equal(hostResult.outcome_kind, "done", `host failed on: ${source}`);
    // deepEqual (not equal/strictEqual): some fixtures return vectors, which
    // cljs.js and the mini backend each produce as freshly-allocated JS
    // arrays, so reference equality would spuriously fail structurally-equal
    // results.
    assert.deepEqual(hostResult.value, expected, `host mismatch on: ${source}`);

    const miniResult = mini.compileAndEval(source);
    assert.deepEqual(miniResult, expected, `mini backend mismatch on: ${source}`);
  }

  console.log(`independent mini backend DDC: PASS (${FIXTURES.length} fixtures)`);
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
