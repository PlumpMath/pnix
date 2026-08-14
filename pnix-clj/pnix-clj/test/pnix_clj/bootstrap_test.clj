(ns pnix-clj.bootstrap-test
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.string :as str]
            [clojure.test :refer [deftest is testing]]
            [pnix-clj.benchmark :as benchmark]
            [pnix-clj.classfile-receipt :as classfile-receipt]
            [pnix-clj.clojure-form :as clojure-form]
            [pnix-clj.clojure-projection :as clojure-projection]
            [pnix-clj.core :as pnix]
            [pnix-clj.coverage :as coverage]
            [pnix-clj.determinism :as determinism]
            [pnix-clj.emit-form-roundtrip :as emit-form-roundtrip]
            [pnix-clj.evaluator :as evaluator]
            [pnix-clj.forward-reference :as forward-reference]
            [pnix-clj.grammar-fuzzer :as grammar-fuzzer]
            [pnix-clj.interop :as interop]
            [pnix-clj.live-oracle :as live-oracle]
            [pnix-clj.lowering :as lowering]
            [pnix-clj.mirror :as mirror]
            [pnix-clj.mirror-chain :as mirror-chain]
            [pnix-clj.machine :as machine]
            [pnix-clj.mirror-error :as mirror-error]
            [pnix-clj.mirror-pair :as mirror-pair]
            [pnix-clj.oracle :as oracle]
            [pnix-clj.parser :as parser]
            [pnix-clj.px-runtime :as px-runtime]
            [pnix-clj.report-artifact :as report-artifact]
            [pnix-clj.rust-batch :as rust-batch]
            [pnix-clj.arith-proof :as arith-proof]
            [pnix-clj.cas :as cas]
            [pnix-clj.store :as store]
            [pnix-clj.bool-proof :as bool-proof]
            [pnix-clj.form-analysis :as form-analysis]
            [pnix-clj.property-fuzzer :as property-fuzzer]
            [pnix-clj.futamura :as futamura]
            [pnix-clj.weval :as weval]
            [pnix-clj.wiki :as wiki]
            [pnix-clj.lane-registry :as lane-registry]
            [pnix-clj.persist :as persist]
            [pnix-clj.purity :as purity]
            [pnix-clj.replay :as replay]
            [pnix-clj.snapshot :as snapshot]
            [pnix-clj.specialize :as specialize]
            [pnix-clj.synthesize :as synthesize]
            [pnix-clj.cached-eval :as cached-eval]
            [pnix-clj.capabilities :as capabilities]
            [pnix-clj.receipt :as receipt]
            [pnix-clj.search :as search]
            [pnix-clj.reflect :as reflect]
            [pnix-clj.safe-eval :as safe-eval]
            [pnix-clj.tower :as tower]
            [pnix-clj.unparse :as unparse]
            [pnix-clj.stage7-core :as stage7-core]
            [pnix-clj.stage15 :as stage15]
            [pnix-clj.strict-audit :as strict-audit]
            [pnix-clj.translation-validation :as translation-validation]
            [pnix-clj.trust :as trust]
            [pnix-clj.witness :as witness]
            [pnix-clj.cegis :as cegis]
            [pnix-clj.generate :as generate]
            [pnix-clj.self-improve :as self-improve]
            [pnix-clj.self-mod-gate :as self-mod-gate]
            [pnix-clj.witnessed-run :as witnessed-run]
            [pnix-clj.value-roundtrip :as value-roundtrip]
            [pnix.clj-meta.host-reflection :as host-reflection]
            [pnix-clj.version :as version])
  (:import [java.io File]
           [java.awt Point]))

(deftest parser-literal-smoke
  (testing "literal source parses to stable AST data"
    (is (= {:op :int
            :value 42
            :span [0 2]}
           (select-keys (:ast (parser/parse-source "42"))
                        [:op :value :span])))
    (is (= {:op :var
            :name "x"
            :span [0 1]}
           (select-keys (:ast (parser/parse-source "x"))
                        [:op :name :span])))
    (let [held (parser/parse-source "@")]
      (is (= :failed (:status held)))
      (is (= :unsupported-syntax (:reason held)))
      (is (= :parse (get-in held [:error :phase])))
      (is (= :syntax-error (get-in held [:error :class])))
      (is (= [0 1] (:span held)))
      (is (= [0 1] (get-in held [:unsupported-syntax 0 :span]))))))

(deftest parse-cache-is-keyed-by-source-hash
  (parser/clear-parse-cache!)
  (let [first-parse (parser/parse-source "42")
        second-parse (parser/parse-source "42")
        stats (parser/parse-cache-stats)]
    (is (= first-parse second-parse))
    (is (= :pnix-clj.parse-cache-key.v0
           (get-in first-parse [:cache-key :schema])))
    (is (= (:source-hash first-parse)
           (get-in first-parse [:cache-key :source-hash])))
    (is (= {:hits 1
            :misses 1
            :entries 1}
           stats))))

(deftest lowering-cache-is-keyed-by-ast-hash-and-policy
  (parser/clear-parse-cache!)
  (lowering/clear-lower-cache!)
  (let [ast (:ast (parser/parse-source "{ x = 1; } ? x"))
        first-lowering (lowering/lower-ast ast)
        second-lowering (lowering/lower-ast ast)
        stats (lowering/lower-cache-stats)]
    (is (= first-lowering second-lowering))
    (is (= :pnix-clj.lower-cache-key.v0
           (get-in first-lowering [:cache-key :schema])))
    (is (= (:ast-hash first-lowering)
           (get-in first-lowering [:cache-key :ast-hash])))
    (is (= :expr-core-v1
           (get-in first-lowering [:cache-key :policy])))
    (is (= false (:source-string-codegen? first-lowering)))
    (is (= {:hits 1
            :misses 3
            :entries 3}
           stats))))

(deftest parser-runtime-grammar-smoke
  (testing "runtime .px grammar pieces parse as data before execution"
    (is (= :ok (:status (parser/parse-source "# comment\n42"))))
    (is (= {:op :import
            :target "./foo/bar.px"}
           (select-keys (:ast (parser/parse-source "import ./foo/bar.px"))
                        [:op :target])))
    (is (= {:op :has-attr
            :attr "visibility"}
           (select-keys (:ast (parser/parse-source "req ? visibility"))
                        [:op :attr])))
    (is (= :not (:op (:ast (parser/parse-source "!false")))))
    (is (= "&&" (:operator (:ast (parser/parse-source "true && false")))))
    (is (= ["x" "y"]
           (mapv :key (get-in (parser/parse-source "{ inherit x y; }")
                              [:ast :attrs]))))
    (let [pattern (get-in (parser/parse-source
                           "builtins.functionArgs ({ x, y ? 1, ... }: x + y)")
                          [:ast :arg :param-pattern])]
      (is (= :attr-pattern (:kind pattern)))
      (is (= true (:ellipsis? pattern)))
      (is (= [{:name "x" :has-default? false}
              {:name "y" :has-default? true}]
             (mapv (fn [param]
                     {:name (:name param)
                      :has-default? (contains? param :default)})
                   (:params pattern)))))
    (is (= true (:value (pnix/eval-source "{ x = 1; } ? x"))))
    (is (= :ok (:status (pnix/lower-source "{ x = 1; } ? x"))))))

(deftest parser-select-or-default-grammar
  (testing "select-or fallback uses Nix's tight select-default grammar"
    (doseq [source ["{ a = 1; }.b or if true then 8 else 9"
                   "{ a = 1; }.b or let x = 8; in x"
                   "{ a = 1; }.b or assert true; 8"
                   "{ a = 1; }.b or with {}; 8"
                   "{ a = 1; }.b or x: x"
                   "{ a = 1; }.b or { x }: x"]]
      (let [held (parser/parse-source source)]
        (is (= :failed (:status held)) source)
        (is (= :unsupported-syntax (:reason held)) source)))
    (let [ast (:ast (parser/parse-source "{ a = 1; }.a or import ./m"))]
      (is (= :call (:op ast)))
      (is (= :select (get-in ast [:fn :op])))
      (is (= {:op :var :name "import"}
             (select-keys (get-in ast [:fn :default]) [:op :name]))))))

(deftest parser-list-attrset-smoke
  (testing "literal list and attrset parse to Clojure data AST"
    (is (= :list (get-in (parser/parse-source "[ 1 true \"x\" ]") [:ast :op])))
    (is (= [:int :bool :string]
           (mapv :op (get-in (parser/parse-source "[ 1 true \"x\" ]") [:ast :items]))))
    (is (= :attrset (get-in (parser/parse-source "{ x = 1; y = \"z\"; }") [:ast :op])))
    (is (= ["x" "y"]
           (mapv :key (get-in (parser/parse-source "{ x = 1; y = \"z\"; }")
                              [:ast :attrs]))))))

(deftest parser-identifier-path-syntax
  (testing "identifiers"
    (let [ast (parser/parse-source "fooBar_1")]
      (is (= :ok (:status ast)))
      (is (= :var (:op (:ast ast))))
      (is (= "fooBar_1" (get-in ast [:ast :name])))))
  (testing "path literals"
    (doseq [source ["./foo/bar" "../x" "~/x" "<nixpkgs>" "/foo/bar"]]
      (let [ast (:ast (parser/parse-source source))]
        (is (= :path (:op ast)) source)
        (is (= :ok (:status (parser/parse-source source))) source)))))

(deftest deep-recursion-nix-parity
  ;; D1 (oracle-gated): real Nix evaluates 100k-deep parens/lists and 10k+
  ;; nested lets; we previously overflowed below 1k (parser) / 10k (eval).
  ;; Fixed by the deep-eval stack thread (core/call-on-deep-stack) + the
  ;; eval-let tail-position LOOP. Depths here sit well beyond the old cliffs
  ;; while keeping the gate fast; let-50k+ remains a filed divergence (D1b).
  (let [deep (fn [pre mid post n] (str (apply str (repeat n pre)) mid
                                       (apply str (repeat n post))))]
    (is (= 1 (:value (pnix/eval-source (deep "(" "1" ")" 20000)))))
    (let [v (:value (pnix/eval-source
                     (str (apply str (repeat 10000 "[ ")) " 1 "
                          (apply str (repeat 10000 " ]")))))]
      (is (vector? v) "10k-deep nested list evaluates (outermost has 1 elem)")
      (is (= 1 (count v))))
    (is (= 5001 (:value (pnix/eval-source (str "1" (apply str (repeat 5000 " + 1")))))))
    (let [nested-let (fn [d] (str (clojure.string/join
                                   " " (map #(str "let a" % " = "
                                                  (if (zero? %) "1" (str "a" (dec %)))
                                                  "; in") (range d)))
                                  " a" (dec d)))]
      (is (= 1 (:value (pnix/eval-source (nested-let 3000))))
          "beyond the old ~1k parser / 9-10k eval cliffs' first wall")))
  (testing "a stack overflow beyond the deep stack is STRUCTURED, never a crash"
    ;; the reason taxonomy exists even if hard to trigger cheaply here
    (is (fn? pnix/eval-source))))

(deftest builtins-strictness-nix-parity
  ;; D2 (oracle-gated): builtins strictness matrix vs nix-instantiate 2.34.7.
  ;; Every expectation below was confirmed against the real oracle; the full
  ;; 64-case probe reached 0 divergences. The five fixed divergence classes:
  ;; map/filter/mapAttrs/sort never force their function argument on an EMPTY
  ;; collection (literal or computed), foldl' passes the initial accumulator
  ;; lazily, and sort forces elements to WHNF (not deep) before comparing.
  (testing "empty collections short-circuit BEFORE the function arg is forced"
    (doseq [[src expected]
            [["builtins.length (builtins.map (throw \"BOOM\") [ ])" 0]
             ["builtins.map (throw \"BOOM\") (builtins.tail [ 1 ])" []]
             ["builtins.map 1 [ ]" []]
             ["builtins.length (builtins.filter (throw \"BOOM\") [ ])" 0]
             ["builtins.attrNames (builtins.mapAttrs (throw \"BOOM\") { })" []]
             ["builtins.attrNames (builtins.mapAttrs (throw \"BOOM\") (builtins.removeAttrs { a = 1; } [ \"a\" ]))" []]
             ["builtins.length (builtins.sort (throw \"BOOM\") [ ])" 0]]]
      (is (= expected (:value (pnix/eval-source src))) src)))
  (testing "non-empty (or non-list) DOES force the function arg, before the list type error"
    (doseq [src ["builtins.map (throw \"BOOM\") [ 1 ]"
                 "builtins.map (throw \"BOOM\") 1"]]
      (is (= :failed (:status (pnix/eval-source src))) src)))
  (testing "the asymmetric strict group forces its function arg EVEN on empty (real Nix does)"
    (doseq [src ["builtins.length (builtins.concatMap (throw \"BOOM\") [ ])"
                 "builtins.any (throw \"BOOM\") [ ]"
                 "builtins.all (throw \"BOOM\") [ ]"
                 "builtins.length (builtins.genList (throw \"BOOM\") 0)"
                 "builtins.foldl' (throw \"BOOM\") 0 [ ]"]]
      (is (= :failed (:status (pnix/eval-source src))) src)))
  (testing "foldl' initial accumulator is lazy; intermediate/final results are strict"
    (is (= 1 (:value (pnix/eval-source
                      "builtins.foldl' (a: b: b) (throw \"BOOM\") [ 1 ]"))))
    (is (= 0 (:value (pnix/eval-source
                      "builtins.foldl' (a: b: a) 0 [ (throw \"BOOM\") ]"))))
    (doseq [src ["builtins.foldl' (a: b: a) (throw \"BOOM\") [ 1 ]"
                 "builtins.foldl' (a: b: b) (throw \"BOOM\") [ ]"]]
      (is (= :failed (:status (pnix/eval-source src))) src)))
  (testing "sort forces elements to WHNF only"
    (is (= :failed (:status (pnix/eval-source
                           "builtins.sort (a: b: true) [ (throw \"BOOM\") 1 ]"))))
    (is (= 2 (:value (pnix/eval-source
                      "builtins.length (builtins.sort (a: b: true) [ [ (throw \"BOOM\") ] 1 ])")))))
  (testing "representative lazy positions stay lazy (oracle parity kept)"
    (doseq [[src expected]
            [["builtins.length [ (throw \"BOOM\") 2 ]" 2]
             ["builtins.elemAt [ (throw \"BOOM\") 2 ] 1" 2]
             ["builtins.length (builtins.map (x: throw \"BOOM\") [ 1 2 3 ])" 3]
             ["builtins.any (x: x) [ true (throw \"BOOM\") ]" true]
             ["builtins.elem 1 [ 1 (throw \"BOOM\") ]" true]
             ["builtins.attrNames { a = throw \"BOOM\"; }" ["a"]]
             ["builtins.attrNames (builtins.listToAttrs [ { name = \"a\"; value = throw \"BOOM\"; } ])" ["a"]]
             ["(builtins.tryEval [ (throw \"BOOM\") ]).success" true]
             ["builtins.seq [ (throw \"BOOM\") ] 2" 2]
             ["builtins.functionArgs ({ a ? throw \"BOOM\" }: 1)" {"a" true}]]]
      (is (= expected (:value (pnix/eval-source src))) src))))

(deftest error-taxonomy-tryeval-nix-parity
  ;; D3 (oracle-gated): catchable-vs-uncatchable matrix vs nix-instantiate
  ;; 2.34.7, 26 probe cases. The tryEval catch-set needed NO change: Nix
  ;; catches ONLY throw and assert; abort, type errors, division by zero,
  ;; missing attrs, out-of-bounds, fromJSON/toString failures, non-function
  ;; calls, missing pattern args, and black-hole self-reference all abort
  ;; evaluation even inside tryEval — and all held here, at parity. The only
  ;; 3 divergent rows are the R2 lenient defaults (non-bool if/assert, "+"
  ;; string coercion), already audited + held under opt-in strict mode whose
  ;; default flip is the owner-gated R2 Phase D; pinned below as-is.
  (testing "catchable: throw and assert only"
    (doseq [src ["builtins.tryEval (throw \"BOOM\")"
                 "builtins.tryEval (builtins.throw \"BOOM\")"
                 "builtins.tryEval ((x: x) (throw \"BOOM\"))"
                 "builtins.tryEval (if (throw \"BOOM\") then 1 else 2)"
                 "builtins.tryEval (builtins.deepSeq [ (throw \"BOOM\") ] 1)"
                 "builtins.tryEval (assert false; 1)"
                 "builtins.tryEval (assert (throw \"BOOM\"); 1)"]]
      (is (= {"success" false "value" false} (:value (pnix/eval-source src)))
          src)))
  (testing "tryEval is shallow and composes"
    (is (= true (:value (pnix/eval-source
                         "(builtins.tryEval [ (throw \"BOOM\") ]).success"))))
    (is (= false (:value (pnix/eval-source
                          "(builtins.tryEval (builtins.tryEval (throw \"BOOM\"))).value.success")))))
  (testing "uncatchable: everything else propagates as held through tryEval"
    (doseq [[src reason]
            [["builtins.tryEval (abort \"BOOM\")" :abort-builtin-called]
             ["builtins.tryEval (builtins.abort \"BOOM\")" :abort-builtin-called]
             ["builtins.tryEval (builtins.throw 42)" :throw-argument-not-string]
             ["builtins.tryEval (1 / 0)" :eval-binary-failed]
             ["builtins.tryEval ({ }.a)" :missing-attr]
             ["builtins.tryEval (builtins.getAttr \"a\" { })" :get-attr-missing]
             ["builtins.tryEval (builtins.head [ ])" :head-of-empty-list]
             ["builtins.tryEval (builtins.elemAt [ 1 ] 5)" :builtin-dispatch-failed]
             ["builtins.tryEval (builtins.fromJSON \"{\")" :from-json-builtin-failed]
             ["builtins.tryEval (toString { })" :to-string-builtin-failed]
             ["builtins.tryEval (1 2)" :call-target-not-callable]
             ["builtins.tryEval (({ a }: a) { })" :missing-lambda-pattern-arg]
             ["builtins.tryEval (let x = x; in x)" :infinite-recursion]]]
      (let [r (pnix/eval-source src)]
        (is (= :failed (:status r)) src)
        (is (= reason (:reason r)) src))))
  (testing "type errors hold BY DEFAULT and stay uncaught by tryEval (Phase D)"
    ;; R2 Phase D landed (owner doctrine, 2026-07-07): the former truthy
    ;; leniency was a Clojure HOST LEAK, removed — strict Nix typing is
    ;; pnix's only semantics, and these hold uncaught exactly like real
    ;; Nix type errors (oracle-confirmed in the D3 matrix).
    (doseq [[src reason]
            [["builtins.tryEval (if 1 then 1 else 2)" :non-bool-if-condition]
             ["builtins.tryEval (assert 1; 2)" :non-bool-assert-condition]
             ["builtins.tryEval (\"x\" + 1)" :string-coercion]]]
      (doseq [r [(pnix/eval-source src) (pnix/eval-source-strict src)]]
        (is (= :failed (:status r)) src)
        (is (= reason (:reason r)) src)))))

(deftest float-parity-nix-parity
  ;; D4 (oracle-gated): float formatting/semantics matrix vs nix-instantiate
  ;; 2.34.7 — 47 probe cases, 0 divergent after two fixes: (1) toString floats
  ;; are C %.6f over the EXACT binary value (nix-float-str: BigDecimal, not
  ;; Java's shortest-repr %f) in the evaluator AND the lowering lane; (2) the
  ;; tokenizer float grammar is Nix's regex — `.5`, `1.`, `1.e3`, `2.5e-2`,
  ;; `1.5E+2` are floats, while `1e3` and `00.5` are applications and `{ }.1`
  ;; lexes `.1` as a float (flex maximal munch), all held exactly like Nix.
  (testing "toString floats are Nix %.6f (exact-value based)"
    (doseq [[src expected]
            [["toString 1.5" "1.500000"]
             ["toString 0.1" "0.100000"]
             ["toString 1.0" "1.000000"]
             ["toString (-0.0)" "0.000000"]
             ["toString (-0.0000001)" "-0.000000"]
             ["toString (-0.0000005)" "-0.000000"]
             ["toString (-0.0000006)" "-0.000001"]
             ["toString (0.0 / (-1.0))" "-0.000000"]
             ["toString ((-1.0) * 0.0)" "-0.000000"]
             ["toString (0.0 - 2.25)" "-2.250000"]
             ["toString 0.0000001" "0.000000"]
             ["toString (1000000.0 * 10000.0)" "10000000000.000000"]
             ["toString (0.1 + 0.2)" "0.300000"]
             ["toString (0 - 1.5)" "-1.500000"]
             ["toString 1." "1.000000"]
             ["builtins.toString [ 1.5 2.5 ]" "1.500000 2.500000"]
             ["\"v=${toString 2.5}\"" "v=2.500000"]
             ["toString (1 + 1)" "2"]]]
      (is (= expected (:value (pnix/eval-source src))) src))
    (is (= "1000000000000000019884624838656.000000"
           (evaluator/nix-float-str 1.0E30))
        "exact binary expansion, not shortest-repr padding"))
  (testing "toJSON floats stay shortest-round-trip (oracle parity kept)"
    (doseq [[src expected]
            [["builtins.toJSON 1.5" "1.5"]
             ["builtins.toJSON (0.1 + 0.2)" "0.30000000000000004"]
             ["builtins.toJSON [ 1.5 2 ]" "[1.5,2]"]]]
      (is (= expected (:value (pnix/eval-source src))) src)))
  (testing "float literal grammar matches the Nix lexer"
    (doseq [[src expected]
            [["1.5" 1.5]
             [".5" 0.5]
             ["1." 1.0]
             ["1.e2" 100.0]
             ["1.e3" 1000.0]
             ["1.0e3" 1000.0]
             ["1.25e-3" 0.00125]
             ["2.5e-2" 0.025]
             ["1.5e+2" 150.0]
             ["1.5E2" 150.0]
             [".5e1" 5.0]
             [".5e2" 50.0]
             ["0.0e-400" 0.0]
             ["0.000e9999" 0.0]
             ["2.2250738585072014e-308" Double/MIN_NORMAL]
             ["builtins.typeOf .5" "float"]]]
      (is (= expected (:value (pnix/eval-source src))) src))
    ;; NOT floats in Nix: exponent needs a dotted base; 00 never starts one;
    ;; `{ }.1` lexes `.1` as a float and becomes a non-callable application.
    (doseq [src ["1e3" "00.5" "00.0" "{ }.1"]]
      (let [r (pnix/eval-source src)]
        (is (= :failed (:status r)) src)
        (is (= :call-target-not-callable (:reason r)) src))))
  (testing "non-zero subnormal and overflowing literals are invalid floats"
    ;; Nix's literal reader rejects strtod ERANGE, including representable
    ;; subnormals. This is intentionally stricter than builtins.fromJSON.
    (doseq [src ["1.0e-400" "1.0e-308" "4.9e-324" "1.0e400"]]
      (let [parsed (parser/parse-source src)
            evaluated (pnix/eval-source src)
            machine-result (machine/eval-source src)
            lowered (pnix/lower-source src)
            compiled (pnix/compile-source src)
            receipt (pnix/verify-source src)
            embedded (px-runtime/runtime-source-execution src)]
        (is (= :failed (:status parsed)) src)
        (is (= :invalid-float-literal
               (get-in parsed [:error :data :reason])) src)
        (is (= :failed (:status evaluated)) src)
        (is (= :failed (:status machine-result)) src)
        (is (= :failed (:status lowered)) src)
        (is (= :failed (:status compiled)) src)
        (is (nil? (:lowering-result compiled)) src)
        (is (nil? (:clj-meta-result compiled)) src)
        (is (= :failed (:status receipt)) src)
        (is (nil? (:clj-meta-result receipt)) src)
        (is (= :failed (:status embedded)) src)
        (is (= :syntax-error (:reason embedded)) src)))
    (is (= 1.0E-308
           (:value (pnix/eval-source
                    "builtins.fromJSON \"1.0e-308\"")))
        "JSON numbers keep their distinct oracle semantics"))
  (testing "mixed int/float semantics (oracle parity kept)"
    (doseq [[src expected]
            [["7 / 2" 3]
             ["7.0 / 2" 3.5]
             ["7 / 2.0" 3.5]
             ["builtins.typeOf (1 + 1.0)" "float"]
             ["1 == 1.0" true]
             ["[ 1 ] == [ 1.0 ]" true]
             ["builtins.floor 1.7" 1]
             ["builtins.floor (0.0 - 1.5)" -2]
             ["builtins.ceil (0.0 - 1.5)" -1]
             ["builtins.ceil 9007199254740992" 9007199254740992]
             ["builtins.ceil 9007199254740994" 9007199254740994]
             ["builtins.floor 9223372036854774784.0" 9223372036854774784]
             ["builtins.floor (-9223372036854775808.0)" -9223372036854775808]
             ["builtins.typeOf (builtins.floor 1.7)" "int"]
             ["builtins.typeOf (builtins.fromJSON \"1.0\")" "float"]
             ["builtins.fromJSON \"1e2\"" 100.0]]]
      (is (= expected (:value (pnix/eval-source src))) src))))

(deftest uri-literal-nix-parity
  ;; Nix 2.34.7 lexer.l URI rule. It wins by maximal match over an
  ;; identifier followed by `:`, but remains an expression-only token (not a
  ;; quoted attribute key).
  (testing "URI scheme/body characters and maximal match"
    (doseq [[src expected]
            [["x:x" "x:x"]
             ["let:x" "let:x"]
             ["a1:b" "a1:b"]
             ["a+b:c" "a+b:c"]
             ["a-b:c" "a-b:c"]
             ["a.b:c" "a.b:c"]
             ["a:b:c" "a:b:c"]
             ["a:%/?::@&=+$,-_.!~*'" "a:%/?::@&=+$,-_.!~*'"]
             ["a:b==c" "a:b==c"]
             ["a:b + \"c\"" "a:bc"]]]
      (is (= expected (:value (pnix/eval-source src))) src)))
  (testing "a delimiter or an invalid URI scheme restores lambda syntax"
    (doseq [src ["builtins.typeOf (x: x)"
                 "builtins.typeOf (x:\n x)"
                 "builtins.typeOf (_x:_x)"]]
      (is (= "lambda" (:value (pnix/eval-source src))) src)))
  (testing "URI tokens are not attribute-name tokens"
    (let [result (parser/parse-source "{ x:y = 1; }")]
      (is (= :failed (:status result)))
      (is (= :syntax-error (get-in result [:error :class]))))))

(deftest evaluator-rounding-boundary-parity
  ;; Nix 2.34.7 guards both conversion seams: NixInt -> f64 must be exact,
  ;; then the rounded f64 must fit signed i64. The lowering lane delegates to
  ;; this same evaluator builtin so compiled execution cannot saturate.
  (testing "ceil/floor reject lossy integers and out-of-range floats"
    (doseq [[src reason]
            [["builtins.ceil 9007199254740993"
              :nix-int-to-float-precision-loss]
             ["builtins.floor 9223372036854775808.0"
              :float-to-nix-int-out-of-range]]]
      (let [direct (pnix/eval-source src)
            row (pnix/verify-source src)]
        (is (= :failed (:status direct)) src)
        (is (= reason (:reason direct)) src)
        (is (= :failed (get-in row [:clj-meta-result :status]))
            (str src " compiled lane")))))
  (testing "exact and in-range boundaries agree across all Clojure lanes"
    (doseq [src ["builtins.ceil 9007199254740994"
                 "builtins.floor 9223372036854774784.0"
                 "toString (-0.0)"
                 "toString (-0.0000001)"
                 "toString (-0.0000005)"
                 "toString (0.0 / (-1.0))"
                 "toString ((-1.0) * 0.0)"]]
      (let [row (pnix/verify-source src)
            expected (get-in row [:eval-result :value])]
        (is (= expected (:value (machine/eval-source src))) src)
        (is (= expected (get-in row [:clj-meta-result :value])) src)))))

(deftest evaluator-nonfinite-float-parity
  ;; Oracle: overflow arithmetic may produce infinities/NaN even though the
  ;; canonical value codec rejects them. toString has a safe string result,
  ;; and Nix derives <=/>= by negating reverse strict order. List order only
  ;; advances over equal elements, so distinct NaNs do not expose the tail.
  (let [source (str "let i = 1.0e308 * 1.0e308;"
                    " n1 = i - i; n2 = i - i; in {"
                    " inf = builtins.toString i;"
                    " negInf = builtins.toString (0.0 - i);"
                    " nan = builtins.toString n1;"
                    " eq = n1 == n1; lt = n1 < n1; gt = n1 > n1;"
                    " le = n1 <= n1; ge = n1 >= n1;"
                    " sharedListEq = [ n1 ] == [ n1 ];"
                    " sharedAttrEq = { a = n1; } == { a = n1; };"
                    " distinctAttrEq = { a = i - i; } == { a = i - i; };"
                    " listLt = [ n1 0 ] < [ n2 1 ];"
                    " listLe = [ n1 0 ] <= [ n2 (-1) ]; }")
        expected {"inf" "inf" "negInf" "-inf" "nan" "nan"
                  "eq" false "lt" false "gt" false "le" true "ge" true
                  "sharedListEq" true "sharedAttrEq" true
                  "distinctAttrEq" false
                  "listLt" false "listLe" true}
        row (pnix/verify-source source)]
    (is (= expected (get-in row [:eval-result :value])))
    (is (= expected (:value (machine/eval-source source))))
    (is (= expected (get-in row [:clj-meta-result :value]))))
  (testing "shared functions compare equal only inside containers"
    (let [source (str "let f = x: x; in {"
                      " scalar = f == f;"
                      " listEq = [ f ] == [ f ];"
                      " listLt = [ f 0 ] < [ f 1 ]; }")
          expected {"scalar" false "listEq" true "listLt" true}
          row (pnix/verify-source source)]
      (is (= expected (get-in row [:eval-result :value])))
      (is (= expected (:value (machine/eval-source source))))
      (is (= expected (get-in row [:clj-meta-result :value]))))))

(deftest parser-list-elements-select-only-nix-parity
  ;; D5 corpus-oracle sweep finding (oracle-confirmed): Nix list elements are
  ;; expr_select ONLY — every operator, including unary !/- and `?`, is a
  ;; syntax error in element position; parenthesized forms and `.`-selection
  ;; (with `or` default) remain legal. Our parser used to parse full operator
  ;; expressions per element ([ 1 + 2 ] => [3]); it now rejects exactly where
  ;; real Nix rejects (found via [2 / 0] corpus fixtures, since corrected).
  (testing "operators are rejected in list-element position"
    (doseq [src ["[ 1 + 2 ]"
                 "[ 2 / 0 ]"
                 "[ 1 - 2 ]"
                 "[ 1 == 1 ]"
                 "[ !true ]"
                 "[ -5 ]"
                 "[ 1 ++ [ 2 ] ]"
                 "[ { } ? a ]"]]
      (is (not= :ok (:status (parser/parse-source src))) src)))
  (testing "select-level and parenthesized elements still parse"
    (doseq [[src expected]
            [["[ (1 + 2) ]" [3]]
             ["[ ({ } ? a) ]" [false]]
             ["[ { a = 1; }.a ]" [1]]
             ["[ { a = 1; }.b or 3 ]" [3]]
             ["[ 1 2 3 ]" [1 2 3]]
             ["[ [ 1 ] ]" [[1]]]
             ["{ v = [ { x = 1 + 1; } ]; }" {"v" [{"x" 2}]}]
             ["[ \"a${toString (1 + 1)}\" ]" ["a2"]]]]
      (is (= expected (:value (pnix/eval-source src))) src))
    (is (= 2 (:value (pnix/eval-source
                      "builtins.length [ builtins.length [ ] ]")))
        "juxtaposed elements stay separate (no application in lists)")))

(deftest operator-precedence-nix-parity
  ;; D6 (oracle-gated): expr_op precedence ladder vs nix-instantiate 2.34.7.
  ;; Fixed, all oracle-confirmed: (1) == != and the relationals are two
  ;; separate NONASSOC levels (chains are syntax errors; == binds looser than
  ;; <); (2) `?` is an operator level looser than application/select/unary
  ;; minus with an ATTRPATH RHS, chains legal, and FALSE on a non-attrset
  ;; target (builtins.hasAttr stays strict); (3) `!` binds looser than
  ;; + - * / ++ ? so its operand parses at the add level. // and ++ keep
  ;; their shared level: their Nix placement is observationally equivalent
  ;; here (every distinguishing tree has both sides erroring).
  (testing "== != < > are nonassoc, == looser than <"
    (doseq [src ["1 == 1 == true" "1 != 2 != false" "1 < 2 < 3"]]
      (is (not= :ok (:status (parser/parse-source src))) src))
    (is (= true (:value (pnix/eval-source "true == 1 < 2"))))
    (is (= true (:value (pnix/eval-source "1 < 2 == true")))))
  (testing "? binds looser than application, RHS is an attrpath, non-set is false"
    (doseq [[src expected]
            [["(x: { a = 1; }) { } ? a" true]
             ["builtins.isBool { } ? a" false]
             ["let f = x: x; in f { } ? a" false]
             ["1 ? a" false]
             ["{ } ? a ? b" false]
             ["{ a.b = 1; } ? a.b" true]
             ["{ } ? a.b" false]
             ["{ a = { b = 1; }; } ? a.c" false]
             ["{ a.b = 1; } ? a.b.c" false]
             ["{ a = 1; } ? \"a\"" true]
             ["{ a = 1; } ? a && true" true]]]
      (is (= expected (:value (pnix/eval-source src))) src))
    (is (= :failed (:status (pnix/eval-source "builtins.hasAttr \"a\" 1")))
        "the BUILTIN stays strict on a non-set (real Nix errors too)"))
  (testing "! operand absorbs tighter operators"
    (doseq [[src expected]
            [["! { } ? a" true]
             ["! true == false" true]
             ["! false || true" true]
             ["!!true" true]]]
      (is (= expected (:value (pnix/eval-source src))) src))
    (is (= :failed (:status (pnix/eval-source "1 + !false")))
        "parses like Nix ((1 + (!false))), then held on bool+int like Nix"))
  (testing "unchanged neighborhoods stay put"
    (doseq [[src expected]
            [["- 1 + 2" 1]
             ["2 * -1" -2]
             ["1 + 2 * 3" 7]
             ["{ a = 1; } // { b = 2; } // { c = 3; }" {"a" 1 "b" 2 "c" 3}]
             ["[1] ++ [2] ++ [3]" [1 2 3]]]]
      (is (= expected (:value (pnix/eval-source src))) src))))

(deftest string-nesting-tokenizer-nix-parity
  ;; D7 (oracle-gated): strings nest through ${...} splices — a splice is a
  ;; full expression that may contain strings of BOTH kinds (recursively),
  ;; comments (with braces inside!), and brace pairs. The old regex lexer cut
  ;; the string token at the first bare quote; the tokenizer and the template
  ;; splitters now share one balanced scanner. Real-Nix block comments
  ;; (/* */) also landed (they were missing entirely). All expectations
  ;; oracle-confirmed on nix-instantiate 2.34.7.
  (testing "strings inside splices"
    (doseq [[src expected]
            [["\"${builtins.concatStringsSep \", \" [ \"a\" \"b\" ]}\"" "a, b"]
             ["\"x${\"inner\"}y\"" "xinnery"]
             ["\"a${ { b = \"c\"; }.b }d\"" "acd"]
             ["\"a${\"n${\"e\"}s\"}t\"" "anest"]
             ["''a${\"q\"}b''" "aqb"]
             ["''x${''in''}y''" "xiny"]
             ["\"x${ (''}'') }y\"" "x}y"]
             ["\"id ${let foo' = \"z\"; in foo'}\"" "id z"]]]
      (is (= expected (:value (pnix/eval-source src))) src)))
  (testing "comments inside splices and block comments at top level"
    (doseq [[src expected]
            [["\"a${ /* } */ \"b\" }c\"" "abc"]
             ["\"a${ # } in a comment\n \"b\"}c\"" "abc"]
             ["/* c */ 1" 1]
             ["1 /* mid */ + 2" 3]]]
      (is (= expected (:value (pnix/eval-source src))) src)))
  (testing "string escapes and non-template strings unchanged"
    (doseq [[src expected]
            [["\"a\\${not}b\"" "a${not}b"]
             ["\"esc \\\" quote\"" "esc \" quote"]
             ["''lit ''${ escaped''" "lit ${ escaped"]
             ["{ \"${\"k\"}\" = 1; }.k" 1]]]
      (is (= expected (:value (pnix/eval-source src))) src)))
  (testing "the whole .px runtime corpus now tokenizes (plate file included)"
    ;; parametric-mirror-plate.px held on exactly this frontier since D5
    ;; diagnosed it; the runtime root must stay fully parseable. Parse on a
    ;; dedicated big-stack thread — the 665KB plate file needs the same deep
    ;; stack the evaluator lane uses.
    (let [root (io/file "resources/pnix_clj/pnix_runtime")
          result (promise)
          t (Thread. nil
                     (fn []
                       (deliver result
                                (vec (for [f (file-seq root)
                                           :when (and (.isFile f)
                                                      (str/ends-with? (.getName f) ".px"))]
                                       [(.getName f)
                                        (:status (parser/parse-source (slurp f)))]))))
                     "d7-px-parse" (* 512 1024 1024))]
      (.start t)
      (.join t)
      (let [results @result]
        (is (seq results))
        (doseq [[name status] results]
          (is (= :ok status) name))))))

(deftest weval-ir-pe-dispatch-elimination
  ;; F8 (bounded spike, docs/REMAINING_DECISION.md B): weval-shaped IR-level
  ;; 1st Futamura on the IR-interpreter body — pc-as-context residual blocks,
  ;; memoized so join points are SHARED (the known anti-exponential fix),
  ;; hand-placed boundaries, static residuals only. Honest labels: correctness
  ;; = construction argument + differential (residual vs interpreter vs the
  ;; real evaluator vs the clj-meta bytecode lane); performance = heuristic
  ;; dispatch-count evidence, never a wall-clock claim.
  (let [report (weval/report)]
    (testing "the whole corpus specializes and agrees across all four ways"
      (is (= :ok (:status report)))
      (is (pos? (:supported report)))
      (is (zero? (:failed report)))
      (is (= (:supported report)
             (get-in report [:clj-meta-lane :agreeing]))
          "every residual also runs identically through clj-meta bytecode"))
    (testing "dispatch is eliminated by construction"
      (is (pos? (get-in report [:dispatch :interpreted-steps])))
      (is (zero? (get-in report [:dispatch :residual-steps])))
      (doseq [row (:rows report)
              :when (not= :unsupported (:status row))]
        (is (true? (:dispatch-free? row)) (:source row))))
    (testing "join points are shared, not exponentially unrolled"
      ;; every reachable pc produces exactly ONE block, so block-count can
      ;; never exceed instr-count even with branches/merges in the program
      (doseq [row (:rows report)
              :when (not= :unsupported (:status row))]
        (is (<= (:block-count row) (:instr-count row)) (:source row)))))
  (testing "outside-fragment sources refuse honestly"
    (doseq [source ["x: x + 1"
                    "{ a = 1; }"
                    "[ 1 2 ]"
                    "let f = f; in f"]]
      (is (= :unsupported (:status (weval/compile-to-ir source))) source)))
  (testing "boundary semantics match the evaluator on held cases too"
    ;; division by zero holds on BOTH the interpreter and the residual, and
    ;; the differential harness treats that as agreement with the evaluator
    (let [ir (weval/compile-to-ir "x / y")]
      (is (= :failed (:status (weval/run-ir ir {"x" 1 "y" 0})))))))

(deftest px-lane-hasattr-precedence-parity
  ;; D8: the D6 `?` fixes reached the HOST lane only; the .px runtime parser
  ;; still had `?` at the postfix level, and `(x: { a = 1; }) { } ? a` was the
  ;; one ACTIVE cross-lane value mismatch (:rejected receipt) found by probing
  ;; the D2-D7 edges through run-source. The px parser now parses `?` at its
  ;; operator level with an attrpath RHS desugared exactly like the host
  ;; (e ? a.b == if e ? a then e.a ? b else false), so all four lanes agree.
  (doseq [[source expected]
          [["(x: { a = 1; }) { } ? a" true]
           ["builtins.isBool { } ? a" false]
           ["let f = x: x; in f { } ? a" false]
           ["1 ? a" false]
           ["{ } ? a ? b" false]
           ["{ a = { b = 1; }; } ? a.b" true]
           ["{ } ? a.b" false]
           ["{ a = 1; } ? a && true" true]]]
    (let [receipt (pnix/verify-source source)]
      (is (= :accepted (:status receipt)) source)
      (is (= :all-lanes-agree (:reason receipt)) source)
      (is (= expected (get-in receipt [:eval-result :value])) source)
      (is (= expected (get-in receipt [:px-runtime :value])) source))))

(deftest lowering-lane-d2-strictness-parity
  ;; D9: the D2 strictness fixes reached the host evaluator only; the lowering
  ;; (clj-meta) lane still evaluated the function/init form EAGERLY, so the D2
  ;; empty-collection edges held with :clj-meta-eval-failed instead of
  ;; agreeing. The lowered forms now mirror D2: map/filter/mapAttrs defer the
  ;; fn form into the non-empty arm, and foldl' passes the initial accumulator
  ;; as a lazy slot (operator forces on USE; the final result stays forced).
  (testing "the clj-meta lane agrees with the host on the D2 edges"
    (doseq [[source expected]
            [["builtins.length (builtins.map (throw \"B\") [ ])" 0]
             ["builtins.length (builtins.filter (throw \"B\") [ ])" 0]
             ["builtins.attrNames (builtins.mapAttrs (throw \"B\") { })" []]
             ["builtins.foldl' (a: b: b) (throw \"B\") [ 1 ]" 1]
             ["builtins.foldl' (a: b: a) 0 [ (1 / 0) ]" 0]]]
      (let [receipt (pnix/verify-source source)]
        (is (= expected (get-in receipt [:eval-result :value])) source)
        (is (= :ok (get-in receipt [:clj-meta-result :status])) source)
        (is (= expected (get-in receipt [:clj-meta-result :value])) source))))
  (testing "ordinary folds/maps still collapse across all lanes"
    (doseq [[source expected]
            [["builtins.foldl' (a: b: a + b) 0 [ 1 2 3 ]" 6]
             ["builtins.map (x: x + 1) [ 1 2 3 ]" [2 3 4]]
             ["builtins.filter (x: x > 1) [ 1 2 3 ]" [2 3]]
             ["builtins.mapAttrs (k: v: v + 1) { a = 1; }" {"a" 2}]]]
      (let [receipt (pnix/verify-source source)]
        (is (= :accepted (:status receipt)) source)
        (is (= :all-lanes-agree (:reason receipt)) source)
        (is (= expected (get-in receipt [:eval-result :value])) source))))
  (testing "the strict-fold boundary stays held identically (final force)"
    (let [receipt (pnix/verify-source "builtins.foldl' (a: b: a) (1 / 0) [ ]")]
      (is (not= :ok (get-in receipt [:eval-result :status])))
      (is (not= :ok (get-in receipt [:clj-meta-result :status]))))))

(deftest attrset-binding-merge-nix-parity
  ;; D10 (oracle-gated): real Nix merges attrset bindings AT PARSE TIME
  ;; (addAttr) — static dotted paths nest, and two bindings of one key merge
  ;; iff BOTH values are attrset LITERALS (rec or not); an expression,
  ;; variable, or inherit collision is `attribute already defined`, a parse
  ;; error. We previously SILENTLY OVERWROTE ({ a.b = 1; a = { c = 2; }; }
  ;; lost b — a wrong-value class) or held too strictly on the reverse order.
  ;; The parser now merges statically; dynamic keys/segments pass through to
  ;; the evaluator unchanged. Bonus: the lowering lane's dotted-binding
  ;; frontier (:unsupported-lowering-attr-key) closed for free.
  (testing "literal merges (all oracle-confirmed)"
    (doseq [[src expected]
            [["{ a.b = 1; a.c = 2; }" {"a" {"b" 1 "c" 2}}]
             ["{ a.b = 1; a = { c = 2; }; }" {"a" {"b" 1 "c" 2}}]
             ["{ a = { c = 2; }; a.b = 1; }" {"a" {"b" 1 "c" 2}}]
             ["{ a = { b = 1; }; a = { c = 2; }; }" {"a" {"b" 1 "c" 2}}]
             ["{ a.b.c = 1; a.b = { d = 2; }; }" {"a" {"b" {"c" 1 "d" 2}}}]
             ["{ a.b.c = 1; a.b.d = 2; }" {"a" {"b" {"c" 1 "d" 2}}}]
             ["{ a = rec { b = 1; }; a.c = 2; }" {"a" {"b" 1 "c" 2}}]]]
      (is (= expected (:value (pnix/eval-source src))) src)))
  (testing "rec scoping survives the merge"
    (is (= 9 (:value (pnix/eval-source
                      "rec { a = { b = 1; }; a.c = z; z = 9; }.a.c"))))
    (is (= 5 (:value (pnix/eval-source "rec { a.b = x; x = 5; }.a.b")))))
  (testing "non-literal collisions are parse errors like real Nix"
    (doseq [src ["{ a.b = 1; a.b = 2; }"
                 "{ a = { b = 1; }; a.b = 2; }"
                 "{ a = builtins.seq 1 { c = 2; }; a.b = 1; }"
                 "let v = { c = 2; }; in { a = v; a.b = 1; }"]]
      (is (not= :ok (:status (parser/parse-source src))) src)))
  (testing "dynamic keys still pass through"
    (is (= {"k" 1 "k2" 2}
           (:value (pnix/eval-source "{ \"${\"k\"}\" = 1; k2 = 2; }")))))
  (testing "the clj-meta lane now lowers dotted bindings"
    (doseq [[src expected]
            [["{ a.b = 1; }" {"a" {"b" 1}}]
             ["{ a.b = 1; a.c = 2; }.a.c" 2]]]
      (let [receipt (pnix/verify-source src)]
        (is (= expected (get-in receipt [:eval-result :value])) src)
        (is (= :ok (get-in receipt [:clj-meta-result :status])) src)))))

(deftest px-lane-eq-rel-nonassoc-parity
  ;; D11 (px backlog, mirrors D6): the px runtime parser had one left-assoc
  ;; comparison level; it now splits == != and < > <= >= into two NONASSOC
  ;; levels like real Nix, so `true == 1 < 2` upgrades from px-held to
  ;; all-lanes agreement and chains stay held at parse like the host.
  (doseq [[source expected]
          [["true == 1 < 2" true]
           ["1 < 2 == true" true]
           ["1 == 1 && 2 < 3" true]]]
    (let [receipt (pnix/verify-source source)]
      (is (= :accepted (:status receipt)) source)
      (is (= :all-lanes-agree (:reason receipt)) source)
      (is (= expected (get-in receipt [:px-runtime :value])) source)))
  (doseq [source ["1 == 1 == true" "1 < 2 < 3"]]
    (is (not= :ok (:status (parser/parse-source source))) source)))

(deftest px-lane-float-grammar-parity
  ;; D12 (px backlog, mirrors D4): the px lexer only knew digit.digit floats;
  ;; it now carries the Nix float grammar (dot-leading `.5`, trailing-dot
  ;; `1.`, exponents `2.5e-2`/`1.5E+2` only after a dot, sign only right
  ;; after e/E) and parse_number normalizes for fromJSON (`1.` -> 1.0).
  (doseq [[source expected]
          [[".5" 0.5]
           ["1." 1.0]
           ["2.5e-2" 0.025]
           ["1.5E+2" 150.0]
           [".5e1" 5.0]
           ["1.5 + .5" 2.0]
           ["[ .5 1. ]" [0.5 1.0]]
           ["42" 42]
           ["7 / 2" 3]]]
    (let [receipt (pnix/verify-source source)]
      (is (= :accepted (:status receipt)) source)
      (is (= :all-lanes-agree (:reason receipt)) source)
      (is (= expected (get-in receipt [:px-runtime :value])) source))))

(deftest lowering-bare-builtins-parity
  ;; D13 (filed during D12): the lowering lane left BARE default-scope
  ;; builtin names as unresolved vars — bare `toString` failed to eval and
  ;; bare `map` leaked clojure.core/map (held by the receipt value-mismatch
  ;; guard). builtin-select-name now treats an UNSHADOWED bare builtin var
  ;; like its builtins.X select (all call special-cases benefit), and the
  ;; :var value position resolves through builtins-attrset. `let` shadows;
  ;; `with` does NOT (oracle + host: the static base scope wins).
  (testing "bare builtins lower and agree with the host"
    (doseq [[source expected]
            [["toString 42" "42"]
             ["toString 1.5" "1.500000"]
             ["map (x: x + 1) [ 1 2 ]" [2 3]]
             ["isNull null" true]
             ["baseNameOf \"/a/b\"" "b"]
             ["dirOf \"/a/b\"" "/a"]
             ["removeAttrs { a = 1; b = 2; } [ \"a\" ]" {"b" 2}]
             ["with { map = 1; }; map (x: x) [ 2 ]" [2]]]]
      (let [receipt (pnix/verify-source source)]
        (is (= expected (get-in receipt [:eval-result :value])) source)
        (is (= :ok (get-in receipt [:clj-meta-result :status])) source)
        (is (= expected (get-in receipt [:clj-meta-result :value])) source))))
  (testing "let-shadowing wins over the bare builtin on every lane"
    (doseq [[source expected]
            [["let map = x: y: 7; in map (x: x) [ 2 ]" 7]
             ["let toString = x: \"X\"; in toString 5" "X"]]]
      (let [receipt (pnix/verify-source source)]
        (is (= :accepted (:status receipt)) source)
        (is (= expected (get-in receipt [:eval-result :value])) source))))
  (testing "D14: the px lane binds the same bare set, so receipts fully accept"
    (doseq [[source expected]
            [["toString 1.5" "1.500000"]
             ["map (x: x + 1) [ 1 2 ]" [2 3]]
             ["isNull null" true]
             ["removeAttrs { a = 1; b = 2; } [ \"a\" ]" {"b" 2}]
             ["with { map = 1; }; map (x: x) [ 2 ]" [2]]]]
      (let [receipt (pnix/verify-source source)]
        (is (= :accepted (:status receipt)) source)
        (is (= :all-lanes-agree (:reason receipt)) source)
        (is (= expected (get-in receipt [:px-runtime :value])) source)))))

(deftest px-lane-d2-strictness-parity
  ;; D15 (px backlog, completes D2 across all three executing lanes): the px
  ;; evaluator's Apply tail forced EVERY native argument, so the D2-lazy
  ;; positions were over-strict there. lazy-native wrappers now receive the
  ;; function/init argument unforced and force/marshal internally in the Nix
  ;; order: map/filter/mapAttrs/sort skip the fn on an empty collection, and
  ;; foldl' takes op strictly but the initial accumulator lazily with
  ;; per-iteration + final forcing (the strict fold).
  (doseq [[source expected]
          [["builtins.length (builtins.map (throw \"B\") [ ])" 0]
           ["builtins.length (builtins.filter (throw \"B\") [ ])" 0]
           ["builtins.attrNames (builtins.mapAttrs (throw \"B\") { })" []]
           ["builtins.length (builtins.sort (throw \"B\") [ ])" 0]
           ["builtins.foldl' (a: b: b) (throw \"B\") [ 1 ]" 1]
           ["builtins.foldl' (a: b: a) 0 [ (1 / 0) ]" 0]
           ["builtins.foldl' (a: b: a + b) 0 [ 1 2 3 ]" 6]
           ["builtins.map (x: x + 1) [ 1 2 3 ]" [2 3 4]]
           ["builtins.filter (x: x > 1) [ 1 2 3 ]" [2 3]]
           ["builtins.sort (a: b: a < b) [ 3 1 2 ]" [1 2 3]]
           ["builtins.mapAttrs (k: v: v + 1) { a = 1; }" {"a" 2}]]]
    (let [receipt (pnix/verify-source source)]
      (is (= :accepted (:status receipt)) source)
      (is (= :all-lanes-agree (:reason receipt)) source)
      (is (= expected (get-in receipt [:px-runtime :value])) source))))

(deftest px-lane-dotted-binding-parity
  ;; D16 (px backlog, mirrors D10): the px parser only knew single-key attrset
  ;; bindings; a static dotted path now nests and MERGES with sibling literals
  ;; exactly like Nix's parse-time addAttr (attr_path_nest + push_assign /
  ;; merge_attr_values), while non-literal collisions are parse errors and
  ;; dynamic keys keep single-key behavior. The px evaluator is untouched —
  ;; only canonical single-key entries reach it, the D10 architecture.
  (doseq [[source expected]
          [["{ a.b = 1; }" {"a" {"b" 1}}]
           ["{ a.b = 1; a.c = 2; }.a.c" 2]
           ["{ a.b = 1; a = { c = 2; }; }.a.b" 1]
           ["{ a.b.c = 1; a.b.d = 2; }.a.b.d" 2]
           ["rec { a.b = x; x = 5; }.a.b" 5]
           ["{ \"k.q\" = 1; }.\"k.q\"" 1]]]
    (let [receipt (pnix/verify-source source)]
      (is (= :accepted (:status receipt)) source)
      (is (= :all-lanes-agree (:reason receipt)) source)
      (is (= expected (get-in receipt [:px-runtime :value])) source))))

(deftest px-lane-string-nesting-parity
  ;; D17 (the LAST px backlog item, mirrors D7): the px string lexer ended the
  ;; token at the first unescaped quote, so a string inside a ${...} splice
  ;; broke it. The splice is now copied VERBATIM (one substring element --
  ;; UTF-8 safe, escapes left raw) using find_matching_brace as the shared
  ;; balanced scanner, which also learned comment skipping; `\$` survives
  ;; lexing so split_interp can tell it from a real splice; and the px expr
  ;; lexer gained real-Nix /* */ block comments (the host got them in D7).
  (doseq [[source expected]
          [["\"x${\"inner\"}y\"" "xinnery"]
           ["\"a${ { b = \"c\"; }.b }d\"" "acd"]
           ["\"a${\"n${\"e\"}s\"}t\"" "anest"]
           ["\"a${ /* } */ \"b\" }c\"" "abc"]
           ["\"a\\${not}b\"" "a${not}b"]
           ["/* c */ 1" 1]
           ["1 /* mid */ + 2" 3]
           ["\"esc \\\" quote\"" "esc \" quote"]
           ["\"한글 ${toString 1} 유지\"" "한글 1 유지"]]]
    (let [receipt (pnix/verify-source source)]
      (is (= :accepted (:status receipt)) source)
      (is (= :all-lanes-agree (:reason receipt)) source)
      (is (= expected (get-in receipt [:px-runtime :value])) source))))

(deftest strict-semantics-default-across-lanes
  ;; R2 Phase D (owner doctrine 2026-07-07): pnix and Clojure are two
  ;; LANGUAGES — truthy if/assert/! and string+non-string coercion were
  ;; Clojure host leaks into the guest, never a dialect. Strict Nix typing is
  ;; now pnix's ONLY semantics, on every executing lane: the evaluator default
  ;; flipped, the lowering lane gained require-bool guards and a both-strings
  ;; plus, and the px lane already held. Flip measurement: 278 corpus+runtime
  ;; sources, strict violations 0 — nothing real broke.
  (testing "type errors hold on every lane"
    (doseq [[source reason]
            [["if 5 then 1 else 2" :non-bool-if-condition]
             ["assert 5; 42" :non-bool-assert-condition]
             ["!5" :non-bool-not-operand]
             ["\"a\" + 1" :string-coercion]]]
      (let [receipt (pnix/verify-source source)]
        (is (= reason (get-in receipt [:eval-result :reason])) source)
        (is (not= :ok (get-in receipt [:clj-meta-result :status])) source)
        (is (not= :ok (get-in receipt [:px-runtime :status])) source))))
  (testing "well-typed forms stay fully accepted"
    (doseq [source ["if true then 1 else 2" "assert true; 42" "!false"
                    "\"a\" + \"b\"" "1 + 2"]]
      (let [receipt (pnix/verify-source source)]
        (is (= :accepted (:status receipt)) source)
        (is (= :all-lanes-agree (:reason receipt)) source)))))

(deftest logical-operand-bool-nix-parity
  ;; D18 (oracle-gated, nix-instantiate 2.34.7): && || -> require BOOLEAN
  ;; operands in real Nix ("value is an integer while a Boolean was
  ;; expected") — the pre-D18 arms leaked Clojure truthiness on all three
  ;; lanes (host `if`, lowered bare and/or, px `l == false then .. else rhs`),
  ;; the exact R2 Phase D host-leak class, missed because only if/assert/!/+
  ;; were audited. Each operand is type-checked when (and only when) it is
  ;; evaluated: the left always (BEFORE the right exists — 1 && throw is the
  ;; type error, not the throw), the right only past the short-circuit, so
  ;; false && (throw "X") stays false. Type errors stay UNCATCHABLE through
  ;; tryEval while a caught throw on the right stays catchable (D3 taxonomy).
  ;; Flip measurement before landing: strict gate 278 classified / 269 ok /
  ;; 0 failed — the px runtime bootstrap (hundreds of && uses) survives.
  (testing "non-bool operands hold with the operator's reason, all lanes"
    (doseq [[source reason]
            [["1 && true" :non-bool-and-operand]
             ["true && 1" :non-bool-and-operand]
             ["1 && (throw \"X\")" :non-bool-and-operand]
             ["true && true && 1" :non-bool-and-operand]
             ["false || 2" :non-bool-or-operand]
             ["1 || true" :non-bool-or-operand]
             ["1 -> true" :non-bool-implies-operand]
             ["true -> 2" :non-bool-implies-operand]]]
      (let [receipt (pnix/verify-source source)]
        (is (= reason (get-in receipt [:eval-result :reason])) source)
        (is (not= :ok (get-in receipt [:clj-meta-result :status])) source)
        (is (not= :ok (get-in receipt [:px-runtime :status])) source))))
  (testing "short-circuits stay lazy (the unevaluated side is never checked)"
    (doseq [[source expected]
            [["false && (throw \"X\")" false]
             ["true || (throw \"X\")" true]
             ["false -> (throw \"X\")" true]
             ["true && true" true]
             ["false || true" true]
             ["true -> false" false]]]
      (let [receipt (pnix/verify-source source)]
        (is (= :accepted (:status receipt)) source)
        (is (= :all-lanes-agree (:reason receipt)) source)
        (is (= expected (get-in receipt [:eval-result :value])) source))))
  (testing "tryEval boundary: the type error is uncatchable, a throw is not"
    (is (= :failed (:status (pnix/eval-source
                           "(builtins.tryEval (1 && true)).success"))))
    (is (= false (:value (pnix/eval-source
                          "(builtins.tryEval (true && (throw \"X\"))).success"))))))

(deftest pattern-lambda-nix-parity
  ;; D19 (oracle-gated, nix-instantiate 2.34.7, found while deriving M7d):
  ;; real Nix pattern-lambda semantics — pre-D19 ALL THREE lanes bound
  ;; defaults EAGERLY in a SEQUENTIAL scope and accepted extra keys:
  ;; (1) defaults are LAZY thunks in a KNOT-TIED recursive scope: an unused
  ;;     default (even `throw`) never evaluates, a default can reference ANY
  ;;     formal (later ones included), a default cycle is infinite recursion;
  ;; (2) REQUIRED formals are checked at application time in pattern order,
  ;;     BEFORE the extra-key check ('({ a, b }: a) { a=1; c=1; }' reports b);
  ;; (3) without `...` an extra argument key is an error (the parser always
  ;;     captured :ellipsis? — the evaluators ignored it);
  ;; (4) the argument must be an attrset; (5) @as binds the ACTUAL argument
  ;;     only (defaults are not in it). All these errors are UNCATCHABLE
  ;;     through tryEval, like real Nix type errors.
  (testing "oracle matrix agrees on all three lanes"
    (doseq [[source expected]
            [["({ a ? throw \"x\" }: 1) { }" 1]
             ["({ a ? 1 }: a) { }" 1]
             ["({ a, ... }: a) { a = 1; b = 2; }" 1]
             ["({ a ? b, b ? 2 }: a) { }" 2]
             ["({ b ? 2, a ? b }: a) { }" 2]
             ["({ a ? throw \"x\", b ? 1 }: b) { }" 1]
             ["({ a ? 5 }@args: args.a or \"absent\") { }" "absent"]
             ["({ a ? \"d\" }: a) { a = 7; }" 7]
             ["({ a, b ? a + 1 }: b) { a = 10; }" 11]]]
      (let [receipt (pnix/verify-source source)]
        (is (= :accepted (:status receipt)) source)
        (is (= :all-lanes-agree (:reason receipt)) source)
        (is (= expected (get-in receipt [:eval-result :value])) source))))
  (testing "application-time errors hold with the oracle-confirmed reasons"
    (doseq [[source reason]
            [["({ a }: a) { a = 1; b = 2; }" :unexpected-lambda-pattern-arg]
             ["({ a }: a) 1" :lambda-pattern-arg-not-attrset]
             ["({ a ? b, b ? a }: a) { }" :infinite-recursion]
             ["({ a }: 1) { }" :missing-lambda-pattern-arg]
             ["({ a }: a) { b = 1; }" :missing-lambda-pattern-arg]
             ["({ a, b }: a) { a = 1; c = 1; }" :missing-lambda-pattern-arg]]]
      (let [receipt (pnix/verify-source source)]
        (is (= reason (get-in receipt [:eval-result :reason])) source)
        (is (not= :ok (get-in receipt [:clj-meta-result :status])) source)
        (is (not= :ok (get-in receipt [:px-runtime :status])) source))))
  (testing "pattern application errors stay uncatchable through tryEval"
    (doseq [source ["(builtins.tryEval (({ a }: a) { a = 1; b = 2; })).success"
                    "(builtins.tryEval (({ a }: a) 1)).success"]]
      (is (= :failed (:status (pnix/eval-source source))) source))))

(deftest dynamic-attr-key-nix-parity
  ;; D20 (oracle-gated, found while preparing M7e): dynamic attr keys —
  ;; pre-D20 an eval-time key collision SILENTLY OVERWROTE (wrong value:
  ;; { a = 1; "${"a"}" = 2; }.a was 2; real Nix: "dynamic attribute 'a'
  ;; already defined") and a non-string dynamic key was coerced via str
  ;; ({ }.${1} or "d" was "d"; real Nix type-errors, and `or` does NOT
  ;; catch it). Laziness preserved: keys evaluate at CONSTRUCTION, so an
  ;; unforced attrset never raises (let s = { a = 1; ${"a"} = 2; }; in 1).
  (testing "collisions and non-string keys hold with oracle reasons"
    (doseq [[source reason]
            [["{ a = 1; \"${\"a\"}\" = 2; }.a" :duplicate-attr]
             ["{ \"${\"a\"}\" = 1; \"${\"a\"}\" = 2; }.a" :duplicate-attr]
             ;; D22: a BARE literal interpolation key folds to a static key
             ;; at parse, so this collision is now a PARSE error like real
             ;; Nix's "attribute 'a' already defined" (quoted template keys
             ;; above stay dynamic = eval-time :duplicate-attr).
             ["{ ${\"a\"} = 1; a = 2; }.a" :unsupported-syntax]
             ["{ }.${1} or \"d\"" :dynamic-attr-key-not-string]]]
      (let [r (pnix/eval-source source)]
        (is (= :failed (:status r)) source)
        (is (= reason (:reason r)) source))))
  (testing "well-typed dynamic keys still work, lazily, on all lanes"
    (doseq [[source expected]
            [["let k = \"x\"; in { \"${k}\" = 5; }.x" 5]
             ["{ ${\"x\"} = 1; ${\"y\"} = 2; }.y" 2]
             ;; D22 CORRECTED PIN: the old row here used a literal ${"a"}
             ;; key and asserted ok/1 WITHOUT oracle confirmation -- real
             ;; Nix parse-folds it and errors 'attribute already defined'
             ;; even unforced (the D5/D19 unoracled-pin class, caught while
             ;; probing D22). Construction-laziness is now pinned with a
             ;; genuinely DYNAMIC key, which real Nix leaves unevaluated:
             ["let s = { a = 1; ${builtins.substring 0 1 \"ab\"} = 2; }; in 1" 1]]]
      (let [receipt (pnix/verify-source source)]
        (is (= :accepted (:status receipt)) source)
        (is (= :all-lanes-agree (:reason receipt)) source)
        (is (= expected (get-in receipt [:eval-result :value])) source))))
  (testing "D21 (oracle-gated): dynamic-segment paths desugar at PARSE to
            nested lazy literals — a.${k} = v IS a = { ${k} = v; }, exactly
            Nix's addAttr: a dynamic sub-key evaluates only when its PARENT
            forces, literal↔path siblings merge in both orders, and the
            static prefixes merge like any literal. The pre-D21 evaluator was
            construction-eager and collision-strict (too-strict helds)."
    (doseq [[source expected]
            [["{ a.${\"b\"} = 3; }.a.b" 3]
             ["{ ${\"a\"}.b = 3; }.a.b" 3]
             ["{ a.${\"b\"} = 1; a.c = 2; }.a.c" 2]
             ["{ a.${\"b\"} = 1; a.c = 2; }.a.b" 1]
             ["{ a = { c = 1; }; a.${\"b\"} = 2; }.a.b" 2]
             ["{ a.${\"b\"} = 2; a = { c = 1; }; }.a.c" 1]
             ["builtins.attrNames { a.${1} = 1; }" ["a"]] ; sub-key UNTOUCHED
             ["let s = { a.${1} = 1; }; in 1" 1]
             ["{ a.${\"b\"}.c = 4; }.a.b.c" 4]
             ["rec { x = 7; a.${\"k\"} = x; }.a.k" 7]]]
      (let [r (pnix/eval-source source)]
        (is (= :ok (:status r)) source)
        (is (= expected (:value r)) source)))
    ;; D22: ${"b"} parse-folds to a static segment, so this duplicate is a
    ;; PARSE error now (oracle: "attribute 'a.b' already defined" -- a parse
    ;; position); a genuinely dynamic duplicate stays the D20 eval-time hold.
    (is (= :unsupported-syntax
           (:reason (pnix/eval-source
                     "{ a.${\"b\"} = 1; a.${\"b\"} = 2; }.a.b"))))
    (is (= :duplicate-attr
           (:reason (pnix/eval-source
                     (str "{ a.${builtins.substring 0 1 \"bc\"} = 1; "
                          "a.${builtins.substring 0 1 \"bc\"} = 2; }.a.b")))))
    ;; the desugar closed the LOWERING frontier for these shapes (no more
    ;; :unsupported-lowering-attr-key — the dynamic-key machinery lowers
    ;; them); the px lane is now the honest blocker, filed as px backlog.
    (let [receipt (pnix/verify-source "{ a.${\"b\"} = 3; }.a.b")]
      (is (= 3 (get-in receipt [:eval-result :value])))
      (is (= 3 (get-in receipt [:clj-meta-result :value]))
          "lowering lane agrees post-desugar")
      (is (not= :ok (get-in receipt [:px-runtime :status]))
          "px dynamic-path support = filed backlog"))
    (is (= 2 (:value (pnix/eval-source "rec { a = 2; \"${\"b\"}\" = a; }.b"))
        ) "rec DIRECT dynamic key unchanged (its lowering frontier remains)")))

(deftest dotted-let-nix-parity
  ;; D22 (oracle-gated): let bindings are the SAME binds production as
  ;; attrsets in real Nix. Pre-D22 the parser only took simple names
  ;; (dotted lets held :unsupported-syntax -- the filed D22 frontier) and
  ;; silently let a DUPLICATE name shadow (oracle-wrong: real Nix parse-
  ;; errors). Two parser moves close it: (1) bare `${"literal"}` attrpath
  ;; segments FOLD to static keys at parse (real Nix does -- both collision
  ;; orders are parse errors even unforced; `let ${"a"} = 1; in a` is 1;
  ;; quoted template keys stay dynamic/eval-checked); (2) let bindings run
  ;; through the SAME parse-time addAttr merge as attrsets (path->nested +
  ;; merge-attr-bindings), so dotted paths nest/merge, dynamic SUB-segments
  ;; keep D21's nested-lazy semantics, and a TOP key still dynamic after the
  ;; fold is real Nix's 'dynamic attributes not allowed in let'.
  (testing "oracle matrix, all three lanes where the row is well-typed"
    (doseq [[source expected]
            [["let a.b = 1; in a.b" 1]
             ["let a.b = 1; a.c = 2; in a.c" 2]
             ["let a = { c = 1; }; a.b = 2; in a.b" 2]
             ["let a.b = 2; a = { c = 1; }; in a.c" 1]
             ["let ${\"a\"} = 1; in a" 1]
             ["let b = a; ${\"a\"} = 1; in b" 1]
             ["let a.${\"b\"} = 1; in a.b" 1]
             ["let a.b = c; c = 5; in a.b" 5]
             ["let a.b.c = 1; a.b.d = 2; in a.b.d" 2]
             ["let inherit ({x=3;}) x; a.y = x; in a.y" 3]]]
      (let [r (pnix/eval-source source)]
        (is (= :ok (:status r)) source)
        (is (= expected (:value r)) source)))
    (is (= {"b" 1} (:value (pnix/eval-source "let a.b = 1; in a")))))
  (testing "parse-time errors exactly where real Nix errors"
    (doseq [source ["let a.b = 1; a.b = 2; in a.b"   ; dup leaf
                    "let x = 1; x = 2; in x"          ; dup name (was silent!)
                    "let ${\"a\" + \"x\"} = 1; in ax" ; genuinely dynamic name
                    "let ${1} = 1; in 2"]]
      (is (= :unsupported-syntax (:reason (pnix/eval-source source)))
          source)))
  (testing "dynamic SUB-segments in let paths keep D21 lazy semantics"
    (is (= 1 (:value (pnix/eval-source
                      "let a.${builtins.substring 0 1 \"bc\"} = 1; in a.b")))))
  (testing "the machine follows with zero changes (shared parser + AST)"
    (let [comparable (fn [r] (if (= :ok (:status r))
                               [:ok (:value r)]
                               [(:status r) (:reason r)]))]
      (doseq [source ["let a.b = 1; in a.b" "let ${\"a\"} = 1; in a"
                      "let a.b = c; c = 5; in a.b" "let x = 1; x = 2; in x"
                      "let a.${builtins.substring 0 1 \"bc\"} = 1; in a.b"]]
        (is (= (comparable (pnix/eval-source source))
               (comparable (machine/eval-source source)))
            source)))))

(deftest machine-derivation-agrees-with-evaluator
  ;; M7 — the abstract machine (pnix-clj.machine) is DERIVED from eval-ast* by
  ;; the functional correspondence (closure conversion + CPS +
  ;; defunctionalization; Ager/Biernacki/Danvy/Midtgaard PPDP'03), call-by-need
  ;; via the Krivine + memoizing-store refinement. The correspondence
  ;; transforms CONTROL only — value semantics are the evaluator's own shared
  ;; public fns — so on the machine's fragment the two lanes must agree
  ;; EXACTLY, ok and held alike. Fragment growth M7b–M7f: attrsets (rec,
  ;; inherit, dynamic keys with the D20 checks), select/has-attr (or-default
  ;; via the :unwind delimited catch; dynamic segments), assert, with,
  ;; templates, builtins under the default env (apply-callable delegation +
  ;; [:try-eval]), pattern lambdas native (D19), paths and import (shared
  ;; resolver seam). The corpus lives in machine/differential-corpus so the
  ;; :machine report artifact regression-pins the same list.
  (let [comparable (fn [r] (if (= :ok (:status r))
                             [:ok (:value r)]
                             [(:status r) (:reason r)]))]
    (testing "the SHARED differential corpus (machine/differential-corpus —
              one list serves this pin and the :machine report artifact)"
      (doseq [source machine/differential-corpus]
        (is (= (comparable (pnix/eval-source source))
               (comparable (machine/eval-source source)))
            source)))
    (testing "import resolves through the SHARED resolver seam"
      (let [resolver (fn [_ctx _target _scope] {:status :ok :value 42})]
        (binding [evaluator/*import-resolver* resolver]
          (is (= 42 (:value (machine/run-ast
                             (:ast (parser/parse-source "import ./m"))))))
          (is (= 42 (:value (evaluator/eval-ast
                             (:ast (parser/parse-source "import ./m")))))))))
    (testing "M7i: dynamic-segment path bindings run in the machine
              (mirroring the HOST bug-for-bug — D21 filed: real Nix defers
              dynamic sub-keys and merges literal↔path; our lanes are eager
              and strict, an oracle-confirmed follow-up on the evaluator)"
      (doseq [source ["{ a.${\"b\"} = 3; }.a.b"
                      "{ ${\"a\"}.b = 3; }.a.b"
                      "{ a.${\"b\"}.c = 4; }.a.b.c"
                      "rec { x = 7; a.${\"k\"} = x; }.a.k"
                      "{ a.${\"b\"} = 1 / 0; c = 2; }.c"
                      "{ a.${\"b\"} = 1; a.${\"b\"} = 2; }.a.b"
                      "{ a.${1} = 1; }"
                      "let p = { a.${\"q\"} = 9; }; in 1"]]
        (is (= (comparable (pnix/eval-source source))
               (comparable (machine/eval-source source)))
            source)))))

(deftest machine-constant-stack-beyond-treewalk
  ;; M7 depth witness — the D1c shapes (oracle-filed: nested-list cliff between
  ;; 40k ok and 100k SOE even on the 2GB deep stack). Raising the evaluator
  ;; lane from 64MB to 2GB also lets the tree-walk handle the shallower-frame
  ;; plus-chain at 100k; the heap-frame machine still handles BOTH shapes and,
  ;; on a tiny 256KB thread, finishes where the tree-walk overflows. That is
  ;; the constant-JVM-stack witness.
  (let [on-stack (fn [kb f]
                   (let [p (promise)
                         t (Thread. nil
                                    (fn [] (deliver p (try {:r (f)}
                                                           (catch Throwable t
                                                             {:threw (class t)}))))
                                    "machine-stack-witness"
                                    (long (* kb 1024)))]
                     (.start t)
                     (.join t)
                     @p))]
    (testing "plus-chain-100k: both deep-stack lanes now finish"
      (let [source (str "1" (apply str (repeat 100000 " + 1")))
            mr (machine/eval-source source)
            er (pnix/eval-source source)]
        (is (= 100001 (:value mr)))
        (is (= 100001 (:value er)))))
    (testing "nested-list-100k: machine evaluates and realizes iteratively"
      (let [source (str (apply str (repeat 100000 "[ ")) "1"
                        (apply str (repeat 100000 " ]")))
            mr (machine/eval-source source)
            ;; verify depth ITERATIVELY — clojure.core/= would itself recurse
            depth (loop [v (:value mr) d 0]
                    (if (vector? v) (recur (first v) (inc d)) d))]
        (is (= :ok (:status mr)))
        (is (= 100000 depth))))
    (testing "256KB thread: same AST, machine finishes, tree-walk overflows"
      (let [parsed (parser/parse-source (str "1" (apply str (repeat 30000 " + 1"))))
            ast (:ast parsed)
            m (on-stack 256 #(machine/run-ast ast))
            e (on-stack 256 #(evaluator/eval-ast ast))]
        (is (= :ok (:status parsed)))
        (is (= 30001 (get-in m [:r :value])))
        (is (or (:threw e) (not= :ok (:status (:r e))))
            "the recursive tree-walk cannot run this on a small stack")))
    (testing "256KB thread, v2 shapes: nested attrsets realize iteratively"
      ;; Depth 3k keeps the gate fast: the parser is measurably superlinear on
      ;; nested attrset literals (20k ≈ 52s parse, machine run+realize 0.2s —
      ;; filed as a parser observation, NOT a machine cost). 3k is far beyond
      ;; what a recursive force-deep can realize on a 256KB stack. The parse
      ;; itself is recursive descent, so it runs on a BIG stack (the machine
      ;; claim is about run+realize only).
      (let [src (str (apply str (repeat 3000 "{ a = ")) "1"
                     (apply str (repeat 3000 "; }")))
            parsed (:r (on-stack (* 2 1024 1024) #(parser/parse-source src)))
            ast (:ast parsed)
            m (on-stack 256 #(machine/run-ast ast))
            e (on-stack 256 #(evaluator/eval-ast ast))
            depth (loop [v (get-in m [:r :value]) d 0]
                    (if (map? v) (recur (get v "a") (inc d)) d))]
        (is (= :ok (:status parsed)))
        (is (= :ok (get-in m [:r :status])))
        (is (= 3000 depth))
        (is (or (:threw e) (not= :ok (:status (:r e))))
            "recursive realize cannot walk this on a small stack")))))

(deftest machine-report-capability
  ;; M7g — the :machine report artifact: the shared differential corpus plus
  ;; the 256KB constant-stack witness, rendered as a regression-pinned report
  ;; (the M-pillar promotion from :smoke pins to a first-class capability).
  ;; Fuel parity rides along: the machine ticks the SAME *fuel* volatile with
  ;; the same tagged throw, so safe-eval-style budgets bound it identically.
  (let [r (machine/report)]
    (is (= :ok (:status r)))
    (is (= :machine-report (:kind r)))
    (is (= (count machine/differential-corpus) (:row-count r)))
    (is (empty? (:divergent r)))
    (is (true? (get-in r [:constant-stack-witness :ok?]))))
  (testing "fuel bounds the machine loop like the evaluator's budget"
    (binding [evaluator/*fuel* (volatile! 3)]
      (is (thrown-with-msg? clojure.lang.ExceptionInfo #"fuel exhausted"
                            (machine/run-whnf
                             (:ast (parser/parse-source
                                    "let a=1; b=a; c=b; d=c; e=d; f=e; g=f; in g"))
                             {}))))))

(deftest guest-surface-extension-registry-drift
  ;; Host-leak audit (owner doctrine 2026-07-07, "two languages, no
  ;; blending"): the guest builtins surface was diffed against real Nix
  ;; 2.34.7 (builtins.attrNames builtins). Every extra name is a guest
  ;; EXTENSION -- it accepts MORE programs but never changes the meaning of
  ;; a valid Nix program -- and the set is PINNED here via
  ;; resources/pnix_clj/guest_surface.edn, so adding a builtin extension is
  ;; always a conscious, reviewed act (registry update), never drift.
  (let [registry (edn/read-string
                  (slurp (io/resource "pnix_clj/guest_surface.edn")))
        documented (set (:extensions registry))
        ours (set (keys (get evaluator/default-env "builtins")))]
    (is (= :pnix-clj.guest-surface.v0 (:schema registry)))
    (is (seq documented))
    (testing "every documented extension actually exists"
      (doseq [x documented]
        (is (contains? ours x) x)))
    (testing "no undocumented extension crept in (drift gate)"
      ;; compute from the captured real-Nix list stored alongside the
      ;; registry: ours - (real ∪ documented) must be empty
      (let [real (set (:captured-real-nix registry))
            unknown (remove #(or (contains? real %) (contains? documented %))
                            ours)]
        (is (empty? unknown) (pr-str (vec unknown)))))))

(deftest parser-rejects-expr-keywords-as-operator-operands
  ;; host-parser-let-if-rhs (found by the property fuzzer, oracle-confirmed):
  ;; real Nix rejects an unparenthesized let/if/with/assert as an OPERAND of
  ;; an operator; only the leftmost position may start one (it swallows the
  ;; rest). The host parser now matches Nix -- and the .px parser, which
  ;; always rejected these, agrees cross-lane.
  (testing "operand-after-operator positions reject expr-level keywords"
    (doseq [src ["1 + let x = 1; in x"
                 "1 + if true then 1 else 2"
                 "1 + with {}; 2"
                 "1 + assert true; 2"
                 "true -> if true then true else false"
                 "- let x = 1; in x"
                 "!if true then false else true"]]
      (is (not= :ok (:status (parser/parse-source src))) src)))
  (testing "function-argument and list-element positions reject them too"
    ;; same bug class, oracle-confirmed: Nix restricts these positions to
    ;; expr_select, so an unparenthesized keyword is a syntax error there.
    (doseq [src ["[ let x = 1; in x ]"
                 "[ if true then 1 else 2 ]"
                 "(x: x) let a = 1; in a"
                 "(x: x) if true then 1 else 2"]]
      (is (not= :ok (:status (parser/parse-source src))) src)))
  (testing "leftmost/parenthesized/body positions still parse (Nix tree shape)"
    (doseq [[src expected]
            [["let x = 1; in x + 1" 2]
             ["if true then 1 else 2 + 3" 1]
             ["1 + (let x = 1; in x)" 2]
             ["(1 + (if true then 1 else 2))" 2]
             ["assert true; 1 + 1" 2]
             ["with { a = 1; }; a + 1" 2]
             ["-5 + 3" -2]
             ["!true || false" false]
             ["[ (let x = 1; in x) ]" [1]]
             ["map (x: x + 1) [ 1 2 3 ]" [2 3 4]]
             ["(x: x) (if true then 1 else 2)" 1]
             ["{ a = let x = 1; in x; }" {"a" 1}]]]
      (is (= expected (:value (pnix/eval-source src))) src))))

(deftest parser-let-if-function-call-parser
  (testing "let"
    (let [ast (parser/parse-source "let a = 1; b = a; in b")]
      (is (= :let (:op (:ast ast))))))
  (testing "if"
    (let [ast (parser/parse-source "if true then 1 else 2")]
      (is (= :if (:op (:ast ast))))))
  (testing "function"
    (let [ast (parser/parse-source "x: x + 1")]
      (is (= :lambda (:op (:ast ast))))
      (is (= "x" (get-in ast [:ast :param])))))
  (testing "call"
    (let [ast1 (parser/parse-source "f 1")
          ast2 (parser/parse-source "builtins.map (x: x) [1 2 3]")]
      (is (= :call (:op (:ast ast1))))
      (is (= :call (:op (:ast ast2)))))))

(deftest parser-string-context-syntax
  (testing "normal string interpolation"
    (let [ast (parser/parse-source "\"a${builtins.toString 7}b\"")]
      (is (= :string-template (:op (:ast ast))))
      (is (= 3 (count (:parts (:ast ast)))))
      (is (= :call (-> ast :ast :parts (nth 1) :expr :op)))))
  (testing "indented string interpolation"
    (let [ast (parser/parse-source "let n = 3; in ''prefix-${builtins.toString n}''")]
      (is (= :let (:op (:ast ast))))
      (is (= :string-template (-> ast :ast :body :op))))))

(deftest clj-meta-host-reflection-snapshot-api
  (testing "var/namespace/class/metadata snapshots"
    (let [var-result (host-reflection/snapshot :var 'clojure.core/str)
          ns-result (host-reflection/snapshot :namespace 'clojure.core)
          class-result (host-reflection/snapshot :class "java.lang.String")
          metadata-result (host-reflection/snapshot :metadata "x")]
      (is (= :ok (:status var-result)))
      (is (= "Var" (get-in var-result [:snapshot "tag"])))
      (is (= :ok (:status ns-result)))
      (is (= "Namespace" (get-in ns-result [:snapshot "tag"])))
      (is (= :ok (:status class-result)))
      (is (= "JavaClass" (get-in class-result [:snapshot "tag"])))
      (is (= :ok (:status metadata-result)))))
  (testing "macroexpand snapshot"
    (let [macro-result (host-reflection/snapshot :macroexpand '(when true 1))]
      (is (= :ok (:status macro-result)))
      (is (map? (:snapshot macro-result)))
      (is (int? (get-in macro-result [:snapshot :step-count])))))
  (testing "classloader snapshot and unknown kind"
    (let [classloader-result (host-reflection/snapshot :classloader nil)
          unknown-result (host-reflection/snapshot :nope "x")]
      (is (= :ok (:status classloader-result)))
      (is (= :failed (:status unknown-result)))
      (is (= :unknown-snapshot-kind (:reason unknown-result)))))
)

(deftest run-source-closes-mirror-spine-for-small-source
  (let [receipt (pnix/verify-source "42")]
    (is (= :accepted (:status receipt)))
    (is (= :all-lanes-agree (:reason receipt)))
    (is (= {:status :ok :value 42} (:eval-result receipt)))
    (is (= :clojure-mirror (get-in receipt [:clojure-mirror :kind])))
    (is (= :ok (get-in receipt [:clojure-mirror
                                :clj-meta-determinism :status])))
    (is (= true (get-in receipt [:clojure-mirror
                                 :clj-meta-determinism
                                 :same-class-name?])))
    (is (= false (get-in receipt [:clojure-mirror
                                  :clj-meta-fallback :fallback?])))
    (is (= :ok (get-in receipt [:clojure-mirror
                                :clj-meta-strict :status])))
    (is (= true (get-in receipt [:clojure-mirror
                                 :clj-meta-strict
                                 :same-value-as-primary?])))
    (is (= 64 (count (:bytecode-hash receipt))))
    (is (= (:bytecode-hash receipt)
           (get-in receipt [:clojure-mirror :bytecode-hash])))
    (is (= :ok (get-in receipt [:clojure-mirror
                                :clj-meta-verified :status])))
    (is (= true (get-in receipt [:clojure-mirror
                                 :clj-meta-verified
                                 :verification
                                 :ok])))
    (is (= :ok (get-in receipt [:clj-meta-result :capability :status])))
    (is (= :host-compile
           (get-in receipt [:clj-meta-result :interop :effect-class])))
    (is (interop/witness? (get-in receipt [:clj-meta-result :witness])))
    (is (= :pnix-clj.mirror/run-mirror
           (get-in receipt [:mirror-run :kind])))
    (is (= mirror/default-facets
           (get-in receipt [:mirror-run :facets])))
    (is (= (:clojure-mirror receipt)
           (get-in receipt [:mirror-run :clojure-mirror])))
    (is (= :px-runtime (get-in receipt [:px-runtime :kind])))
    (is (= :ok (get-in receipt [:px-runtime :status])))
    (is (= 42 (get-in receipt [:px-runtime :value])))
    (is (some? (get-in receipt [:px-runtime :artifact :hash])))
    (is (some? (:px-runtime-hash receipt)))
    (is (= (:px-runtime receipt)
           (get-in receipt [:mirror-run :px-runtime])))
    (is (= :pnix-mirror (get-in receipt [:pnix-mirror :kind])))
    (is (= :ok (get-in receipt [:pnix-mirror :status])))
    (is (= :pnix-mirror-from-runtime-receipt
           (get-in receipt [:pnix-mirror :reason])))
    (is (= "pnixc-pnix.eval.run-mirror.v0"
           (get-in receipt [:pnix-mirror :runtime-receipt "mirror_schema"])))
    (is (= "ok" (get-in receipt [:pnix-mirror :runtime-receipt "status"])))
    (is (= "Int" (get-in receipt [:pnix-mirror :runtime-receipt "ast_tag"])))
    (is (= 42 (get-in receipt [:pnix-mirror :value])))
    (is (= (:pnix-mirror receipt)
           (get-in receipt [:mirror-run :pnix-mirror])))
    (is (= :cross-mirror-verdict
           (get-in receipt [:cross-mirror-verdict :kind])))
    (is (= :ok (get-in receipt [:cross-mirror-verdict :status])))
    (is (= :agree (get-in receipt [:cross-mirror-verdict :equivalence])))
    (is (= true (get-in receipt [:cross-mirror-verdict :host-mirror-agrees?])))
    (is (= :mirrors-agree
           (get-in receipt [:cross-mirror-verdict :reason])))
    (is (= (:cross-mirror-verdict receipt)
           (get-in receipt [:mirror-run :cross-mirror-verdict])))
    (is (= :ok
           (get-in receipt [:mirror-run :facet-statuses :inner/px-runtime])))
    (is (= :ok
           (get-in receipt [:mirror-run :facet-statuses :cross/value-agreement])))
    (is (= [:pnix-clj-evaluator
            :pnix-clj-lowering-clj-meta
            :clojure-stage15-mirror
            :px-runtime-pnix-mirror]
           (mapv :lane (:lane-summary receipt))))
    (is (= :ok (get-in receipt [:lane-summary 2 :status])))
    (is (= :held (get-in receipt [:lane-summary 2 :stage15-control-status])))
    (is (= :px-runtime-pnix-mirror
           (get-in receipt [:lane-summary 3 :lane])))
    (is (= :ok (get-in receipt [:lane-summary 3 :status])))))

(deftest builtin-dispatch-closes-attr-list-and-arithmetic-slice
  (doseq [[source expected]
          [["builtins.removeAttrs { a = 1; b = 2; c = 3; } [ \"b\" ]"
            {"a" 1 "c" 3}]
           ["builtins.concatLists [ [1 2] [3] [] ]"
            [1 2 3]]
           ["builtins.concatStrings [\"a\" \"b\" \"c\"]"
            "abc"]
           ["builtins.concatMap (x: [ x (x + 10) ]) [ 1 2 ]"
            [1 11 2 12]]
           ["builtins.concatMapStrings (x: builtins.toString x) [ 1 2 3 ]"
            "123"]
           ["builtins.concatMapStringsSep \", \" (x: builtins.toString x) [ 1 2 3 ]"
            "1, 2, 3"]
           ["builtins.append [ 1 2 ] [ 3 4 ]"
            [1 2 3 4]]
           ["builtins.take 2 [ 1 2 3 ]"
            [1 2]]
           ["builtins.drop 1 [ 1 2 3 ]"
            [2 3]]
           ["builtins.reverseList [ 1 2 3 ]"
            [3 2 1]]
           ["builtins.last [ 1 2 3 ]"
            3]
           ["builtins.init [ 1 2 3 ]"
            [1 2]]
           ["builtins.unique [ 1 2 2 3 1 3 ]"
            [1 2 3]]
           ["builtins.replicate 3 \"x\""
            ["x" "x" "x"]]
           ["builtins.get { x = 1; } \"x\""
            1]
           ["builtins.set { x = 1; } \"y\" 2"
            {"x" 1 "y" 2}]
           ["builtins.keys { b = 2; a = 1; }"
            ["a" "b"]]
           ["builtins.values { b = 2; a = 1; }"
            [1 2]]
           ["builtins.attrValues { b = 2; a = 1; }"
            [1 2]]
           ["builtins.merge { a = 1; } { b = 2; }"
            {"a" 1 "b" 2}]
           ["builtins.find 2 [ 1 2 3 ]"
            2]
           ["builtins.zip [ 1 2 ] [ \"a\" \"b\" \"c\" ]"
            [[1 "a"] [2 "b"]]]
           ["builtins.length (builtins.zip [ (1 / 0) ] [ 2 ])"
            1]
           ["builtins.flatten [ 1 [ 2 [ 3 ] ] ]"
            [1 2 3]]
           ["builtins.catAttrs \"x\" [ { x = 1; } { y = 2; } { x = 3; } ]"
            [1 3]]
           ["builtins.attrByPath [\"a\" \"b\"] 99 { a = { b = 7; }; }"
            7]
           ["builtins.attrByPath [\"a\" \"x\"] 99 { a = { b = 7; }; }"
            99]
           ["builtins.mapAttrs (name: value: if name == \"a\" then value + 10 else value + 20) { a = 1; b = 2; }"
            {"a" 11 "b" 22}]
           ["builtins.mapAttrsToList (k: v: k) { b = 2; a = 1; }"
            ["a" "b"]]
           ["builtins.mapAttrsToList (k: v: v) { b = 2; a = 1; }"
            [1 2]]
           ["builtins.mapAttrs' (k: v: { name = k + \"!\"; value = v + 1; }) { a = 1; b = 2; }"
            {"a!" 2 "b!" 3}]
           ["builtins.mapAttrs' (k: v: { name = \"same\"; value = v; }) { a = 1; b = 2; }"
            {"same" 1}]
           ["builtins.filterAttrs (name: value: name == \"keep\" && value > 0) { keep = 1; drop = 0; }"
            {"keep" 1}]
           ["builtins.intersectAttrs { a = 1; b = 2; } { b = 9; c = 3; }"
            {"b" 9}]
           ["builtins.groupBy (x: if x < 3 then \"small\" else \"big\") [ 1 2 3 4 ]"
            {"small" [1 2] "big" [3 4]}]
           ["builtins.partition (x: x < 3) [ 1 2 3 4 ]"
            {"right" [1 2] "wrong" [3 4]}]
           ["builtins.genAttrs [ \"a\" \"b\" ] (n: n + \"x\")"
            {"a" "ax" "b" "bx"}]
           ["builtins.nameValuePair \"k\" 42"
            {"name" "k" "value" 42}]
           ["builtins.foldlAttrs (acc: k: v: acc + v) 0 { a = 1; b = 2; c = 3; }"
            6]
           ["builtins.addErrorContext \"ctx\" 99"
            99]
           ["builtins.functionArgs ({ x, y ? 1, ... }: x + y)"
            {"x" false "y" true}]
           ["builtins.functionArgs builtins.map"
            {}]
           ["builtins.isString \"x\""
            true]
           ["builtins.isAttrs { a = 1; }"
            true]
           ["builtins.isAttrs (x: x)"
            false]
           ["builtins.isList [ 1 2 ]"
            true]
           ["builtins.isFunction (x: x)"
            true]
           ["builtins.isFloat 1.5"
            true]
           ["builtins.isBool false"
            true]
           ["builtins.isNull null"
            true]
           ["builtins.isInt 42"
            true]
           ["builtins.typeOf (x: x)"
            "lambda"]
           ["builtins.typeOf { a = 1; }"
            "set"]
           ["builtins.currentSystem"
            version/current-system]
           ["builtins.nixVersion"
            version/nix-version]
           ["builtins.storeDir"
            version/store-dir]
           ["builtins.baseNameOf \"/a/b/c.txt\""
            "c.txt"]
           ["builtins.dirOf \"/a/b/c.txt\""
            "/a/b"]
           ["builtins.toLower \"AbC\""
            "abc"]
           ["builtins.toUpper \"AbC\""
            "ABC"]
           ["builtins.stringToCharacters \"abc\""
            ["a" "b" "c"]]
           ["builtins.splitString \"/\" \"a/b/c\""
            ["a" "b" "c"]]
           ["builtins.splitString \"/\" \"/a/\""
            ["" "a" ""]]
           ["builtins.match \"a.*\" \"abc\""
            []]
           ["builtins.match \"(a)(b)\" \"ab\""
            ["a" "b"]]
           ["builtins.match \"z\" \"abc\""
            nil]
           ["builtins.split \"a\" \"xayaz\""
            ["x" [] "y" [] "z"]]
           ["builtins.split \"(a)b\" \"abc\""
            ["" ["a"] "c"]]
           ["builtins.range 2 5"
            [2 3 4 5]]
           ["builtins.hasInfix \"bc\" \"abcd\""
            true]
           ["builtins.compareVersions \"1.2.3\" \"1.10.0\""
            -1]
           ["builtins.compareVersions \"2.0\" \"2.0\""
            0]
           ["builtins.compareVersions \"2.0\" \"1.9\""
            1]
           ["builtins.bitAnd 12 10"
            8]
           ["builtins.bitOr 12 10"
            14]
           ["builtins.bitXor 12 10"
            6]
           ["builtins.splitVersion \"1.2-rc1\""
            ["1" "2" "rc" "1"]]
           ["builtins.parseDrvName \"hello-1.2.3\""
            {"name" "hello" "version" "1.2.3"}]
           ["builtins.parseDrvName \"a-1-b-2\""
            {"name" "a" "version" "1-b-2"}]
           ["builtins.parseDrvName \"-1\""
            {"name" "" "version" "1"}]
           ["builtins.toPath \"/a/../b\""
            "/b"]
           ["builtins.break 42"
            42]
           ["[ builtins.true builtins.false builtins.null ]"
            [true false nil]]
           ["builtins.langVersion"
            6]
           ["builtins.builtins ? parseDrvName"
            true]
           ["builtins.builtins ? true"
            true]
           ["builtins.fromJSON \"{\\\"a\\\":1,\\\"b\\\":[2,3]}\""
            {"a" 1 "b" [2 3]}]
           ["builtins.listToAttrs [ { name = \"a\"; value = 1; } { name = \"a\"; value = 2; } { name = \"b\"; value = 3; } ]"
            {"a" 1 "b" 3}]
           ["builtins.cons 1 [ 2 3 ]"
            [1 2 3]]
           ["builtins.tryEval (1 + 2)"
            {"success" true "value" 3}]
           ["builtins.tryEval (builtins.throw \"boom\")"
            {"success" false "value" false}]
           ["builtins.seq 1 2"
            2]
           ["builtins.deepSeq { a = [ 1 2 3 ]; } \"done\""
            "done"]
           ["builtins.trace \"hello\" 42"
            42]
           ["assert (1 < 2); 42"
            42]
           ["builtins.genList (i: i + 10) 3"
            [10 11 12]]
           ["builtins.all (x: x > 0) [ 1 2 3 ]"
            true]
           ["builtins.any (x: x > 2) [ 1 2 3 ]"
            true]
           ["builtins.foldl (acc: x: acc - x) 100 [ 1 2 3 ]"
            94]
           ["builtins.count (x: x > 2) [ 1 2 3 4 5 ]"
            3]
           ["builtins.zipListsWith (a: b: a + b) [ 1 2 3 ] [ 10 20 30 40 ]"
            [11 22 33]]
           ["builtins.imap0 (i: x: [ i x ]) [ \"a\" \"b\" ]"
            [[0 "a"] [1 "b"]]]
           ["builtins.imap1 (i: x: [ i x ]) [ \"a\" \"b\" ]"
            [[1 "a"] [2 "b"]]]
           ["builtins.findFirst (x: x > 2) 99 [ 1 2 3 4 ]"
            3]
           ["builtins.findFirst (x: x > 9) 99 [ 1 2 3 ]"
            99]
           ["builtins.foldr (x: acc: x - acc) 0 [ 1 2 3 ]"
            2]
           [(str "builtins.genericClosure { startSet = [{ key = 1; }]; "
                 "operator = item: if item.key < 4 then [{ key = item.key + 1; }] else []; }")
            [{"key" 1} {"key" 2} {"key" 3} {"key" 4}]]
           ["builtins.genericClosure { startSet = [{ key = 1; } { key = 1; }]; operator = item: []; }"
            [{"key" 1}]]
           ["builtins.genericClosure { startSet = []; operator = item: []; }"
            []]
           ["builtins.pipe 1 [ (x: x + 1) (x: x * 10) (x: x - 5) ]"
            15]
           ["builtins.recursiveUpdate { a = { x = 1; y = 2; }; } { a = { y = 9; z = 3; }; }"
            {"a" {"x" 1 "y" 9 "z" 3}}]
           ["builtins.zipAttrsWith (name: values: builtins.concatStringsSep \":\" [ name (builtins.toString (builtins.length values)) ]) [ { a = 1; b = 2; } { a = 3; c = 4; } ]"
            {"a" "a:2" "b" "b:1" "c" "c:1"}]
           ["builtins.and true true"
            true]
           ["builtins.or false true"
            true]
           ["builtins.not false"
            true]
           ["builtins.boolToString true"
            "true"]
           ["builtins.boolToString false"
            "false"]
           ["builtins.id 42"
            42]
           ["builtins.toInt \"  42 \""
            42]
           ["builtins.flip (a: b: a - b) 3 10"
            7]
           ["builtins.eq \"ab\" (\"a\" + \"b\")"
            true]
           ["builtins.lt 1 2"
            true]
           ["builtins.le 2 2"
            true]
           ["builtins.gt 3 2"
            true]
           ["builtins.ge 3 3"
            true]
           ["builtins.mod 17 5"
            2]
           ["builtins.neg 3"
            -3]
           ["builtins.abs (-4)"
            4]
           ["builtins.pow 2 5"
            32]
           ["builtins.sqrt 25"
            5.0]
           ["builtins.floor 3.9"
            3]
           ["builtins.ceil 3.1"
            4]
           ["builtins.exp 0"
            1.0]
           ["builtins.ln 1"
            0.0]
           ["builtins.sin 0"
            0.0]
           ["builtins.cos 0"
            1.0]
           ["builtins.atan2 0 1"
            0.0]
           ["builtins.add 2 3"
            5]
           ["builtins.sub 10 3"
            7]
           ["builtins.mul 4 5"
            20]
           ["builtins.div 20 4"
            5]
           ["builtins.lessThan 1 2"
            true]
           ["builtins.hasPrefix \"ab\" \"abcdef\""
            true]
           ["builtins.hasPrefix \"xy\" \"abcdef\""
            false]
           ["builtins.hasSuffix \"ef\" \"abcdef\""
            true]
           ["builtins.hasSuffix \"ab\" \"abcdef\""
            false]
           ["builtins.optional true 5"
            [5]]
           ["builtins.optional false 5"
            []]
           ["builtins.optionals true [1 2]"
            [1 2]]
           ["builtins.optionals false [1 2]"
            []]
           ["builtins.min 7 3"
            3]
           ["builtins.max 3 7"
            7]
           ["builtins.optionalString true \"x\""
            "x"]
           ["builtins.optionalString false \"x\""
            ""]
           ["builtins.removePrefix \"ab\" \"abcd\""
            "cd"]
           ["builtins.removePrefix \"xy\" \"abcd\""
            "abcd"]
           ["builtins.removeSuffix \"cd\" \"abcd\""
            "ab"]
           ["builtins.removeSuffix \"xy\" \"abcd\""
            "abcd"]]]
    (let [receipt (pnix/verify-source source)]
      (is (= :accepted (:status receipt)) source)
      (is (= :all-lanes-agree (:reason receipt)) source)
      (is (= expected (get-in receipt [:eval-result :value])) source)
      (is (= expected (get-in receipt [:clj-meta-result :value])) source)
      (is (= expected (get-in receipt [:px-runtime :value])) source)
      (is (= expected (get-in receipt [:pnix-mirror :value])) source))))

(deftest evaluator-integer-overflow-is-structured
  (doseq [[source operator phase]
          [["9223372036854775807 + 1" "+" :eval]
           ["builtins.add 9223372036854775807 1" "+" :eval]
           ["9223372036854775807 - (-1)" "-" :eval]
           ["builtins.sub 9223372036854775807 (-1)" "-" :eval]
           ["3037000500 * 3037000500" "*" :eval]
           ["builtins.mul 3037000500 3037000500" "*" :eval]
           ["let x = -9223372036854775807 - 1; in -x" "-" :eval]
           ["let x = -9223372036854775807 - 1; in x / -1" "/" :eval]
           ["builtins.div (-9223372036854775807 - 1) (-1)" "/" :eval]]]
    (let [r (pnix/eval-source source)
          row (pnix/verify-source source)]
      (is (= :failed (:status r)) source)
      (is (= :integer-overflow (:reason r)) source)
      (is (= phase (get-in r [:error :phase])) source)
      (is (= operator (get-in r [:error :evidence :operator])) source)
      (is (= :failed (get-in row [:clj-meta-result :status]))
          (str source " compiled lane"))
      (is (= :integer-overflow
             (get-in row [:clj-meta-result :error :class]))
          (str source " compiled error class"))))
  (testing "integer overflow remains uncatchable through tryEval"
    (let [source "builtins.tryEval (9223372036854775807 + 1)"
          row (pnix/verify-source source)
          machine-result (machine/eval-source source)]
      (is (= :integer-overflow (get-in row [:eval-result :reason])))
      (is (= :integer-overflow (:reason machine-result)))
      (is (= :integer-overflow
             (get-in row [:clj-meta-result :error :class])))
      (is (= :failed (get-in row [:px-runtime :status])))))
  (testing "mixed int/float arithmetic stays numeric instead of int-overflow checked"
    (doseq [[source expected]
            [["1 + 2.5" 3.5]
             ["builtins.add 1 1.5" 2.5]
             ["builtins.sub 4.5 1" 3.5]
             ["builtins.mul 2 1.5" 3.0]
             ["builtins.div 7.0 2" 3.5]
             ["builtins.lessThan 1 1.5" true]]]
      (let [row (pnix/verify-source source)]
        (is (= expected (get-in row [:eval-result :value])) source)
        (is (= expected (:value (machine/eval-source source))) source)
        (is (= expected (get-in row [:clj-meta-result :value])) source)))))

(deftest compile-source-executes-without-proof-admission
  (let [compiled (pnix/compile-source "42")
        held (pnix/compile-source "@")]
    (is (= :pnix-clj.compile-source (:kind compiled)))
    (is (= :pnix-clj.compile-source.v0 (:schema compiled)))
    (is (= :ok (:status compiled)))
    (is (= :pnix-source-host-execution-ready (:reason compiled)))
    (is (= 42 (:lowered-form compiled)))
    (is (= false (get-in compiled [:lowering-result :source-string-codegen?])))
    (is (= :ok (get-in compiled [:clj-meta-result :status])))
    (is (= 42 (get-in compiled [:clj-meta-result :value])))
    (is (= 'pnix.clj-meta.compiler/eval-form
           (get-in compiled [:clj-meta-result :execution-api])))
    ;; Basic host execution is not governed by repeat compilation, bytecode
    ;; proof, mirror agreement, or admission receipts. Those belong to the
    ;; explicit verification lane.
    (is (nil? (:compile-receipt compiled)))
    (is (nil? (:bytecode-hash compiled)))
    (is (nil? (get-in compiled [:clj-meta-result :api-values-agree?])))
    (is (= :pnix-clj.compile-source (:kind held)))
    (is (= :failed (:status held)))
    (is (= :unsupported-syntax (:reason held)))
    (is (nil? (:bytecode-hash held)))
    (is (nil? (:compile-receipt held)))))

(deftest deterministic-classfile-report-pins-asm-and-generated-classes
  (let [report (classfile-receipt/report)
        rows-by-kind (into {} (map (juxt :class-kind identity)
                                   (:generated-class-rows report)))]
    (is (= :pnix-deterministic-classfile-report (:kind report)))
    (is (= :pnix-clj.deterministic-classfile-report.v0 (:schema report)))
    (is (= :pin-asm-and-enumerate-generated-classfiles (:policy report)))
    (is (= :ok (:status report)))
    (is (= 5 (:row-count report)))
    (is (= #{:deftype :defrecord :reify :proxy}
           (set (:generated-class-kinds report))))
    (is (every? #(= "9.7.1" (:mvn/version %))
                (get-in report [:dependency-pins :asm-util])))
    (is (= :ok (get-in report [:pnix-compile-row
                               :summary
                               :bytecode-status])))
    (is (= true (get-in rows-by-kind [:proxy :summary :verified-ok?])))
    (is (pos? (get-in rows-by-kind
                      [:defrecord :summary :bytecode-class-count])))
    (is (= 64 (count (:receipt-hash report))))))

(deftest common-mode-risk-report-records-independent-oracle-mitigation
  (let [report (trust/report)
        mitigation-kinds (set (map :kind (:mitigations report)))]
    (is (= :pnix-common-mode-risk-report (:kind report)))
    (is (= :pnix-clj.common-mode-risk-report.v0 (:schema report)))
    (is (= :ok (:status report)))
    (is (= :correlated-common-mode-failure (:risk report)))
    (is (contains? mitigation-kinds :independent-live-oracle))
    (is (contains? mitigation-kinds :generated-differential-gate))
    (is (contains? (set (:shared-tcb report)) :pnix-parser))
    (is (= 64 (count (:report-hash report))))))

(deftest translation-validation-report-frames-receipts-as-validators
  (let [report (translation-validation/report)
        validators (set (map :id (:validators report)))
        sample-statuses (into {} (map (juxt :validator :status)
                                      (:sample-rows report)))]
    (is (= :pnix-translation-validation-report (:kind report)))
    (is (= :pnix-clj.translation-validation-report.v0 (:schema report)))
    (is (= :validate-source-candidate-default-held (:policy report)))
    (is (= :ok (:status report)))
    (is (= :translation-validators-framed (:reason report)))
    (is (contains? validators :parse-source))
    (is (contains? validators :cross-mirror))
    (is (contains? validators :external-live-oracle))
    (is (every? #(contains? % :default-on-uncertainty)
                (:validators report)))
    (is (= :ok (:parse-source sample-statuses)))
    (is (= :ok (:cross-mirror sample-statuses)))
    (is (= 64 (count (:receipt-hash report))))))

(deftest emit-form-roundtrip-report-checks-analyzer-emitted-values
  (let [report (emit-form-roundtrip/report)
        row-by-id (into {} (map (juxt :id identity) (:rows report)))]
    (is (= :pnix-emit-form-roundtrip-report (:kind report)))
    (is (= :pnix-clj.emit-form-roundtrip-report.v0 (:schema report)))
    (is (= :analyzer-emit-form-value-roundtrip (:policy report)))
    (is (= :ok (:status report)))
    (is (= 6 (:case-count report)))
    (is (= 6 (:ok report)))
    (is (= 0 (:held-or-rejected report)))
    (is (= '(let* [x 20] (clojure.lang.Numbers/add x 22))
           (get-in row-by-id [:let-arithmetic :emitted-form])))
    (is (= 64 (count (:receipt-hash report))))))

(deftest value-roundtrip-report-synthesizes-stable-clojure-forms
  (let [report (value-roundtrip/report)
        row-by-id (into {} (map (juxt :id identity) (:rows report)))]
    (is (= :pnix-value-roundtrip-report (:kind report)))
    (is (= :pnix-clj.value-roundtrip-report.v0 (:schema report)))
    (is (= :pnix-value-to-clojure-form-synthesis (:policy report)))
    (is (= :ok (:status report)))
    (is (= 8 (:case-count report)))
    (is (= 8 (:ok report)))
    (is (= 0 (:held-or-rejected report)))
    (is (every? :forward-value-equal? (:rows report)))
    (is (every? :synthesized-value-equal? (:rows report)))
    (is (every? :closure-form-equal? (:rows report)))
    (is (= '(array-map "a" 1 "b" [true nil])
           (get-in row-by-id [:attrset-literal :synthesized-form])))
    (is (= [1 2]
           (get-in row-by-id [:builtin-attr-values :synthesized-value])))
    (is (= 64 (count (:receipt-hash report))))))

(deftest benchmark-reports-stable-receipt-baseline
  (let [result (benchmark/run-benchmark {:sources ["42"]
                                         :iterations 1
                                         :run-iterations 1})
        lane-ids (set (map :id (:lanes result)))]
    (is (= :pnix-clj-benchmark (:kind result)))
    (is (= :pnix-clj.benchmark.v0 (:schema result)))
    (is (= :ok (:status result)))
    (is (= :semantic-receipts-stable (:reason result)))
    (is (= 1 (:source-count result)))
    (is (= {:total 1
            :accepted 1
            :rejected 0
            :held 0
            :first-frontier nil
            :first-rejected nil}
           (:preflight result)))
    (is (contains? lane-ids :parse-source-cold))
    (is (contains? lane-ids :parse-source-warm))
    (is (contains? lane-ids :lower-ast-cold))
    (is (contains? lane-ids :lower-ast-warm))
    (is (contains? lane-ids :full-report))
    (is (pos? (get-in result [:parse-cache :entries])))
    (is (pos? (get-in result [:lower-cache :entries])))))

(deftest report-summarizes-accepted-first-slice
  (let [summary (pnix/report ["42" "true" "\"x\""])]
    (is (= 3 (:total summary)))
    (is (= 3 (:accepted summary)))
    (is (= 0 (:rejected summary)))
    (is (= 0 (:held summary)))
    (is (= 3 (get (:reason-counts summary) :all-lanes-agree)))
    (is (nil? (:first-frontier summary)))))

(deftest mirror-pair-report-tracks-basic-runtime-fixtures
  (let [report (report-artifact/report-for :mirror-pair)
        rows (:mirror-pair-rows report)
        row-by-id (into {} (map (juxt :source-id identity) rows))]
    (is (= :mirror-pair-report (:kind report)))
    (is (= :mirror-pair-fixture-set (:fixture-kind report)))
    (is (= 199 (:fixture-count report)))
    (is (= 199 (:accepted report)))
    (is (= 0 (:rejected report)))
    (is (= 0 (:held report)))
    (is (= 199 (:mirror-pair-ready-count report)))
    (is (= 0 (:mirror-pair-not-ready-count report)))
    (is (nil? (:first-frontier report)))
    (is (every? :ready? rows))
    (is (every? #(= :agree (get-in % [:cross-mirror :equivalence]))
                rows))
    (is (every? #(= :ok (get-in % [:clojure-mirror
                                   :determinism-status]))
                rows))
    (is (every? #(= true (get-in % [:clojure-mirror
                                    :same-class-name?]))
                rows))
    (is (every? #(= false (get-in % [:clojure-mirror :fallback?]))
                rows))
    (is (every? #(= :ok (get-in % [:px-runtime :status]))
                rows))
    (is (every? #(= "ok" (get-in % [:px-runtime :runtime-mirror-status]))
                rows))
    (is (every? #(= "pnixc-pnix.eval.run-mirror.v0"
                    (get-in % [:pnix-mirror :runtime-mirror-schema]))
                rows))
    (is (every? #(= "pnixc-pnix/eval/evaluator.px"
                    (get-in % [:pnix-mirror :runtime-mirror-owner]))
                rows))
    (is (every? #(= "pnix-mirror-runtime"
                    (get-in % [:px-runtime :artifact :root]))
                rows))
    (is (every? #(= "vm.px"
                    (get-in % [:px-runtime :artifact :relative-path]))
                rows))
    (is (every? #(= (get-in % [:value-hashes :evaluator])
                    (get-in % [:value-hashes :clj-meta])
                    (get-in % [:value-hashes :px-runtime])
                    (get-in % [:value-hashes :pnix-mirror]))
                rows))
    (is (= :agree (get-in row-by-id
                          [:mirror-pair/path-literal
                           :cross-mirror :equivalence])))
    (is (= :agree (get-in row-by-id
                          [:mirror-pair/absolute-path-literal
                           :cross-mirror :equivalence])))
    (is (= :agree (get-in row-by-id
                          [:mirror-pair/path-is-path
                           :cross-mirror :equivalence])))
    (is (= :agree (get-in row-by-id
                          [:mirror-pair/absolute-path-is-path
                           :cross-mirror :equivalence])))
    (is (= :agree (get-in row-by-id
                          [:mirror-pair/path-typeof
                           :cross-mirror :equivalence])))
    (is (= :agree (get-in row-by-id
                          [:mirror-pair/path-is-not-attrs
                           :cross-mirror :equivalence])))
    (is (= :agree (get-in row-by-id
                          [:mirror-pair/path-equality
                           :cross-mirror :equivalence])))
    (is (= :agree (get-in row-by-id
                          [:mirror-pair/path-to-string-pure
                           :cross-mirror :equivalence])))
    (is (= :agree (get-in row-by-id
                          [:mirror-pair/absolute-path-to-string-pure
                           :cross-mirror :equivalence])))
    (is (= :agree (get-in row-by-id
                          [:mirror-pair/path-base-name
                           :cross-mirror :equivalence])))
    (is (= :agree (get-in row-by-id
                          [:mirror-pair/path-dir-of
                           :cross-mirror :equivalence])))
    (is (= :agree (get-in row-by-id
                          [:mirror-pair/path-dir-of-type
                           :cross-mirror :equivalence])))
    (is (= :agree (get-in row-by-id
                          [:mirror-pair/interpolation-out-path
                           :cross-mirror :equivalence])))
    (is (= :agree (get-in row-by-id
                          [:mirror-pair/interpolation-to-string
                           :cross-mirror :equivalence])))
    (is (= :agree (get-in row-by-id
                          [:mirror-pair/interpolation-to-string-self
                           :cross-mirror :equivalence])))
    (doseq [source-id [:mirror-pair/assert-true-body
                       :mirror-pair/not-false
                       :mirror-pair/neg-int
                       :mirror-pair/and-right-branch
                       :mirror-pair/and-short-circuit
                       :mirror-pair/or-right-branch
                       :mirror-pair/or-short-circuit
                       :mirror-pair/implies-false-short-circuit
                       :mirror-pair/implies-true-right-branch
                       :mirror-pair/implies-right-assoc
                       :mirror-pair/not-equal-int
                       :mirror-pair/less-or-equal-equal
                       :mirror-pair/greater-or-equal
                       :mirror-pair/with-basic
                       :mirror-pair/with-lexical-shadow
                       :mirror-pair/with-inner-shadow
                       :mirror-pair/with-closure-capture]]
      (is (= :agree (get-in row-by-id
                            [source-id :cross-mirror :equivalence]))
          (name source-id)))
    (doseq [source-id [:mirror-pair/lazy-take-zero-length
                       :mirror-pair/lazy-drop-length
                       :mirror-pair/lazy-append-length
                       :mirror-pair/lazy-intersect-attrnames
                       :mirror-pair/lazy-zip-length
                       :mirror-pair/lazy-zip-lists-with-head-ignores-right
                       :mirror-pair/lazy-zip-lists-with-elemat-ignores-right
                       :mirror-pair/lazy-zip-lists-with-head-ignores-right-tail-lhs
                       :mirror-pair/lazy-zip-lists-with-left-empty-ignores-right
                       :mirror-pair/lazy-zip-lists-with-right-empty-ignores-left
                       :mirror-pair/lazy-zip-lists-with-empty-both-ignores-f
                       :mirror-pair/lazy-sort-length
                       :mirror-pair/lazy-map-attrs-attrnames
                       :mirror-pair/lazy-map-attrs-to-list
                       :mirror-pair/lazy-filter-attrs-attrnames
                       :mirror-pair/lazy-zip-lists-with-length
                       :mirror-pair/seq-list-whnf-lazy
                       :mirror-pair/seq-attrset-whnf-lazy
                       :mirror-pair/deep-seq-attrset-ok
                       :mirror-pair/to-json-attrset
                       :mirror-pair/to-json-list
                       :mirror-pair/from-json-attr-select
                       :mirror-pair/replace-strings-basic
                       :mirror-pair/regex-match-capture
                       :mirror-pair/regex-split-capture
                       :mirror-pair/compare-versions-order
                       :mirror-pair/parse-drv-name-version
                       :mirror-pair/split-version-parts
                       :mirror-pair/floor-number
                       :mirror-pair/ceil-number
                       :mirror-pair/bit-and-positive
                       :mirror-pair/bit-or-positive
                       :mirror-pair/bit-xor-positive
                       :mirror-pair/try-eval-throw
                       :mirror-pair/generic-closure-chain
                       :mirror-pair/is-bool-false
                       :mirror-pair/is-int
                       :mirror-pair/is-float
                       :mirror-pair/is-string
                       :mirror-pair/is-list
                       :mirror-pair/is-null
                       :mirror-pair/is-function
                       :mirror-pair/lazy-map-length-ignored-input
                       :mirror-pair/lazy-filter-length-ignored-input
                       :mirror-pair/lazy-concat-map-empty-ignored-input
                       :mirror-pair/lazy-all-ignored-input
                       :mirror-pair/lazy-any-ignored-input
                       :mirror-pair/lazy-count-ignored-input
                       :mirror-pair/lazy-foldl-ignored-input
                       :mirror-pair/lazy-foldr-ignored-input
                       :mirror-pair/lazy-find-first-miss-ignored-input
                       :mirror-pair/lazy-group-by-attrnames-ignored-input
                       :mirror-pair/lazy-imap0-length-ignored-input
                       :mirror-pair/lazy-imap1-length-ignored-input
                       :mirror-pair/lazy-list-to-attrs-attrnames
                       :mirror-pair/lazy-zip-attrs-with-attrnames
                       :mirror-pair/lazy-zip-attrs-with-empty-rows-ignores-f
                       :mirror-pair/lazy-zip-attrs-with-empty-rows-ignores-f-length
                       :mirror-pair/lazy-cat-attrs-length
                       :mirror-pair/lazy-foldl-attrs-ignored-value
                       :mirror-pair/lazy-map-attrs-prime-attrnames
                       :mirror-pair/lazy-gen-attrs-attrnames
                       :mirror-pair/elem-numeric-equality
                       :mirror-pair/lazy-elem-stops-before-tail-error
                       :mirror-pair/lazy-elem-empty-list-needle
                       :mirror-pair/lazy-attr-values-length
                       :mirror-pair/lazy-attr-values-elem-at
                       :mirror-pair/lazy-values-alias-length
                       :mirror-pair/lazy-concat-lists-length
                       :mirror-pair/lazy-concat-lists-elem-at
                       :mirror-pair/lazy-reverse-list-length
                       :mirror-pair/lazy-reverse-list-elem-at
                       :mirror-pair/lazy-unique-singleton-length
                       :mirror-pair/find-numeric-equality
                       :mirror-pair/lazy-find-stops-before-tail-error
                       :mirror-pair/lazy-optionals-true-length
                       :mirror-pair/lazy-zip-attrs-with-values-length
                       :mirror-pair/lazy-recursive-update-nested-select
                       :mirror-pair/lazy-optional-false-value
                       :mirror-pair/lazy-optional-true-length
                       :mirror-pair/lazy-optional-string-false
                       :mirror-pair/lazy-gen-list-length
                       :mirror-pair/lazy-replicate-length
                       :mirror-pair/lazy-map-result-length
                       :mirror-pair/lazy-imap0-result-length
                       :mirror-pair/lazy-imap1-result-length
                       :mirror-pair/lazy-zip-lists-with-result-length
                       :mirror-pair/lazy-map-attrs-result-attrnames
                       :mirror-pair/lazy-map-attrs-to-list-result-length
                       :mirror-pair/lazy-map-attrs-prime-result-attrnames
                       :mirror-pair/lazy-zip-attrs-with-result-attrnames
                       :mirror-pair/lazy-map-identity-head
                       :mirror-pair/lazy-filter-true-head
                       :mirror-pair/lazy-concat-map-identity-head
                       :mirror-pair/lazy-sort-constant-comparator-length-two
                       :mirror-pair/lazy-filter-attrs-true-attrnames
                       :mirror-pair/lazy-map-attrs-identity-values-length
                       :mirror-pair/lazy-zip-attrs-with-head-values-length
                       :mirror-pair/lazy-concat-lists-map-result-length
                       :mirror-pair/dynamic-attr-key-select
                       :mirror-pair/dynamic-attr-key-attrnames
                       :mirror-pair/dynamic-attr-key-merge-attrnames
                       :mirror-pair/dynamic-select
                       :mirror-pair/dynamic-has-attr
                       :mirror-pair/dynamic-select-default
                       :mirror-pair/dynamic-string-template-has-attr
                       :mirror-pair/select-or-present
                       :mirror-pair/select-or-missing
                       :mirror-pair/select-or-nested-present
                       :mirror-pair/select-or-missing-intermediate
                       :mirror-pair/select-or-parenthesized-if-default
                       :mirror-pair/numeric-equality-int-float
                       :mirror-pair/numeric-equality-list-int-float
                       :mirror-pair/numeric-equality-attrset-int-float]]
      (is (= :agree (get-in row-by-id
                            [source-id :cross-mirror :equivalence]))
          (name source-id)))))

(deftest mirror-error-report-aligns-expected-error-boundaries
  (let [report (mirror-error/report)
        rows (:mirror-error-rows report)
        row-by-id (into {} (map (juxt :source-id identity) rows))]
    (is (= :mirror-error-report (:kind report)))
    (is (= :mirror-error-fixture-set (:fixture-kind report)))
    ;; rec-forward-reference was reclassified out of this error-agreement corpus
    ;; (see rec-forward-reference-taxonomy.md); it is a valid forward reference,
    ;; not an error the lanes agree on. Genuine error cases remain here.
    (is (= 4 (:fixture-count report)))
    (is (= 4 (:total report)))
    (is (= 4 (:accepted report)))
    (is (= 0 (:rejected report)))
    (is (= 0 (:held report)))
    (is (nil? (:first-frontier report)))
    (is (every? #(= :agree (:alignment %)) rows))
    (is (every? #(every? :ok? (:alignments %)) rows))
    (is (every? #(= "pnixc-pnix.eval.run-mirror.v0"
                    (get-in % [:observed :runtime-mirror-schema]))
                rows))
    (is (every? #(= "error"
                    (get-in % [:observed :runtime-mirror-status]))
                rows))
    (is (every? #(= :pnix.machine.eval-error-model.v1
                    (get-in % [:observed :eval-error-schema]))
                rows))
    (is (every? #(= :eval
                    (get-in % [:observed :eval-error-phase]))
                rows))
    (is (every? #(= (get-in % [:observed :eval-reason])
                    (get-in % [:observed :eval-error-reason]))
                rows))
    (is (= :unbound-var
           (get-in row-by-id
                   [:mirror-error/unknown-var :observed :eval-reason])))
    (is (= "Var"
           (get-in row-by-id
                   [:mirror-error/unknown-var
                    :observed :runtime-mirror-ast-tag])))
    (is (= :missing-attr
           (get-in row-by-id
                   [:mirror-error/missing-select
                    :observed :eval-reason])))
    (is (= "Select"
           (get-in row-by-id
                   [:mirror-error/missing-select
                    :observed :runtime-mirror-ast-tag])))
    (is (= :assertion-failed
           (get-in row-by-id
                   [:mirror-error/assertion-failed
                    :observed :eval-reason])))
    (is (= "Assert"
           (get-in row-by-id
                   [:mirror-error/assertion-failed
                    :observed :runtime-mirror-ast-tag])))
    (is (= :string-interpolation-coercion-failed
           (get-in row-by-id
                   [:mirror-error/interpolation-int-coercion
                    :observed :eval-reason])))
    (is (= "Binary"
           (get-in row-by-id
                   [:mirror-error/interpolation-int-coercion
                    :observed :runtime-mirror-ast-tag])))))

(deftest clojure-projection-report-validates-reader-term-shape-in-px
  (let [report (report-artifact/report-for :clojure-projection)
        rows (:clojure-projection-rows report)
        row-by-id (into {} (map (juxt :source-id identity) rows))
        host-crossing-rows (remove #(= :reader-source (:source-kind %)) rows)]
    (is (= :clojure-projection-report (:kind report)))
    (is (= :clojure-projection-fixture-set (:fixture-kind report)))
    (is (= "pnix-clj.clojure-projection.v0" (:schema report)))
    (is (= :ok (:runtime-status report)))
    (is (= "ok" (get (:runtime-self-test report) "status")))
    (is (= 46 (:fixture-count report)))
    (is (= 46 (:accepted report)))
    (is (= 0 (:rejected report)))
    (is (= 0 (:held report)))
    (is (= (count host-crossing-rows) (:host-crossing-count report)))
    (is (nil? (:first-frontier report)))
    (is (every? #(= "ok" (get-in % [:validation-value "status"]))
                rows))
    (is (every? #(= :ok (get-in % [:host-crossing :capability :status]))
                host-crossing-rows))
    (is (every? #(interop/witness? (get-in % [:host-crossing :witness]))
                host-crossing-rows))
    (is (= "Scalar"
           (get-in row-by-id
                   [:clojure-projection/int-scalar :term "tag"])))
    (is (= "int"
           (get-in row-by-id
                   [:clojure-projection/int-scalar :term "kind"])))
    (is (= "bigdec"
           (get-in row-by-id
                   [:clojure-projection/bigdecimal-scalar :term "kind"])))
    (is (= "Symbol"
           (get-in row-by-id
                   [:clojure-projection/symbol :term "tag"])))
    (is (= "clojure.core"
           (get-in row-by-id
                   [:clojure-projection/symbol :term "ns"])))
    (is (= "List"
           (get-in row-by-id
                   [:clojure-projection/list-form :term "tag"])))
    (is (= true
           (get-in row-by-id
                   [:clojure-projection/list-form :term "form"])))
    (is (= "List"
           (get-in row-by-id
                   [:clojure-projection/lazy-seq :term "tag"])))
    (is (= false
           (get-in row-by-id
                   [:clojure-projection/lazy-seq :term "form"])))
    (is (= "Map"
           (get-in row-by-id
                   [:clojure-projection/map :term "tag"])))
    (is (= "Set"
           (get-in row-by-id
                   [:clojure-projection/set :term "tag"])))
    (is (= "Var"
           (get-in row-by-id
                   [:clojure-projection/var-ref :term "tag"])))
    (is (= "clojure.core"
           (get-in row-by-id
                   [:clojure-projection/var-ref :term "ns"])))
    (is (= "Namespace"
           (get-in row-by-id
                   [:clojure-projection/namespace-object :term "tag"])))
    (is (= "Exception"
           (get-in row-by-id
                   [:clojure-projection/exception-object :term "tag"])))
    (is (= "ControlFlowReceipt"
           (get-in row-by-id
                   [:clojure-projection/control-flow-value :term "tag"])))
    (is (= "Keyword"
           (get-in row-by-id
                   [:clojure-projection/control-flow-finally-effect
                    :term "effects" 0 "tag"])))
    (is (= "finally"
           (get-in row-by-id
                   [:clojure-projection/control-flow-finally-effect
                    :term "effects" 0 "name"])))
    (is (= "MacroexpandReceipt"
           (get-in row-by-id
                   [:clojure-projection/macroexpand-when :term "tag"])))
    (is (= "macroexpand-all-trace"
           (get-in row-by-id
                   [:clojure-projection/macroexpand-when :term "phase"])))
    (is (pos? (get-in row-by-id
                      [:clojure-projection/macroexpand-when :term "step_count"])))
    (is (= "Map"
           (get-in row-by-id
                   [:clojure-projection/macroexpand-when
                    :term "steps" 0 "tag"])))
    (is (= "MacroexpandReceipt"
           (get-in row-by-id
                   [:clojure-projection/macroexpand-defmacro-syntax-quote
                    :term "tag"])))
    (is (<= 2
            (get-in row-by-id
                    [:clojure-projection/macroexpand-defmacro-syntax-quote
                     :term "step_count"])))
    (is (= ["defmacro" "syntax-quote" "unquote" "unquote-splicing" "auto-gensym"]
           (get-in row-by-id
                   [:clojure-projection/macroexpand-defmacro-syntax-quote
                    :term "features"])))
    (is (= "List"
           (get-in row-by-id
                   [:clojure-projection/macroexpand-defmacro-syntax-quote
                    :term "final_term" "tag"])))
    (is (= "DynamicBindingReceipt"
           (get-in row-by-id
                   [:clojure-projection/dynamic-binding :term "tag"])))
    (is (= "*print-length*"
           (get-in row-by-id
                   [:clojure-projection/dynamic-binding :term "var" "name"])))
    (is (= "JavaInteropReceipt"
           (get-in row-by-id
                   [:clojure-projection/java-interop-instance :term "tag"])))
    (is (= "length"
           (get-in row-by-id
                   [:clojure-projection/java-interop-instance :term "member"])))
    (is (= "java.lang.Integer"
           (get-in row-by-id
                   [:clojure-projection/java-interop-instance
                    :term "result_class"])))
    (is (= "sqrt"
           (get-in row-by-id
                   [:clojure-projection/java-interop-static-call
                    :term "member"])))
    (is (= "MAX_VALUE"
           (get-in row-by-id
                   [:clojure-projection/java-interop-static-field
                    :term "member"])))
    (is (= "JavaObject"
           (get-in row-by-id
                   [:clojure-projection/java-interop-constructor-object
                    :term "result" "tag"])))
    (is (= "java.lang.StringBuilder"
           (get-in row-by-id
                   [:clojure-projection/java-interop-constructor-object
                    :term "result" "class"])))
    (is (= "JavaClass"
           (get-in row-by-id
                   [:clojure-projection/java-class-object
                    :term "result" "tag"])))
    (is (= "java.lang.String"
           (get-in row-by-id
                   [:clojure-projection/java-class-object
                    :term "result" "name"])))
    (is (= "ReflectionReceipt"
           (get-in row-by-id
                   [:clojure-projection/reflection-declared-field
                    :term "tag"])))
    (is (= "java.awt.Point"
           (get-in row-by-id
                   [:clojure-projection/reflection-declared-field
                    :term "target_class"])))
    (is (= "x"
           (get-in row-by-id
                   [:clojure-projection/reflection-declared-field
                    :term "result" "value"])))
    (is (= "substring"
           (get-in row-by-id
                   [:clojure-projection/reflection-method-return-type
                    :term "member"])))
    (is (= "ClassloaderReceipt"
           (get-in row-by-id
                   [:clojure-projection/classloader-system
                    :term "tag"])))
    (is (= "JavaObject"
           (get-in row-by-id
                   [:clojure-projection/classloader-system
                    :term "result" "tag"])))
    (is (= "ClassloaderReceipt"
           (get-in row-by-id
                   [:clojure-projection/classloader-load-class
                    :term "tag"])))
    (is (= "JavaClass"
           (get-in row-by-id
                   [:clojure-projection/classloader-load-class
                    :term "result" "tag"])))
    (is (= "java.lang.String"
           (get-in row-by-id
                   [:clojure-projection/classloader-load-class
                    :term "result" "name"])))
    (is (= "NamespaceResolutionReceipt"
           (get-in row-by-id
                   [:clojure-projection/namespace-require-alias-resolve
                    :term "tag"])))
    (is (= "clojure.string"
           (get-in row-by-id
                   [:clojure-projection/namespace-require-alias-resolve
                    :term "result" "ns"])))
    (is (= "join"
           (get-in row-by-id
                   [:clojure-projection/namespace-require-alias-resolve
                    :term "result" "name"])))
    (is (= "JavaClass"
           (get-in row-by-id
                   [:clojure-projection/namespace-import-class-resolve
                    :term "result" "tag"])))
    (is (= "java.util.ArrayList"
           (get-in row-by-id
                   [:clojure-projection/namespace-import-class-resolve
                    :term "result" "name"])))
    (is (= "NamespaceResolutionReceipt"
           (get-in row-by-id
                   [:clojure-projection/namespace-local-var-resolve
                    :term "tag"])))
    (is (= "local-v"
           (get-in row-by-id
                   [:clojure-projection/namespace-local-var-resolve
                    :term "result" "name"])))
    (is (= "HostObjectConstructionReceipt"
           (get-in row-by-id
                   [:clojure-projection/host-object-deftype
                    :term "tag"])))
    (is (= "deftype"
           (get-in row-by-id
                   [:clojure-projection/host-object-deftype
                    :term "construction_kind"])))
    (is (= "PBox:7"
           (get-in row-by-id
                   [:clojure-projection/host-object-deftype
                    :term "object" "string"])))
    (is (some #{"clojure.lang.IType"}
              (get-in row-by-id
                      [:clojure-projection/host-object-deftype
                       :term "interfaces"])))
    (is (= true
           (get-in row-by-id
                   [:clojure-projection/host-object-defrecord
                    :term "record"])))
    (is (= "Map"
           (get-in row-by-id
                   [:clojure-projection/host-object-defrecord
                    :term "result" "tag"])))
    (is (some #{"clojure.lang.IRecord"}
              (get-in row-by-id
                      [:clojure-projection/host-object-defrecord
                       :term "interfaces"])))
    (is (= "reify"
           (get-in row-by-id
                   [:clojure-projection/host-object-reify-callable
                    :term "construction_kind"])))
    (is (some #{"java.util.concurrent.Callable"}
              (get-in row-by-id
                      [:clojure-projection/host-object-reify-callable
                       :term "interfaces"])))
    (is (= "PolymorphismDispatchReceipt"
           (get-in row-by-id
                   [:clojure-projection/polymorphism-protocol-deftype
                    :term "tag"])))
    (is (= "defprotocol"
           (get-in row-by-id
                   [:clojure-projection/polymorphism-protocol-deftype
                    :term "dispatch_kind"])))
    (is (= "Greeter"
           (get-in row-by-id
                   [:clojure-projection/polymorphism-protocol-deftype
                    :term "dispatch_value"])))
    (is (= "hi pnix"
           (get-in row-by-id
                   [:clojure-projection/polymorphism-protocol-deftype
                    :term "result" "value"])))
    (is (= "JavaObject"
           (get-in row-by-id
                   [:clojure-projection/polymorphism-protocol-deftype
                    :term "args" 0 "tag"])))
    (is (= "PolymorphismDispatchReceipt"
           (get-in row-by-id
                   [:clojure-projection/polymorphism-multimethod-keyword
                    :term "tag"])))
    (is (= "defmulti"
           (get-in row-by-id
                   [:clojure-projection/polymorphism-multimethod-keyword
                    :term "dispatch_kind"])))
    (is (= ":rect"
           (get-in row-by-id
                   [:clojure-projection/polymorphism-multimethod-keyword
                    :term "dispatch_value"])))
    (is (= 42
           (get-in row-by-id
                   [:clojure-projection/polymorphism-multimethod-keyword
                    :term "result" "value"])))
    (is (= "MetadataReceipt"
           (get-in row-by-id
                   [:clojure-projection/metadata-with-meta-vector
                    :term "tag"])))
    (is (= "with-meta"
           (get-in row-by-id
                   [:clojure-projection/metadata-with-meta-vector
                    :term "op"])))
    (is (= "vec"
           (get-in row-by-id
                   [:clojure-projection/metadata-with-meta-vector
                    :term "result" "value"])))
    (is (= "Vector"
           (get-in row-by-id
                   [:clojure-projection/metadata-with-meta-vector
                    :term "target" "tag"])))
    (is (= "MetadataReceipt"
           (get-in row-by-id
                   [:clojure-projection/metadata-var
                    :term "tag"])))
    (is (= "var-meta"
           (get-in row-by-id
                   [:clojure-projection/metadata-var
                    :term "op"])))
    (is (= "Var"
           (get-in row-by-id
                   [:clojure-projection/metadata-var
                    :term "target" "tag"])))
    (is (= "projection"
           (get-in row-by-id
                   [:clojure-projection/metadata-var
                    :term "result" "name"])))
    (is (= "StateEffectReceipt"
           (get-in row-by-id
                   [:clojure-projection/state-atom-swap
                    :term "tag"])))
    (is (= "atom-swap"
           (get-in row-by-id
                   [:clojure-projection/state-atom-swap
                    :term "op"])))
    (is (= "clojure.lang.Atom"
           (get-in row-by-id
                   [:clojure-projection/state-atom-swap
                    :term "target_class"])))
    (is (= 1
           (get-in row-by-id
                   [:clojure-projection/state-atom-swap
                    :term "initial" "value"])))
    (is (= 42
           (get-in row-by-id
                   [:clojure-projection/state-atom-swap
                    :term "final" "value"])))
    (is (= "swap!"
           (get-in row-by-id
                   [:clojure-projection/state-atom-swap
                    :term "effects" 1 "name"])))
    (is (= "StateEffectReceipt"
           (get-in row-by-id
                   [:clojure-projection/state-volatile-reset
                    :term "tag"])))
    (is (= "volatile-reset"
           (get-in row-by-id
                   [:clojure-projection/state-volatile-reset
                    :term "op"])))
    (is (= "clojure.lang.Volatile"
           (get-in row-by-id
                   [:clojure-projection/state-volatile-reset
                    :term "target_class"])))
    (is (= 9
           (get-in row-by-id
                   [:clojure-projection/state-volatile-reset
                    :term "result" "value"])))
    (is (= "LazyEvaluationReceipt"
           (get-in row-by-id
                   [:clojure-projection/lazy-delay-force
                    :term "tag"])))
    (is (= "delay-force"
           (get-in row-by-id
                   [:clojure-projection/lazy-delay-force
                    :term "op"])))
    (is (= 1
           (get-in row-by-id
                   [:clojure-projection/lazy-delay-force
                    :term "realized_count"])))
    (is (= "forced"
           (get-in row-by-id
                   [:clojure-projection/lazy-delay-force
                    :term "effects" 0 "name"])))
    (is (= 42
           (get-in row-by-id
                   [:clojure-projection/lazy-delay-force
                    :term "result" "value"])))
    (is (= "LazyEvaluationReceipt"
           (get-in row-by-id
                   [:clojure-projection/lazy-seq-take
                    :term "tag"])))
    (is (= "lazy-seq-take"
           (get-in row-by-id
                   [:clojure-projection/lazy-seq-take
                    :term "op"])))
    (is (= "clojure.lang.LazySeq"
           (get-in row-by-id
                   [:clojure-projection/lazy-seq-take
                    :term "target_class"])))
    (is (= 2
           (get-in row-by-id
                   [:clojure-projection/lazy-seq-take
                    :term "realized_count"])))
    (is (= "Vector"
           (get-in row-by-id
                   [:clojure-projection/lazy-seq-take
                    :term "result" "tag"])))
    (is (= 4
           (get-in row-by-id
                   [:clojure-projection/lazy-seq-take
                    :term "result" "items" 1 "value"])))
    (is (= "ConcurrencyReceipt"
           (get-in row-by-id
                   [:clojure-projection/concurrency-future-deref
                    :term "tag"])))
    (is (= "future-deref"
           (get-in row-by-id
                   [:clojure-projection/concurrency-future-deref
                    :term "op"])))
    (is (= true
           (get-in row-by-id
                   [:clojure-projection/concurrency-future-deref
                    :term "completed"])))
    (is (= "started"
           (get-in row-by-id
                   [:clojure-projection/concurrency-future-deref
                    :term "effects" 0 "name"])))
    (is (= "deref"
           (get-in row-by-id
                   [:clojure-projection/concurrency-future-deref
                    :term "effects" 1 "name"])))
    (is (= 42
           (get-in row-by-id
                   [:clojure-projection/concurrency-future-deref
                    :term "result" "value"])))
    (is (= "ConcurrencyReceipt"
           (get-in row-by-id
                   [:clojure-projection/concurrency-promise-deliver
                    :term "tag"])))
    (is (= "promise-deliver"
           (get-in row-by-id
                   [:clojure-projection/concurrency-promise-deliver
                    :term "op"])))
    (is (= true
           (get-in row-by-id
                   [:clojure-projection/concurrency-promise-deliver
                    :term "completed"])))
    (is (= "deliver"
           (get-in row-by-id
                   [:clojure-projection/concurrency-promise-deliver
                    :term "effects" 0 "name"])))
    (is (= 42
           (get-in row-by-id
                   [:clojure-projection/concurrency-promise-deliver
                    :term "result" "value"])))
    (is (= "CoordinationReceipt"
           (get-in row-by-id
                   [:clojure-projection/coordination-stm-dosync-alter
                    :term "tag"])))
    (is (= "dosync-alter"
           (get-in row-by-id
                   [:clojure-projection/coordination-stm-dosync-alter
                    :term "op"])))
    (is (= "clojure.lang.Ref"
           (get-in row-by-id
                   [:clojure-projection/coordination-stm-dosync-alter
                    :term "target_class"])))
    (is (= true
           (get-in row-by-id
                   [:clojure-projection/coordination-stm-dosync-alter
                    :term "completed"])))
    (is (= 42
           (get-in row-by-id
                   [:clojure-projection/coordination-stm-dosync-alter
                    :term "final" "value"])))
    (is (= "alter"
           (get-in row-by-id
                   [:clojure-projection/coordination-stm-dosync-alter
                    :term "effects" 2 "name"])))
    (is (= "CoordinationReceipt"
           (get-in row-by-id
                   [:clojure-projection/coordination-agent-send-await
                    :term "tag"])))
    (is (= "agent-send-await"
           (get-in row-by-id
                   [:clojure-projection/coordination-agent-send-await
                    :term "op"])))
    (is (= "clojure.lang.Agent"
           (get-in row-by-id
                   [:clojure-projection/coordination-agent-send-await
                    :term "target_class"])))
    (is (= true
           (get-in row-by-id
                   [:clojure-projection/coordination-agent-send-await
                    :term "completed"])))
    (is (= "await"
           (get-in row-by-id
                   [:clojure-projection/coordination-agent-send-await
                    :term "effects" 1 "name"])))
    (is (= 42
           (get-in row-by-id
                   [:clojure-projection/coordination-agent-send-await
                    :term "result" "value"])))))

(deftest clojure-form-report-compares-host-and-clj-meta-semantics
  (let [report (report-artifact/report-for :clojure-form)
        rows (:clojure-form-rows report)
        row-by-id (into {} (map (juxt :source-id identity) rows))]
    (is (= :clojure-form-report (:kind report)))
    (is (= :clojure-form-fixture-set (:fixture-kind report)))
    (is (= 54 (:fixture-count report)))
    (is (= 54 (:accepted report)))
    (is (= 0 (:rejected report)))
    (is (= 0 (:held report)))
    (is (nil? (:first-frontier report)))
    (is (every? #(= (get-in % [:host-result :value])
                    (get-in % [:clj-meta-result :value]))
                rows))
    (is (every? #(= :ok (get-in % [:host-result :capability :status]))
                rows))
    (is (every? #(interop/witness? (get-in % [:host-result :witness]))
                rows))
    (is (every? #(= :ok (get-in % [:projection-validation :status]))
                rows))
    (is (every? #(= 'pnix.clj-meta.compiler/eval-form
                    (get-in % [:clj-meta-result :execution-api]))
                rows))
    (is (every? #(true? (get-in % [:clj-meta-result :api-values-agree?]))
                rows))
    (is (= 42 (get-in row-by-id
                      [:clojure-form/fn-apply :host-result :value])))
    (is (= 42 (get-in row-by-id
                      [:clojure-form/let-bind :clj-meta-result :value])))
    (is (= 2 (get-in row-by-id
                     [:clojure-form/if-branch :host-result :value])))
    (is (= 3 (get-in row-by-id
                     [:clojure-form/do-form :host-result :value])))
    (is (= :divzero
           (get-in row-by-id
                   [:clojure-form/try-catch :clj-meta-result :value])))
    (is (= true
           (get-in row-by-id
                   [:clojure-form/letfn-mutual :host-result :value])))
    (is (= 10
           (get-in row-by-id
                   [:clojure-form/loop-recur :clj-meta-result :value])))
    (is (= 2
           (get-in row-by-id
                   [:clojure-form/case-keyword :host-result :value])))
    (is (= [1 1]
           (get-in row-by-id
                   [:clojure-form/try-finally :clj-meta-result :value])))
    (is (= ['if true]
           (get-in row-by-id
                   [:clojure-form/macroexpand-when :host-result :value])))
    (is (= "clojure.core"
           (get-in row-by-id
                   [:clojure-form/namespace-object-name
                    :clj-meta-result :value])))
    (is (= "#'clojure.core/+"
           (get-in row-by-id
                   [:clojure-form/var-object-name :host-result :value])))
    (is (= 1
           (get-in row-by-id
                   [:clojure-form/dynamic-binding :clj-meta-result :value])))
    (is (= 5
           (get-in row-by-id
                   [:clojure-form/java-instance-call :host-result :value])))
    (is (= 4.0
           (get-in row-by-id
                   [:clojure-form/java-static-call :clj-meta-result :value])))
    (is (= 42
           (get-in row-by-id
                   [:clojure-form/multi-arity-fn :host-result :value])))
    (is (= [1 2]
           (get-in row-by-id
                   [:clojure-form/variadic-rest-fn
                    :clj-meta-result :value])))
    (is (= [1 2 '(3 4)]
           (get-in row-by-id
                   [:clojure-form/vector-destructuring
                    :host-result :value])))
    (is (= 42
           (get-in row-by-id
                   [:clojure-form/map-destructuring-default
                    :clj-meta-result :value])))
    (is (= "x"
           (get-in row-by-id
                   [:clojure-form/locking-monitor
                    :host-result :value])))
    (is (= 10
           (get-in row-by-id
                   [:clojure-form/reduce-sequence
                    :clj-meta-result :value])))
    (is (= [2 4 6]
           (get-in row-by-id
                   [:clojure-form/mapv-closure
                    :host-result :value])))
    (is (= [1 3]
           (get-in row-by-id
                   [:clojure-form/filterv-predicate
                    :clj-meta-result :value])))
    (is (= [2 3 4]
           (get-in row-by-id
                   [:clojure-form/into-transducer
                    :host-result :value])))
    (is (= 9
           (get-in row-by-id
                   [:clojure-form/transduce-map
                    :clj-meta-result :value])))
    (is (= '(3 5)
           (get-in row-by-id
                   [:clojure-form/sequence-comp
                    :host-result :value])))
    (is (= [1 2]
           (get-in row-by-id
                   [:clojure-form/transient-vector-roundtrip
                    :clj-meta-result :value])))
    (is (= "FBox:7"
           (get-in row-by-id
                   [:clojure-form/deftype-object-form
                    :host-result :value])))
    (is (= [1 2 true]
           (get-in row-by-id
                   [:clojure-form/defrecord-map-form
                    :clj-meta-result :value])))
    (is (= "ok"
           (get-in row-by-id
                   [:clojure-form/reify-callable-form
                    :host-result :value])))
    (is (= true
           (get-in row-by-id
                   [:clojure-form/proxy-runnable-form
                    :clj-meta-result :value])))
    (is (= 42
           (get-in row-by-id
                   [:clojure-form/defmulti-dispatch-form
                    :clj-meta-result :value])))
    (is (= 7
           (get-in row-by-id
                   [:clojure-form/java-field-read
                    :host-result :value])))
    (is (= 8
           (get-in row-by-id
                   [:clojure-form/java-field-set
                    :clj-meta-result :value])))
    (is (= true
           (get-in row-by-id
                   [:clojure-form/java-instance-predicate
                    :host-result :value])))
    (is (= "ab"
           (get-in row-by-id
                   [:clojure-form/java-char-array-overload
                    :clj-meta-result :value])))
    (is (= "42"
           (get-in row-by-id
                   [:clojure-form/java-static-overload
                    :host-result :value])))
    (is (= 0
           (get-in row-by-id
                   [:clojure-form/java-arg-constructor
                    :clj-meta-result :value])))
    (is (= ["clojure.lang.BigInt" 5N]
           (get-in row-by-id
                   [:clojure-form/bigint-literal-class
                    :host-result :value])))
    (is (= ["clojure.lang.BigInt" 10000000000000000000N]
           (get-in row-by-id
                   [:clojure-form/bigint-out-of-long-literal
                    :clj-meta-result :value])))
    (is (= ["java.math.BigDecimal" 1.5M]
           (get-in row-by-id
                   [:clojure-form/bigdecimal-literal
                    :host-result :value])))
    (is (= ["clojure.lang.Ratio" 1/3]
           (get-in row-by-id
                   [:clojure-form/ratio-literal
                    :clj-meta-result :value])))
    (is (= ["java.util.regex.Pattern" true]
           (get-in row-by-id
                   [:clojure-form/regex-literal
                    :host-result :value])))
    (is (= '(1 2 sym)
           (get-in row-by-id
                   [:clojure-form/quoted-list-literal
                    :clj-meta-result :value])))
    (is (= #{1 2 3}
           (get-in row-by-id
                   [:clojure-form/set-literal
                    :host-result :value])))
    (is (= {:x [1 2], :y :z}
           (get-in row-by-id
                   [:clojure-form/nested-collection-literal
                    :clj-meta-result :value])))
    (is (= "a,b"
           (get-in row-by-id
                   [:clojure-form/requiring-resolve-join
                    :host-result :value])))
    (is (= "#'clojure.core/+"
           (get-in row-by-id
                   [:clojure-form/find-var-core-plus
                    :clj-meta-result :value])))
    (is (= "#'clojure.core/+"
           (get-in row-by-id
                   [:clojure-form/resolve-core-plus
                    :host-result :value])))
    (is (= "clojure.core"
           (get-in row-by-id
                   [:clojure-form/find-ns-core
                    :clj-meta-result :value])))
    (is (= ['local-i 7]
           (get-in row-by-id
                   [:clojure-form/intern-local-var
                    :host-result :value])))
    (is (= 42
           (get-in row-by-id
                   [:clojure-form/alter-var-root-local
                    :clj-meta-result :value])))
    (is (= true
           (get-in row-by-id
                   [:clojure-form/ns-publics-core-plus
                    :host-result :value])))
    (is (= true
           (get-in row-by-id
                   [:clojure-form/all-ns-core-present
                    :clj-meta-result :value])))))

(deftest evaluator-lazy-let-defers-and-recurses
  ;; Evaluator-lane laziness: `let` bindings are memoized thunks, so an unused
  ;; binding is never evaluated, forward/mutual references resolve, recursive
  ;; functions still work, and direct self-reference is a bounded
  ;; infinite-recursion held rather than a hang. Full 4-lane forward-reference
  ;; receipts live in `forward-reference-frontier-corpus`; this asserts the
  ;; semantic evaluator lane directly.
  (testing "an unused binding is never forced"
    (let [r (pnix/eval-source "let a = nonexistent; b = 2; in b")]
      (is (= :ok (:status r)))
      (is (= 2 (:value r)))))
  (testing "let is recursive: forward references resolve"
    (let [r (pnix/eval-source "let a = b; b = 5; in a")]
      (is (= :ok (:status r)))
      (is (= 5 (:value r)))))
  (testing "recursive functions still evaluate"
    (let [r (pnix/eval-source
             "let fib = n: if n < 2 then n else fib (n - 1) + fib (n - 2); in fib 10")]
      (is (= :ok (:status r)))
      (is (= 55 (:value r)))))
  (testing "a referenced unbound binding still errors"
    (let [r (pnix/eval-source "let a = nonexistent; in a")]
      (is (= :failed (:status r)))
      (is (= :unbound-var (:reason r)))))
  (testing "direct self-reference is a bounded infinite-recursion held"
    (let [r (pnix/eval-source "let a = a; in a")]
      (is (= :failed (:status r)))
      (is (= :infinite-recursion (:reason r))))))

(deftest evaluator-lazy-arguments-are-call-by-need
  ;; Function application is call-by-need: an unused argument to a simple-param
  ;; lambda is never evaluated, while builtins and pattern lambdas stay strict.
  (testing "an unused lambda argument is never evaluated"
    (let [r (pnix/eval-source "let const = x: y: x; in const 1 nonexistent")]
      (is (= :ok (:status r)))
      (is (= 1 (:value r))))
    (let [r (pnix/eval-source "let const = x: y: x; in const 1 (builtins.div 1 0)")]
      (is (= :ok (:status r)))
      (is (= 1 (:value r)))))
  (testing "used arguments and curried application still evaluate"
    (let [r (pnix/eval-source
             "let compose = f: g: x: f (g x); inc = x: x + 1; dbl = x: x * 2; in compose inc dbl 10")]
      (is (= :ok (:status r)))
      (is (= 21 (:value r)))))
  (testing "builtins remain strict in their arguments"
    (let [r (pnix/eval-source "builtins.add 1 (builtins.div 1 0)")]
      (is (= :failed (:status r)))))
  (testing "pattern lambdas force the argument to an attrset"
    (let [r (pnix/eval-source "({ x, y ? 1 }: x + y) { x = 5; }")]
      (is (= :ok (:status r)))
      (is (= 6 (:value r))))))

(deftest evaluator-lazy-collection-values-are-forced-on-demand
  ;; Attrset values and list elements are stored as thunks. Consumers that only
  ;; inspect shape or select a single slot must not force unrelated values.
  (testing "attr selection forces only the selected path"
    (let [r (pnix/eval-source
             "let s = { a = 1 / 0; b = { c = 5; d = 1 / 0; }; }; in s.b.c")]
      (is (= :ok (:status r)))
      (is (= 5 (:value r)))))
  (testing "attr name inspection does not force values"
    (let [r (pnix/eval-source "builtins.attrNames { a = 1 / 0; b = 2; }")]
      (is (= :ok (:status r)))
      (is (= ["a" "b"] (:value r)))))
  (testing "list length and selected elements avoid unrelated elements"
    (let [length-result (pnix/eval-source "builtins.length [ (1 / 0) ]")
          head-result (pnix/eval-source "builtins.head [ 1 (1 / 0) ]")
          elem-result (pnix/eval-source "builtins.elemAt [ (1 / 0) 7 ] 1")]
      (is (= :ok (:status length-result)))
      (is (= 1 (:value length-result)))
      (is (= :ok (:status head-result)))
      (is (= 1 (:value head-result)))
      (is (= :ok (:status elem-result)))
      (is (= 7 (:value elem-result))))))

(deftest evaluator-select-or-default
  ;; `a.b or default`: fallback when the *final* select's target is WHNF and
  ;; lacks the attr / is non-attrset. Does NOT catch a failed intermediate
  ;; select (oracle: `{a=1;}.b.c or 7` and `({a=1;}.b).c or 9` error on `.b`).
  (testing "returns the attribute when present"
    (let [r (pnix/eval-source "{ a = 1; }.a or 99")]
      (is (= :ok (:status r)))
      (is (= 1 (:value r)))))
  (testing "uses the default when the attribute is missing"
    (let [r (pnix/eval-source "{ a = 1; }.b or 99")]
      (is (= :ok (:status r)))
      (is (= 99 (:value r)))))
  (testing "present intermediate then missing final uses default (oracle)"
    (let [r (pnix/eval-source "{ a = {}; }.a.b or 7")]
      (is (= :ok (:status r)))
      (is (= 7 (:value r)))))
  (testing "missing intermediate select is NOT caught by outer or (oracle)"
    (let [r (pnix/eval-source "{ a = 1; }.b.c or 7")]
      (is (= :failed (:status r)))
      (is (= :missing-attr (:reason r))))
    (let [r (pnix/eval-source "({ a = 1; }.b).c or 9")]
      (is (= :failed (:status r)))
      (is (= :missing-attr (:reason r)))))
  (testing "null target with or uses default (oracle)"
    (is (= 5 (:value (pnix/eval-source "null.x or 5")))))
  (testing "does not swallow an unbound variable"
    (let [r (pnix/eval-source "missing.b or 7")]
      (is (= :failed (:status r)))
      (is (= :unbound-var (:reason r)))))
  (testing "a plain missing select without or is still held"
    (let [r (pnix/eval-source "{ a = 1; }.b")]
      (is (= :failed (:status r)))
      (is (= :missing-attr (:reason r)))))
  (testing "builtins.or still resolves as a normal select"
    (let [r (pnix/eval-source "builtins.or true false")]
      (is (= :ok (:status r)))
      (is (= true (:value r)))))
  (testing "parentheses are required for special-form defaults"
    (let [r (pnix/eval-source "{ a = 1; }.b or (if true then 8 else 9)")]
      (is (= :ok (:status r)))
      (is (= 8 (:value r)))))
  (testing "unparenthesized special-form defaults are syntax-held"
    (let [r (pnix/eval-source "{ a = 1; }.b or if true then 8 else 9")]
      (is (= :failed (:status r)))
      (is (= :unsupported-syntax (:reason r))))))

(deftest evaluator-assert-expression
  ;; `assert cond; body` evaluates body when cond is true, otherwise an
  ;; assertion-failed held; a failing condition propagates.
  (testing "a true condition evaluates the body"
    (let [r (pnix/eval-source "assert (1 == 1); 42")]
      (is (= :ok (:status r)))
      (is (= 42 (:value r)))))
  (testing "a false condition is an assertion-failed held"
    (let [r (pnix/eval-source "assert (1 == 2); 42")]
      (is (= :failed (:status r)))
      (is (= :assertion-failed (:reason r)))))
  (testing "a held condition propagates rather than asserting"
    (let [r (pnix/eval-source "assert missing; 42")]
      (is (= :failed (:status r)))
      (is (= :unbound-var (:reason r)))))
  (testing "asserts chain and the body can be any expression"
    (let [r (pnix/eval-source "assert (1 < 2); assert (2 < 3); let a = 1; in a + 6")]
      (is (= :ok (:status r)))
      (is (= 7 (:value r))))))

(deftest evaluator-at-patterns
  ;; `@`-patterns bind the whole argument attrset alongside the destructured
  ;; parameters, in either `name@{...}:` or `{...}@name:` position.
  (testing "leading name@{...} binds the whole set"
    (let [r (pnix/eval-source "(args@{ a, b }: a + b + args.a) { a = 1; b = 2; }")]
      (is (= :ok (:status r)))
      (is (= 4 (:value r)))))
  (testing "trailing {...}@name binds the whole set"
    (let [r (pnix/eval-source "({ a, b }@args: args.a + b) { a = 10; b = 20; }")]
      (is (= :ok (:status r)))
      (is (= 30 (:value r)))))
  (testing "@-binding coexists with defaults and ellipsis"
    (let [r (pnix/eval-source "(a@{ x, ... }: x + a.y) { x = 1; y = 2; z = 3; }")]
      (is (= :ok (:status r)))
      (is (= 3 (:value r)))))
  (testing "plain attr patterns still work"
    (let [r (pnix/eval-source "({ x, y ? 1 }: x + y) { x = 5; }")]
      (is (= :ok (:status r)))
      (is (= 6 (:value r))))))

(deftest evaluator-with-expression
  ;; `with attrs; body` adds attrs as a fallback scope behind the lexical env.
  (testing "with brings attrset members into scope"
    (let [r (pnix/eval-source "with { a = 1; b = 2; }; a + b")]
      (is (= :ok (:status r)))
      (is (= 3 (:value r)))))
  (testing "lexical bindings shadow a with scope"
    (let [r (pnix/eval-source "let a = 10; in with { a = 1; b = 2; }; a + b")]
      (is (= :ok (:status r)))
      (is (= 12 (:value r)))))
  (testing "an inner with shadows an outer with"
    (let [r (pnix/eval-source "with { a = 1; }; with { a = 2; }; a")]
      (is (= :ok (:status r)))
      (is (= 2 (:value r)))))
  (testing "with scope is captured by closures defined under it"
    (let [r (pnix/eval-source "(with { f = x: x + 1; }; (z: f z)) 41")]
      (is (= :ok (:status r)))
      (is (= 42 (:value r)))))
  (testing "with on a non-attrset is a no-op (oracle: with 5; 1 => 1)"
    ;; Pre-fix over-strict :with-not-attrset; Nix evaluates the body anyway.
    (is (= 1 (:value (pnix/eval-source "with 5; 1"))))
    (is (= 1 (:value (pnix/eval-source "with null; 1"))))
    (is (= 1 (:value (pnix/eval-source "with []; 1")))))
  (testing "a name absent from both lexical and with scope is unbound"
    (let [r (pnix/eval-source "with { a = 1; }; b")]
      (is (= :failed (:status r)))
      (is (= :unbound-var (:reason r))))))

(deftest evaluator-builtin-breadth-batch
  ;; First builtin-breadth batch (Completeness Roadmap Axis 1, item 5):
  ;; attrValues, concatStrings, hasPrefix, hasSuffix.
  (testing "attrValues returns values in sorted-key order"
    (let [r (pnix/eval-source "builtins.attrValues { b = 2; a = 1; }")]
      (is (= :ok (:status r)))
      (is (= [1 2] (:value r)))))
  (testing "concatStrings joins a list of strings"
    (let [r (pnix/eval-source "builtins.concatStrings [\"a\" \"b\" \"c\"]")]
      (is (= :ok (:status r)))
      (is (= "abc" (:value r)))))
  (testing "hasPrefix"
    (is (= true (:value (pnix/eval-source "builtins.hasPrefix \"ab\" \"abcdef\""))))
    (is (= false (:value (pnix/eval-source "builtins.hasPrefix \"xy\" \"abcdef\"")))))
  (testing "hasSuffix"
    (is (= true (:value (pnix/eval-source "builtins.hasSuffix \"ef\" \"abcdef\""))))
    (is (= false (:value (pnix/eval-source "builtins.hasSuffix \"ab\" \"abcdef\""))))))

(deftest evaluator-builtin-breadth-batch-2
  ;; Second builtin-breadth batch (Axis 1, item 5): bit ops, foldr, attrByPath.
  (testing "bit operators"
    (is (= 8 (:value (pnix/eval-source "builtins.bitAnd 12 10"))))
    (is (= 14 (:value (pnix/eval-source "builtins.bitOr 12 10"))))
    (is (= 6 (:value (pnix/eval-source "builtins.bitXor 12 10")))))
  (testing "foldr is a right fold"
    (let [r (pnix/eval-source "builtins.foldr (x: acc: x - acc) 0 [1 2 3]")]
      (is (= :ok (:status r)))
      (is (= 2 (:value r))))
    (is (= 10 (:value (pnix/eval-source
                       "builtins.foldr (x: acc: builtins.add x acc) 0 [1 2 3 4]"))))
    (testing "empty list returns the seed"
      (is (= 42 (:value (pnix/eval-source "builtins.foldr (x: acc: x - acc) 42 []"))))))
  (testing "attrByPath walks the path or returns the default"
    (is (= 7 (:value (pnix/eval-source
                      "builtins.attrByPath [\"a\" \"b\"] 99 { a = { b = 7; }; }"))))
    (is (= 99 (:value (pnix/eval-source
                       "builtins.attrByPath [\"a\" \"x\"] 99 { a = { b = 7; }; }"))))))

(deftest evaluator-indented-strings
  ;; `'' ''` indented strings: dedent + `${}` interpolation (Axis 1, item 2).
  (testing "single-line indented string"
    (let [r (pnix/eval-source "''hello''")]
      (is (= :ok (:status r)))
      (is (= "hello" (:value r)))))
  (testing "common leading indentation is stripped"
    (let [r (pnix/eval-source "''\n    foo\n    bar\n  ''")]
      (is (= :ok (:status r)))
      (is (= "foo\nbar\n" (:value r)))))
  (testing "interpolation inside an indented string"
    (let [r (pnix/eval-source "let n = 3; in ''count: ${builtins.toString n}''")]
      (is (= :ok (:status r)))
      (is (= "count: 3" (:value r)))))
  (testing "indented string escapes"
    (is (= "$" (:value (pnix/eval-source "''''$''"))))
    (is (= "quote: '' end" (:value (pnix/eval-source "''quote: ''' end''"))))
    (is (= "line\nnext" (:value (pnix/eval-source "''line''\\nnext''"))))
    (is (= "literal ${name}" (:value (pnix/eval-source "''literal ''${name}''")))))
  (testing "regular double-quoted strings still work"
    (is (= "a1b" (:value (pnix/eval-source "\"a${builtins.toString 1}b\""))))
    (is (= "plain" (:value (pnix/eval-source "\"plain\""))))))

(deftest string-interpolation-nested-braces
  ;; Interpolation scanning must find the matching interpolation close, not the
  ;; first `}` inside an attrset or string literal in the embedded expression.
  (doseq [[source expected]
          [["\"v=${{ a = \\\"one\\\"; }.a}\"" "v=one"]
           ["\"brace=${{ a = \\\"}\\\"; }.a}\"" "brace=}"]]]
    (let [receipt (pnix/verify-source source)]
      (is (= :accepted (:status receipt)) source)
      (is (= expected (get-in receipt [:eval-result :value])) source)
      (is (= expected (get-in receipt [:clj-meta-result :value])) source)
      (is (= expected (get-in receipt [:px-runtime :value])) source))))

(deftest string-interpolation-rejects-non-string-coercions
  ;; Interpolation is stricter than builtins.toString: ints, bools, null, lists,
  ;; plain attrsets, and lambdas do not coerce implicitly.
  (doseq [source ["\"value=${1}\""
                  "\"value=${true}\""
                  "\"value=${null}\""
                  "\"value=${[1 2]}\""
                  "\"value=${{ a = 1; }}\""
                  "\"value=${x: x}\""]]
    (let [receipt (pnix/verify-source source)]
      (is (= :failed (:status receipt)) source)
      (is (= :string-interpolation-coercion-failed
             (get-in receipt [:eval-result :reason]))
          source)))
  (testing "builtins.toString remains the explicit coercion escape hatch"
    (is (= "value=1"
           (:value (pnix/eval-source "\"value=${builtins.toString 1}\""))))))

(deftest evaluator-builtin-breadth-batch-3
  ;; Third builtin-breadth batch (Axis 1, item 5): dirOf, mapAttrsToList,
  ;; optional, optionals.
  (testing "dirOf"
    (is (= "/a/b" (:value (pnix/eval-source "builtins.dirOf \"/a/b/c\""))))
    (is (= "." (:value (pnix/eval-source "builtins.dirOf \"foo\""))))
    (is (= "/" (:value (pnix/eval-source "builtins.dirOf \"/a\"")))))
  (testing "mapAttrsToList maps over attrs in sorted-key order"
    (is (= ["a" "b"]
           (:value (pnix/eval-source "builtins.mapAttrsToList (k: v: k) { b = 2; a = 1; }"))))
    (is (= [1 2]
           (:value (pnix/eval-source "builtins.mapAttrsToList (k: v: v) { b = 2; a = 1; }")))))
  (testing "optional / optionals"
    (is (= [5] (:value (pnix/eval-source "builtins.optional true 5"))))
    (is (= [] (:value (pnix/eval-source "builtins.optional false 5"))))
    (is (= [1 2] (:value (pnix/eval-source "builtins.optionals true [1 2]"))))
    (is (= [] (:value (pnix/eval-source "builtins.optionals false [1 2]"))))))

(deftest evaluator-builtin-breadth-batch-4
  ;; Fourth builtin-breadth batch (Axis 1, item 5): toLower, toUpper,
  ;; stringToCharacters, range.
  (testing "toLower / toUpper"
    (is (= "abc" (:value (pnix/eval-source "builtins.toLower \"AbC\""))))
    (is (= "ABC" (:value (pnix/eval-source "builtins.toUpper \"AbC\"")))))
  (testing "stringToCharacters"
    (is (= ["a" "b" "c"]
           (:value (pnix/eval-source "builtins.stringToCharacters \"abc\"")))))
  (testing "range is inclusive and empty when from > to"
    (is (= [2 3 4 5] (:value (pnix/eval-source "builtins.range 2 5"))))
    (is (= [3] (:value (pnix/eval-source "builtins.range 3 3"))))
    (is (= [] (:value (pnix/eval-source "builtins.range 5 2"))))))

(deftest evaluator-builtin-breadth-batch-5
  ;; Fifth builtin-breadth batch (Axis 1, item 5): last, init, unique.
  (testing "last (held on empty)"
    (is (= 3 (:value (pnix/eval-source "builtins.last [1 2 3]"))))
    (is (= 9 (:value (pnix/eval-source "builtins.last [9]"))))
    (is (= :failed (:status (pnix/eval-source "builtins.last []")))))
  (testing "init (all but last)"
    (is (= [1 2] (:value (pnix/eval-source "builtins.init [1 2 3]"))))
    (is (= [] (:value (pnix/eval-source "builtins.init [9]")))))
  (testing "unique preserves first-seen order"
    (is (= [1 2 3] (:value (pnix/eval-source "builtins.unique [1 2 2 3 1 3]"))))))

(deftest evaluator-builtin-breadth-batch-6
  ;; Sixth builtin-breadth batch (Axis 1, item 5): concatMapStrings, splitString.
  (testing "concatMapStrings"
    (is (= "123"
           (:value (pnix/eval-source
                    "builtins.concatMapStrings (x: builtins.toString x) [1 2 3]"))))
    (is (= "a-b-"
           (:value (pnix/eval-source
                    "builtins.concatMapStrings (s: s + \"-\") [\"a\" \"b\"]")))))
  (testing "splitString keeps empty pieces; empty separator splits to chars"
    (is (= ["a" "b" "c"] (:value (pnix/eval-source "builtins.splitString \"/\" \"a/b/c\""))))
    (is (= ["" "a" ""] (:value (pnix/eval-source "builtins.splitString \"/\" \"/a/\""))))
    (is (= ["a" "b" "c"] (:value (pnix/eval-source "builtins.splitString \"\" \"abc\""))))))

(deftest evaluator-generic-closure
  ;; builtins.genericClosure: worklist traversal deduped by `key` (Axis 1, item 5).
  (testing "bounded traversal collects all reached items"
    (let [r (pnix/eval-source
             (str "builtins.genericClosure { startSet = [{ key = 1; }]; "
                  "operator = item: if item.key < 4 then [{ key = item.key + 1; }] else []; }"))]
      (is (= :ok (:status r)))
      (is (= [{"key" 1} {"key" 2} {"key" 3} {"key" 4}] (:value r)))))
  (testing "duplicate keys are visited once"
    (let [r (pnix/eval-source
             "builtins.genericClosure { startSet = [{ key = 1; } { key = 1; }]; operator = item: []; }")]
      (is (= :ok (:status r)))
      (is (= [{"key" 1}] (:value r)))))
  (testing "empty startSet"
    (is (= [] (:value (pnix/eval-source
                       "builtins.genericClosure { startSet = []; operator = item: []; }"))))))

(deftest evaluator-builtin-breadth-batch-8
  ;; Eighth builtin-breadth batch (Axis 1, item 5): min, max, imap0, imap1.
  (testing "min / max"
    (is (= 3 (:value (pnix/eval-source "builtins.min 3 7"))))
    (is (= 3 (:value (pnix/eval-source "builtins.min 7 3"))))
    (is (= 7 (:value (pnix/eval-source "builtins.max 3 7")))))
  (testing "imap0 / imap1 pass a 0- or 1-based index"
    (is (= [[0 "a"] [1 "b"]]
           (:value (pnix/eval-source "builtins.imap0 (i: x: [i x]) [\"a\" \"b\"]"))))
    (is (= [[1 "a"] [2 "b"]]
           (:value (pnix/eval-source "builtins.imap1 (i: x: [i x]) [\"a\" \"b\"]"))))))

(deftest evaluator-builtin-breadth-batch-9
  ;; Ninth builtin-breadth batch (Axis 1, item 5): optionalString, removePrefix,
  ;; removeSuffix, concatMapStringsSep.
  (testing "optionalString"
    (is (= "x" (:value (pnix/eval-source "builtins.optionalString true \"x\""))))
    (is (= "" (:value (pnix/eval-source "builtins.optionalString false \"x\"")))))
  (testing "removePrefix / removeSuffix (no-op when absent)"
    (is (= "cd" (:value (pnix/eval-source "builtins.removePrefix \"ab\" \"abcd\""))))
    (is (= "abcd" (:value (pnix/eval-source "builtins.removePrefix \"xy\" \"abcd\""))))
    (is (= "ab" (:value (pnix/eval-source "builtins.removeSuffix \"cd\" \"abcd\""))))
    (is (= "abcd" (:value (pnix/eval-source "builtins.removeSuffix \"xy\" \"abcd\"")))))
  (testing "concatMapStringsSep"
    (is (= "1, 2, 3"
           (:value (pnix/eval-source
                    "builtins.concatMapStringsSep \", \" (x: builtins.toString x) [1 2 3]"))))))

(deftest evaluator-builtin-breadth-batch-10
  ;; Tenth builtin-breadth batch (Axis 1, item 5): id, flip, toInt.
  ;; (`const` is intentionally omitted: it is lazy in its second argument in Nix,
  ;; which a strict builtin cannot model — write `x: y: x` for that.)
  (testing "id"
    (is (= 42 (:value (pnix/eval-source "builtins.id 42")))))
  (testing "flip f a b = f b a"
    (is (= 7 (:value (pnix/eval-source "builtins.flip (a: b: a - b) 3 10")))))
  (testing "toInt parses (held on non-numeric)"
    (is (= 42 (:value (pnix/eval-source "builtins.toInt \"  42 \""))))
    (is (= :failed (:status (pnix/eval-source "builtins.toInt \"xx\""))))))

(deftest interop-opaque-host-refs-and-marshalling
  ;; Interop boundary (separation §10): host objects cross as opaque refs and must
  ;; not be value-serialized into pnix terms; pure pnix values pass through.
  (testing "host-object? flags foreign JVM objects, not pnix values"
    (is (false? (interop/host-object? 42)))
    (is (false? (interop/host-object? [1 2 "x"])))
    (is (false? (interop/host-object? {"a" 1})))
    (is (true? (interop/host-object? (Point. 1 2))))
    (is (true? (interop/host-object? [1 (Point. 1 2)]))))
  (testing "from-host wraps a host object as an opaque ref; pure values pass through"
    (let [p (Point. 3 4)
          r (interop/from-host p)]
      (is (interop/opaque-host-ref? r))
      (is (= "java.awt.Point" (:class r)))
      (is (= p (:value (interop/opaque-ref-deref r))))
      (is (= p (interop/to-host r))))
    (is (= [1 {"a" 2}] (interop/from-host [1 {"a" 2}]))))
  (testing "a released opaque ref fails deterministically"
    (let [r (interop/from-host (Point.))]
      (interop/release-opaque-ref! r)
      (let [result (interop/opaque-ref-deref r)]
        (is (= :failed (:status result)))
        (is (= :opaque-ref-released (:reason result)))
        (is (= :interop-contract (get-in result [:error :phase])))))))

(deftest interop-effect-class-capability-gate
  ;; Interop boundary (separation §10): deny-by-default capability gate over a
  ;; closed effect-class taxonomy.
  (testing "deny-by-default: only :pure crosses without a grant"
    (is (= :ok (:status (interop/check-capability :pure))))
    (let [result (interop/check-capability :file-write)]
      (is (= :failed (:status result)))
      (is (= :capability-denied (:reason result)))
      (is (= :capability (get-in result [:error :phase])))))
  (testing "an explicit grant allows the effect"
    (is (= :ok (:status (interop/check-capability :file-write #{:pure :file-write})))))
  (testing "host compile is part of the closed crossing taxonomy"
    (is (= :ok (:status (interop/check-capability
                         :host-compile
                         interop/host-compile-capabilities)))))
  (testing "an unknown effect class fails its closed contract"
    (let [result (interop/check-capability :bogus)]
      (is (= :failed (:status result)))
      (is (= :unknown-effect-class (:reason result)))
      (is (= :interop-contract (get-in result [:error :phase])))))
  (testing "effect-class? recognizes the closed set"
    (is (true? (interop/effect-class? :reflection)))
    (is (false? (interop/effect-class? :nope)))))

(deftest interop-crossing-witness
  ;; Interop boundary (separation §10): a crossing witness is pure, content-hashed
  ;; evidence (deterministic; distinct fields -> distinct hash).
  (let [fields {:kind :call :direction :pnix->clojure :effect-class :host-call
                :loss-status :opaque :input-hash "a" :output-hash "b"}
        w (interop/make-witness fields)
        w-same (interop/make-witness fields)
        w-diff (interop/make-witness (assoc fields :output-hash "z"))]
    (testing "schema + predicate"
      (is (= :pnix-clj.interop.witness.v0 (:schema w)))
      (is (interop/witness? w))
      (is (false? (interop/witness? {:x 1}))))
    (testing "content-addressed and deterministic"
      (is (= (:witness-hash w) (:witness-hash w-same)))
      (is (not= (:witness-hash w) (:witness-hash w-diff)))))
  (testing "host exceptions are structured failures, not Held or exception text"
    (let [meta (interop/interop-meta {:direction :pnix->clojure
                                      :effect-class :host-call
                                      :loss-status :opaque})
          result (interop/run-crossing :failure-probe meta {:input 1}
                                       #{:host-call}
                                       #(throw (Exception. "private host text")))]
      (is (= :failed (:status result)))
      (is (= :host-call-failed (get-in result [:error :class])))
      (is (nil? (get-in result [:error :message])))
      (is (interop/witness? (:witness result))))))

(deftest interop-host-eval-is-gated-and-witnessed
  ;; Real host crossing: default capabilities deny host eval; the clojure-form
  ;; proof lane must pass an explicit :host-eval grant and gets a witness receipt.
  (testing "default host-eval is denied"
    (let [r (interop/host-eval-form :default-deny '(+ 1 2))]
      (is (= :failed (:status r)))
      (is (= :capability-denied (:reason r)))
      (is (= :host-eval (get-in r [:capability :effect])))
      (is (interop/witness? (:witness r)))))
  (testing "explicit host-eval grant runs and emits a witness"
    (let [r (interop/host-eval-form :explicit-grant
                                    '(+ 1 2)
                                    interop/host-eval-capabilities)]
      (is (= :ok (:status r)))
      (is (= 3 (:value r)))
      (is (= :ok (get-in r [:capability :status])))
      (is (= :host-eval (get-in r [:interop :effect-class])))
      (is (interop/witness? (:witness r)))))
  (testing "host-eval exceptions are stable failures"
    (let [r (interop/host-eval-form :failure
                                    '(throw (Exception. "private host text"))
                                    interop/host-eval-capabilities)]
      (is (= :failed (:status r)))
      (is (= :host-eval-failed (get-in r [:error :class])))
      (is (nil? (get-in r [:error :message]))))))

(deftest evaluator-path-literals
  ;; `./x` `../x` `~/x`, `/x`, and `<search>` path literals (Axis 1, item 2).
  ;; Division/comparison/select stay unaffected. Resolution (relative-to-file,
  ;; NIX_PATH) is a frontier; a path keeps its own value tag so it no longer
  ;; collapses into an ordinary string.
  (testing "path literals evaluate to path values"
    (is (= {"__pnix_value_kind" "path" "path" "./foo"} (:value (pnix/eval-source "./foo"))))
    (is (= {"__pnix_value_kind" "path" "path" "../bar"} (:value (pnix/eval-source "../bar"))))
    (is (= {"__pnix_value_kind" "path" "path" "~/x"} (:value (pnix/eval-source "~/x"))))
    (is (= {"__pnix_value_kind" "path" "path" "./a/b/c"} (:value (pnix/eval-source "./a/b/c"))))
    (is (= {"__pnix_value_kind" "path" "path" "<nixpkgs>"} (:value (pnix/eval-source "<nixpkgs>"))))
    (is (= [{"__pnix_value_kind" "path" "path" "./a"}
            {"__pnix_value_kind" "path" "path" "<nixpkgs>"}]
           (:value (pnix/eval-source "[ ./a <nixpkgs> ]")))))
  (testing "path values are distinct from strings"
    (is (= true (:value (pnix/eval-source "builtins.isPath ./foo"))))
    (is (= false (:value (pnix/eval-source "builtins.isPath \"./foo\""))))
    (is (= false (:value (pnix/eval-source "builtins.isString ./foo"))))
    (is (= "path" (:value (pnix/eval-source "builtins.typeOf ./foo"))))
    (is (= "./foo" (:value (pnix/eval-source "builtins.toString ./foo"))))
    (is (= "bar.txt" (:value (pnix/eval-source "builtins.baseNameOf ./foo/bar.txt"))))
    (is (= {"__pnix_value_kind" "path" "path" "./foo"}
           (:value (pnix/eval-source "builtins.dirOf ./foo/bar.txt"))))
    (is (= "path" (:value (pnix/eval-source "builtins.typeOf (builtins.dirOf ./foo/bar.txt)"))))
    (is (= true (:value (pnix/eval-source "./foo == ./foo"))))
    (is (= false (:value (pnix/eval-source "./foo == \"./foo\"")))))
  (testing "division, comparison, update, and select are unaffected"
    (is (= 3 (:value (pnix/eval-source "6 / 2"))))
    (is (= :failed (:status (pnix/eval-source "1 / 0"))))
    (is (= true (:value (pnix/eval-source "1 <= 2"))))
    (is (= {"a" 1 "b" 2} (:value (pnix/eval-source "{ a = 1; } // { b = 2; }"))))
    (is (= 1 (:value (pnix/eval-source "{ a = 1; }.a"))))))

(deftest evaluator-builtin-breadth-batch-11
  ;; Eleventh builtin-breadth batch (Axis 1, item 5): replicate, findFirst, foldl.
  (testing "replicate"
    (is (= ["a" "a" "a"] (:value (pnix/eval-source "builtins.replicate 3 \"a\""))))
    (is (= [] (:value (pnix/eval-source "builtins.replicate 0 9")))))
  (testing "findFirst returns the first match or the default"
    (is (= 3 (:value (pnix/eval-source "builtins.findFirst (x: x > 2) 99 [1 2 3 4]"))))
    (is (= 99 (:value (pnix/eval-source "builtins.findFirst (x: x > 9) 99 [1 2 3]")))))
  (testing "foldl is a left fold (alias of foldl')"
    (is (= 94 (:value (pnix/eval-source "builtins.foldl (acc: x: acc - x) 100 [1 2 3]"))))))

(deftest evaluator-recursive-update
  ;; builtins.recursiveUpdate: deep-merge (Axis 1, item 5).
  (testing "shallow merge keeps lhs-only keys; rhs wins on overlap"
    (is (= {"a" 1 "b" 3 "c" 4}
           (:value (pnix/eval-source
                    "builtins.recursiveUpdate { a = 1; b = 2; } { b = 3; c = 4; }")))))
  (testing "nested attrsets merge recursively"
    (is (= {"a" {"x" 1 "y" 9 "z" 3}}
           (:value (pnix/eval-source
                    "builtins.recursiveUpdate { a = { x = 1; y = 2; }; } { a = { y = 9; z = 3; }; }")))))
  (testing "a non-attrset on the right replaces"
    (is (= {"a" 5}
           (:value (pnix/eval-source
                    "builtins.recursiveUpdate { a = { x = 1; }; } { a = 5; }"))))))

(deftest evaluator-builtin-breadth-batch-13
  ;; Thirteenth builtin-breadth batch (Axis 1, item 5): hasInfix, pipe.
  (testing "hasInfix"
    (is (= true (:value (pnix/eval-source "builtins.hasInfix \"cd\" \"abcdef\""))))
    (is (= false (:value (pnix/eval-source "builtins.hasInfix \"zz\" \"abcdef\"")))))
  (testing "pipe threads a value left-to-right through a list of functions"
    (is (= 15 (:value (pnix/eval-source
                       "builtins.pipe 1 [ (x: x + 1) (x: x * 10) (x: x - 5) ]"))))
    (is (= 7 (:value (pnix/eval-source "builtins.pipe 7 []"))))))

(deftest evaluator-context-aware-string-builtins
  ;; Growing the context-aware-builtins allowlist: these string builtins now
  ;; accept contextful strings with Nix semantics — content-based results for
  ;; length/predicates, context KEPT on substring/case-conversion, context
  ;; UNIONED by concatStringsSep. Plain strings take the untouched legacy path.
  (let [L (str "let s = builtins.appendContext \"Hello\""
               " { \"/p1\" = { path = true; }; };"
               " t = builtins.appendContext \"World\""
               " { \"/p2\" = { path = true; }; }; in ")]
    (testing "content-based results"
      (is (= 5 (:value (pnix/eval-source (str L "builtins.stringLength s")))))
      (is (= true (:value (pnix/eval-source (str L "builtins.hasPrefix \"He\" s")))))
      (is (= true (:value (pnix/eval-source (str L "builtins.hasSuffix \"lo\" s")))))
      (is (= true (:value (pnix/eval-source (str L "builtins.hasInfix \"ell\" s"))))))
    (testing "substring slices content but keeps the whole context"
      (is (= {"/p1" {"path" true}}
             (:value (pnix/eval-source
                      (str L "builtins.getContext (builtins.substring 1 3 s)")))))
      (is (= "ell" (:value (pnix/eval-source
                            (str L "builtins.unsafeDiscardStringContext"
                                 " (builtins.substring 1 3 s)"))))))
    (testing "case conversion keeps context"
      (is (= "HELLO" (:value (pnix/eval-source
                              (str L "builtins.unsafeDiscardStringContext"
                                   " (builtins.toUpper s)")))))
      (is (= true (:value (pnix/eval-source
                           (str L "builtins.hasContext (builtins.toUpper s)"))))))
    (testing "path interpolation creates context; only full discard removes it"
      (let [source (str "[ (builtins.hasContext \"${./p}\")"
                        " (builtins.hasContext (builtins.unsafeDiscardOutputDependency \"${./p}\"))"
                        " (builtins.hasContext (builtins.unsafeDiscardStringContext \"${./p}\"))"
                        " (builtins.hasContext (builtins.toString ./p)) ]")
            receipt (pnix/verify-source {:source source})]
        (is (= [true true false false] (get-in receipt [:eval-result :value])))
        (is (= [true true false false] (get-in receipt [:clj-meta-result :value])))
        (is (= [true true false false] (get-in receipt [:px-runtime :value])))))
    (testing "concatStringsSep joins contents and unions contexts"
      (is (= "Hello, World"
             (:value (pnix/eval-source
                      (str L "builtins.unsafeDiscardStringContext"
                           " (builtins.concatStringsSep \", \" [ s t ])")))))
      (is (= {"/p1" {"path" true} "/p2" {"path" true}}
             (:value (pnix/eval-source
                      (str L "builtins.getContext"
                           " (builtins.concatStringsSep \", \" [ s t ])"))))))
    (testing "plain strings keep the legacy path (plain String results)"
      (is (= 5 (:value (pnix/eval-source "builtins.stringLength \"hello\""))))
      (is (= "ell" (:value (pnix/eval-source "builtins.substring 1 3 \"hello\""))))
      (is (string? (:value (pnix/eval-source
                            "builtins.concatStringsSep \", \" [\"a\" \"b\"]"))))
      ;; Phase D: non-string list elements are a TYPE error, like real Nix
      ;; (oracle-confirmed in D4: concatStringsSep on numbers errors)
      (is (= :string-list-builtin-non-string-element
             (:reason (pnix/eval-source
                       "builtins.concatStringsSep \",\" [1 2]")))))
    (testing "not-yet-context-aware builtins stay held at the frontier"
      (is (= :string-context-frontier
             (:reason (pnix/eval-source (str L "builtins.baseNameOf s")))))
      (is (= :string-context-frontier
             (:reason (pnix/eval-source
                       (str L "builtins.concatMapStringsSep \",\" (x: x) [ s ]"))))))
    (testing "replaceStrings carries source context plus USED replacement contexts"
      (is (= {"/p1" {"path" true} "/p2" {"path" true}}
             (:value (pnix/eval-source
                      (str L "builtins.getContext"
                           " (builtins.replaceStrings [\"l\"] [t] s)")))))
      (is (= {"/p1" {"path" true}}
             (:value (pnix/eval-source
                      (str L "builtins.getContext"
                           " (builtins.replaceStrings [\"zz\"] [t] s)"))))
          "an unused replacement contributes no context")
      (is (= "HeWorldWorldo"
             (:value (pnix/eval-source
                      (str L "builtins.unsafeDiscardStringContext"
                           " (builtins.replaceStrings [\"l\"] [t] s)"))))))))

(deftest evaluator-hash-string-nix-parity
  ;; Oracle-pinned against nix-instantiate 2.34.7. hashString is a pure JVM
  ;; MessageDigest acceleration of the Nix primitive, not a portable semantic
  ;; owner: exact algorithm names, UTF-8 bytes, lowercase hex, and Nix context
  ;; behavior are fixed here.
  (testing "all non-experimental Nix algorithms return lowercase hex"
    (doseq [[algorithm expected]
            [["md5" "900150983cd24fb0d6963f7d28e17f72"]
             ["sha1" "a9993e364706816aba3e25717850c26c9cd0d89d"]
             ["sha256" "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"]
             ["sha512" (str "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a"
                            "2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f")]]]
      (is (= expected
             (:value (pnix/eval-source
                      (str "builtins.hashString \"" algorithm "\" \"abc\""))))
          algorithm)))
  (testing "input text is hashed as UTF-8"
    (is (= "bd87f9bb68b67d2fa1cb82b6751820e946d5b1316d25d5fd96512fb4be44a2a8"
           (:value (pnix/eval-source
                    "builtins.hashString \"sha256\" \"한글\"")))))
  (testing "data context is consumed and the digest is context-free"
    (let [source (str "let s = builtins.appendContext \"abc\""
                      " { \"/nix/store/00000000000000000000000000000000-x\""
                      " = { path = true; }; }; in "
                      "builtins.getContext (builtins.hashString \"sha256\" s)")]
      (is (= {} (:value (pnix/eval-source source))))))
  (testing "algorithm context, unknown algorithms, invalid types, and raw bytes"
    (let [ctx-algorithm (str "let a = builtins.appendContext \"sha256\""
                             " { \"/nix/store/00000000000000000000000000000000-x\""
                             " = { path = true; }; }; in "
                             "builtins.hashString a \"abc\"")]
      (is (= :hash-string-algorithm-has-context
             (:reason (pnix/eval-source ctx-algorithm)))))
    (is (= :hash-string-unsupported-algorithm
           (:reason (pnix/eval-source
                     "builtins.hashString \"sha384\" \"abc\""))))
    (is (= :hash-string-algorithm-not-string
           (:reason (pnix/eval-source "builtins.hashString 1 \"abc\""))))
    (is (= :hash-string-data-not-string
           (:reason (pnix/eval-source "builtins.hashString \"sha256\" 1"))))
    (is (= "3ad4e44a4306fb62b2df0ab7069c67b9a0f8c8eff9f1cba8e7f851199df720c9"
           (:value (pnix/eval-source
                    "builtins.hashString \"sha256\" (builtins.substring 0 1 \"가\")")))))
  (testing "algorithm validation precedes payload forcing in every Clojure lane"
    (doseq [[source expected-reason]
            [["builtins.hashString \"sha3\" (throw \"payload\")"
              :hash-string-unsupported-algorithm]
             ["builtins.hashString 1 (throw \"payload\")"
              :hash-string-algorithm-not-string]
             ["builtins.hashString \"sha3\" 1"
              :hash-string-unsupported-algorithm]
             [(str "builtins.hashString (builtins.substring 0 1 \"가\") "
                   "(throw \"payload\")")
              :hash-string-raw-bytes-unsupported]]]
      (let [row (pnix/verify-source source)
            machine-result (machine/eval-source source)]
        (is (= expected-reason (get-in row [:eval-result :reason])) source)
        (is (= expected-reason (:reason machine-result)) source)
        (is (= expected-reason
               (get-in row [:clj-meta-result :error :class]))
            (str source " compiled lane"))
        (is (= :failed (get-in row [:px-runtime :status]))
            (str source " embedded lane"))
        (is (= expected-reason
               (get-in row [:px-runtime :error :class]))
            (str source " embedded selector order"))))
    (is (= :throw-builtin-called
           (:reason (pnix/eval-source
                     "builtins.hashString \"sha256\" (throw \"payload\")")))
        "a valid algorithm forces and propagates the payload error")
    (let [source (str "(builtins.tryEval (builtins.hashString \"sha256\""
                      " (throw \"payload\"))).success")
          row (pnix/verify-source source)]
      (is (= false (get-in row [:eval-result :value])) source)
      (is (= false (:value (machine/eval-source source))) source)
      (is (= false (get-in row [:clj-meta-result :value]))
          (str source " compiled catchability"))))
  (testing "invalid selector/data errors remain uncatchable through tryEval"
    (doseq [[source expected-reason]
            [["builtins.tryEval (builtins.hashString \"sha3\" \"payload\")"
              :hash-string-unsupported-algorithm]
             ["builtins.tryEval (builtins.hashString \"sha256\" 1)"
              :hash-string-data-not-string]]]
      (let [row (pnix/verify-source source)
            machine-result (machine/eval-source source)]
        (is (= expected-reason (get-in row [:eval-result :reason])) source)
        (is (= expected-reason (:reason machine-result)) source)
        (is (= expected-reason
               (get-in row [:clj-meta-result :error :class])) source)
        (is (= :failed (get-in row [:px-runtime :status])) source)
        (is (= expected-reason
               (get-in row [:px-runtime :error :class])) source))))
  (testing "the evaluator, machine, and lowered Clojure lane share one builtin"
    (doseq [source ["builtins.hashString \"sha256\" \"abc\""
                    "builtins.hashString \"sha256\" (builtins.substring 0 1 \"가\")"]]
      (let [row (pnix/verify-source source)
            expected (get-in row [:eval-result :value])]
        (is (= :ok (get-in row [:eval-result :status])))
        (is (= expected (get-in row [:clj-meta-result :value])))
        (is (= expected (:value (machine/eval-source source))))
        (is (= expected (get-in row [:px-runtime :value])))))))

(deftest evaluator-string-context-kinds-and-regex-json
  ;; Context KINDS + match/split/toJSON semantics, each verified against the
  ;; local Nix oracle (nix-instantiate 2.34.7) before implementation:
  ;;   - appendContext with an EMPTY info attrset adds NO context
  ;;   - path=true / allOutputs=true / outputs=[..] encode as "<p>" / "=<p>" /
  ;;     "!o!<p>" and getContext decodes them back, merging kinds per path
  ;;   - match/split results are context-FREE even from a contextful subject,
  ;;     and a contextful REGEX is an error
  ;;   - toJSON KEEPS the union of embedded contexts on the result string
  (testing "empty info attrset is a no-op (oracle: hasContext = false)"
    (is (= false (:value (pnix/eval-source
                          "builtins.hasContext (builtins.appendContext \"a\" { \"/p\" = {}; })")))))
  (testing "kinds round-trip through getContext"
    (is (= {"/p" {"path" true}}
           (:value (pnix/eval-source
                    "builtins.getContext (builtins.appendContext \"a\" { \"/p\" = { path = true; }; })"))))
    (is (= {"/d.drv" {"allOutputs" true}}
           (:value (pnix/eval-source
                    "builtins.getContext (builtins.appendContext \"a\" { \"/d.drv\" = { allOutputs = true; }; })"))))
    (is (= {"/d.drv" {"outputs" ["dev" "out"]}}
           (:value (pnix/eval-source
                    "builtins.getContext (builtins.appendContext \"a\" { \"/d.drv\" = { outputs = [\"dev\" \"out\"]; }; })")))))
  (testing "mixed kinds on one path merge (oracle shape)"
    (is (= {"/d.drv" {"allOutputs" true "outputs" ["out"]}}
           (:value (pnix/eval-source
                    (str "builtins.getContext"
                         " ((builtins.appendContext \"a\" { \"/d.drv\" = { allOutputs = true; }; })"
                         " + (builtins.appendContext \"b\" { \"/d.drv\" = { outputs = [\"out\"]; }; }))"))))))
  (let [L (str "let s = builtins.appendContext \"abc\""
               " { \"/p\" = { path = true; }; }; in ")]
    (testing "match groups are context-free; contextful regex is held"
      (is (= ["a" "bc"] (:value (pnix/eval-source
                                 (str L "builtins.match \"(a)(bc)\" s")))))
      (is (= false (:value (pnix/eval-source
                            (str L "builtins.hasContext"
                                 " (builtins.elemAt (builtins.match \"(a)(bc)\" s) 0)")))))
      (is (nil? (:value (pnix/eval-source (str L "builtins.match \"z\" s")))))
      (is (= :regex-argument-has-context
             (:reason (pnix/eval-source (str L "builtins.match s \"abc\""))))))
    (testing "split pieces are context-free"
      (is (= ["x" [] "y"]
             (:value (pnix/eval-source
                      (str "let s = builtins.appendContext \"xay\""
                           " { \"/p\" = { path = true; }; }; in builtins.split \"a\" s"))))))
    (testing "toJSON keeps embedded contexts on the result string"
      (is (= true (:value (pnix/eval-source
                           (str L "builtins.hasContext (builtins.toJSON { k = s; })")))))
      (is (= "{\"k\":\"abc\"}"
             (:value (pnix/eval-source
                      (str L "builtins.unsafeDiscardStringContext"
                           " (builtins.toJSON { k = s; })")))))
      (is (= {"/p" {"path" true}}
             (:value (pnix/eval-source
                      (str L "builtins.getContext"
                           " (builtins.toJSON { deep = { list = [ s 1 ]; }; })")))))
      (is (string? (:value (pnix/eval-source "builtins.toJSON { a = 1; }"))))
      ;; oracle: an attrset with outPath serializes as that path, recursively,
      ;; so a derivation becomes its store path with context kept.
      (is (= "\"/x\"" (:value (pnix/eval-source
                              "builtins.toJSON { outPath = \"/x\"; other = 1; }"))))
      (is (= "[\"/a\",\"/b\"]"
             (:value (pnix/eval-source
                      "builtins.toJSON [ { outPath = \"/a\"; } { outPath = \"/b\"; x = 2; } ]"))))
      (is (= true (:value (pnix/eval-source
                           (str "let d = builtins.derivation { name = \"t\";"
                                " system = \"s\"; builder = \"b\"; }; in"
                                " builtins.hasContext (builtins.toJSON { pkg = d; })"))))))
    (testing "toString collects contexts and coerces __toString/outPath (oracle)"
      (is (= true (:value (pnix/eval-source
                           (str L "builtins.hasContext (builtins.toString [ s \"b\" ])")))))
      (is (= "abc b" (:value (pnix/eval-source
                              (str L "builtins.unsafeDiscardStringContext"
                                   " (builtins.toString [ s \"b\" ])")))))
      (is (= "N" (:value (pnix/eval-source
                          "builtins.toString { __toString = self: self.n; n = \"N\"; }"))))
      (is (= true (:value (pnix/eval-source
                           (str L "builtins.hasContext"
                                " (builtins.toString { __toString = self: s; })"))))))
    (testing "toJSON __toString wins over outPath, gets self, must yield a string"
      (is (= "\"S\"" (:value (pnix/eval-source
                              "builtins.toJSON { __toString = self: \"S\"; outPath = \"/x\"; }"))))
      (is (= "\"N\"" (:value (pnix/eval-source
                              "builtins.toJSON { __toString = self: self.n; n = \"N\"; }"))))
      (is (= :to-json-tostring-not-string
             (:reason (pnix/eval-source "builtins.toJSON { __toString = self: 42; }")))))
    (testing "fromJSON rejects contextful input (oracle)"
      (is (= :from-json-argument-has-context
             (:reason (pnix/eval-source (str L "builtins.fromJSON s"))))))
    (testing "stringToCharacters keeps context per char; splitString pieces are context-free"
      (is (= true (:value (pnix/eval-source
                           (str L "builtins.hasContext"
                                " (builtins.head (builtins.stringToCharacters s))")))))
      (is (= ["x" "y"]
             (:value (pnix/eval-source
                      (str "let c = builtins.appendContext \"x-y\""
                           " { \"/nix/store/p\" = { path = true; }; }; in"
                           " builtins.splitString \"-\" c"))))))
    (testing "structural list ops pass contextful elements through"
      (is (= true (:value (pnix/eval-source (str L "builtins.elem \"abc\" [ s ]")))))
      (is (= 2 (:value (pnix/eval-source (str L "builtins.length [ s s ]"))))))
    (testing "string utils: removePrefix/removeSuffix keep ctx, toInt parses, concats union"
      (is (= true (:value (pnix/eval-source
                           (str L "builtins.hasContext (builtins.removePrefix \"a\" s)")))))
      (is (= "bc" (:value (pnix/eval-source
                           (str L "builtins.unsafeDiscardStringContext"
                                " (builtins.removePrefix \"a\" s)")))))
      (is (= "ab" (:value (pnix/eval-source
                           (str L "builtins.unsafeDiscardStringContext"
                                " (builtins.removeSuffix \"c\" s)")))))
      (is (= 42 (:value (pnix/eval-source
                         (str "let n = builtins.appendContext \"42\""
                              " { \"/nix/store/p\" = { path = true; }; }; in"
                              " builtins.toInt n")))))
      (is (= true (:value (pnix/eval-source
                           (str L "builtins.hasContext (builtins.concatStrings [ s \"b\" ])")))))
      (is (= "abc!" (:value (pnix/eval-source
                             (str L "builtins.unsafeDiscardStringContext"
                                  " (builtins.concatMapStrings (x: x) [ s \"!\" ])")))))
      (is (= true (:value (pnix/eval-source
                           (str L "builtins.hasContext"
                                " (builtins.concatMapStrings (x: x) [ s ])"))))))))

(deftest evaluator-derivation-pure-simulation
  ;; Completeness Roadmap item 3, second slice: derivation values as a pure
  ;; simulation (Tvix-style evaluator/builder separation — deterministic pseudo
  ;; store paths carrying string context, no builder, no on-disk store; paths
  ;; are NOT byte-compatible with real Nix hashing).
  (let [D (str "builtins.derivation { name = \"hello\";"
               " system = \"x86_64-linux\"; builder = \"/bin/sh\"; }")]
    (testing "derivation returns a type=derivation attrset with context paths"
      (is (= "derivation" (:value (pnix/eval-source (str "(" D ").type")))))
      (is (= "out" (:value (pnix/eval-source (str "(" D ").outputName")))))
      (is (= "hello" (:value (pnix/eval-source (str "(" D ").name")))))
      (is (= true (:value (pnix/eval-source
                           (str "builtins.hasContext (" D ").outPath")))))
      (is (= true (:value (pnix/eval-source
                           (str "builtins.hasContext (" D ").drvPath"))))))
    (testing "outPath/drvPath context shapes match the Nix oracle"
      ;; oracle (nix-instantiate 2.34.7): getContext d.outPath =
      ;; { <drvPath> = { outputs = ["out"]; }; }, getContext d.drvPath =
      ;; { <drvPath> = { allOutputs = true; }; }.
      (let [out (:value (pnix/eval-source
                         (str "builtins.unsafeDiscardStringContext (" D ").outPath")))
            ctx (:value (pnix/eval-source
                         (str "builtins.getContext (" D ").outPath")))
            drv-ctx (:value (pnix/eval-source
                             (str "builtins.getContext (" D ").drvPath")))]
        (is (str/starts-with? out "/nix/store/"))
        (is (str/ends-with? out "-hello"))
        (is (= 1 (count ctx)))
        (let [dep (first (keys ctx))]
          (is (str/starts-with? dep "/nix/store/"))
          (is (str/ends-with? dep "-hello.drv"))
          (is (= {"outputs" ["out"]} (get ctx dep)))
          (is (= {dep {"allOutputs" true}} drv-ctx)))))
    (testing "interpolating a derivation carries the dependency in context"
      (let [r (:value (pnix/eval-source
                       (str "let d = " D "; in builtins.getContext \"${d}/bin/hello\"")))]
        (is (= 1 (count r)))
        (is (str/starts-with? (first (keys r)) "/nix/store/"))
        (is (= {"outputs" ["out"]} (first (vals r))))))
    (testing "deterministic: same input same path, different name different path"
      (is (= true (:value (pnix/eval-source
                           (str "((" D ").outPath) == ((" D ").outPath)")))))
      (is (= false (:value (pnix/eval-source
                            (str "(builtins.derivation { name = \"a\"; system = \"s\";"
                                 " builder = \"b\"; }).outPath =="
                                 " (builtins.derivation { name = \"c\"; system = \"s\";"
                                 " builder = \"b\"; }).outPath"))))))
    (testing "derivationStrict returns drvPath and out with context"
      (is (= true (:value (pnix/eval-source
                           (str "builtins.hasContext (builtins.derivationStrict"
                                " { name = \"x\"; system = \"s\"; builder = \"b\"; }).out"))))))
    (testing "validation errors are held"
      (is (= :derivation-missing-required-attr
             (:reason (pnix/eval-source "builtins.derivation { name = \"x\"; }"))))
      (is (= :derivation-argument-not-attrset
             (:reason (pnix/eval-source "builtins.derivation 5"))))
      (is (= :derivation-attr-not-coercible
             (:reason (pnix/eval-source
                       (str "builtins.derivation { name = \"x\"; system = \"s\";"
                            " builder = \"b\"; f = (x: x); }"))))))
    (testing "placeholder is deterministic and context-free"
      (is (= true (:value (pnix/eval-source
                           "builtins.placeholder \"out\" == builtins.placeholder \"out\""))))
      (is (= false (:value (pnix/eval-source
                            "builtins.hasContext (builtins.placeholder \"out\")"))))
      (is (str/starts-with?
           (:value (pnix/eval-source "builtins.placeholder \"out\"")) "/")))
    (testing "storePath is purity-gated like Nix pure-eval mode"
      (is (= :store-path-purity-gated
             (:reason (pnix/eval-source "builtins.storePath \"/nix/store/x\"")))))
    (testing "multi-output derivations (oracle-verified shapes)"
      ;; oracle (nix-instantiate 2.34.7): outputName follows the FIRST output;
      ;; each d.<o> is a derivation attrset with its own outputName and
      ;; context { <drv> = { outputs = [o]; }; }; non-"out" paths get the
      ;; "-<output>" name suffix; derivationStrict returns one attr per output.
      (let [M (str "builtins.derivation { name = \"t\"; system = \"s\";"
                   " builder = \"b\"; outputs = [\"out\" \"dev\"]; }")]
        (is (= "out" (:value (pnix/eval-source (str "(" M ").outputName")))))
        (is (= ["out" "dev"] (:value (pnix/eval-source (str "(" M ").outputs")))))
        (is (= "dev" (:value (pnix/eval-source (str "(" M ").dev.outputName")))))
        (is (= "derivation" (:value (pnix/eval-source (str "(" M ").dev.type")))))
        (is (= {"outputs" ["dev"]}
               (first (vals (:value (pnix/eval-source
                                     (str "builtins.getContext (" M ").dev.outPath")))))))
        (is (str/ends-with?
             (:value (pnix/eval-source
                      (str "builtins.unsafeDiscardStringContext (" M ").dev.outPath")))
             "-t-dev"))
        (is (str/ends-with?
             (:value (pnix/eval-source
                      (str "builtins.unsafeDiscardStringContext (" M ").out.outPath")))
             "-t"))
        (is (= {"outputs" ["dev"]}
               (first (vals (:value (pnix/eval-source
                                     (str "let d = " M "; in"
                                          " builtins.getContext \"${d.dev}/inc\"")))))))
        (is (= "dev" (:value (pnix/eval-source
                              (str "(builtins.derivation { name = \"t\";"
                                   " system = \"s\"; builder = \"b\";"
                                   " outputs = [\"dev\" \"out\"]; }).outputName")))))
        (is (= ["dev" "drvPath" "out"]
               (:value (pnix/eval-source
                        (str "builtins.attrNames (builtins.derivationStrict"
                             " { name = \"t\"; system = \"s\"; builder = \"b\";"
                             " outputs = [\"out\" \"dev\"]; })")))))
        (is (= :derivation-invalid-outputs
               (:reason (pnix/eval-source
                         (str "builtins.derivation { name = \"t\"; system = \"s\";"
                              " builder = \"b\"; outputs = []; }")))))
        (is (= :derivation-invalid-outputs
               (:reason (pnix/eval-source
                         (str "builtins.derivation { name = \"t\"; system = \"s\";"
                              " builder = \"b\"; outputs = [\"out\" \"out\"]; }")))))))))

(deftest parser-interpolated-string-as-call-argument
  ;; `f "a${b}c"` is a valid call in Nix, but call-start-token? only accepted
  ;; plain :string tokens, so template/indented-string arguments fell out of the
  ;; application parse (held :unsupported-syntax). Discovered while testing
  ;; string-context interpolation.
  (testing "a template string is accepted as a direct call argument"
    (is (= 10 (:value (pnix/eval-source
                       "let s = \"q\"; in builtins.stringLength \"pre ${s} post\""))))
    (is (= "v=3!" (:value (pnix/eval-source
                           "(x: x + \"!\") \"v=${builtins.toString 3}\"")))))
  (testing "context flows through a template passed directly to a builtin"
    (is (= {"/p1" {"path" true} "/p2" {"path" true}}
           (:value (pnix/eval-source
                    (str "let a = builtins.appendContext \"A\""
                         " { \"/p1\" = { path = true; }; };"
                         " b = builtins.appendContext \"B\""
                         " { \"/p2\" = { path = true; }; };"
                         " in builtins.getContext \"${a}-${b}\""))))))
  (testing "plain-string call arguments and template bodies regress cleanly"
    (is (= 3 (:value (pnix/eval-source "builtins.stringLength \"abc\""))))
    (is (= "pre q post" (:value (pnix/eval-source
                                 "let s = \"q\"; in \"pre ${s} post\""))))))

(deftest unparse-roundtrip-full-grammar
  ;; Roadmap M1: the residual emitter. parse(unparse(ast)) must equal the
  ;; original AST up to span/source metadata, across the whole grammar.
  (doseq [src ["42" "1.5" "true" "null" "\"hi\"" "x: x + 1" "-3" "!true"
               "./foo" "<nixpkgs>" "assert true; 1" "with { a = 1; }; a"
               "import ./m" "rec { inherit x; a.b = 1; }" "{ a = 1; }.b or 0"
               "{ a = 1; } ? a" "let x = 5; in let y = x + 1; x = 10; in y"
               "[ 1 (2 + 3) \"s\" ]" "({ a, b ? 1, ... }@args: a + b) { a = 1; }"
               "if 1 < 2 then \"y\" else \"n\"" "builtins.length [ 1 2 3 ]"
               "\"pre ${toString 1} post\"" "let k = \"a\"; in { ${k} = 7; }.${k}"
               "\"quote\\\"back\\\\slash\\nnl\""]]
    (let [a1 (:ast (parser/parse-source src))
          out (unparse/unparse a1)
          p2 (parser/parse-source out)]
      (is (= :ok (:status p2)) (str "reparse: " src " -> " out))
      (is (= (unparse/strip-positions a1)
             (unparse/strip-positions (:ast p2)))
          (str "structural roundtrip: " src " -> " out)))))

(deftest lowering-lane-pattern-lambdas
  ;; Root of the functionArgs probe frontier: pattern lambdas now lower —
  ;; sequential formal binding (defaults see earlier formals, mirroring the
  ;; evaluator bind loop), @as binds the whole attrset, extra keys accepted,
  ;; and the lowered fn carries :pnix/function-args metadata so functionArgs
  ;; works on VALUES (the old syntactic special-case silently returned {}
  ;; through variables).
  (let [agree? (fn [src]
                 (let [row (pnix/verify-source src)]
                   (= (:value (:eval-result row))
                      (:value (:clj-meta-result row)))))]
    (is (agree? "({ a ? 1 }: a) {}"))
    (is (agree? "({ a, b ? 2 }: a + b) { a = 40; }"))
    (is (agree? "({ a, b ? a + 1 }: b) { a = 10; }") "default sees earlier formal")
    (is (agree? "({ a, ... }@args: args.b + a) { a = 1; b = 2; }"))
    (is (agree? "({ a }: a) { a = 1; b = 99; }") "extra keys accepted")
    (is (agree? "builtins.functionArgs ({ a, b ? 1 }: a)"))
    (is (agree? "let f = { a ? 1 }: a; in builtins.functionArgs f")
        "functionArgs through a variable (was silently {})")
    (is (agree? "builtins.functionArgs (x: x)"))
    (let [row (pnix/verify-source "({ a }: a) {}")]
      (is (= :failed (:status (:eval-result row))))
      (is (= :failed (:status (:clj-meta-result row))))
      (is (= (:value (:eval-result row)) (:value (:clj-meta-result row)))
          "missing formal holds on both lanes"))))

(deftest px-lane-pattern-lambdas
  ;; The .px self-runtime now evaluates pattern lambdas: mk_pattern_closure
  ;; carries formals, apply_pattern_closure binds sequentially (defaults see
  ;; earlier formals, present attrs stay lazy slots), and functionArgs is a
  ;; MetaBuiltin VALUE — the generic native path marshals closures into opaque
  ;; host fns (formals lost => silent {}), so functionArgs dispatches on the
  ;; raw meta value and survives variables AND aliases where the old
  ;; syntactic special-case went silent-wrong.
  (let [agree? (fn [src]
                 (let [row (pnix/verify-source src)]
                   (and (= :ok (:status (:px-runtime row)))
                        (= (:value (:eval-result row))
                           (:value (:px-runtime row))))))]
    (is (agree? "({ a ? 1 }: a) {}"))
    (is (agree? "({ a, b ? 2 }: a + b) { a = 40; }"))
    (is (agree? "({ a, b ? a + 1 }: b) { a = 10; }") "default sees earlier formal")
    ;; D19: an extra key WITHOUT `...` is an application-time ERROR in real
    ;; Nix — the old row here ("extra keys accepted") had pinned the lenient
    ;; pre-D19 behavior without oracle confirmation. Corrected: held on the
    ;; px lane too, and the ellipsis row keeps the accepted case covered.
    (let [row (pnix/verify-source "({ a }: a) { a = 1; b = 99; }")]
      (is (= :failed (:status (:eval-result row))) "extra key errors (D19)")
      (is (not= :ok (:status (:px-runtime row))) "extra key errors on px too"))
    (is (agree? "({ a, ... }: a) { a = 1; boom = 1 / 0; }")
        "unused extra attr stays lazy")
    (is (agree? "builtins.map ({ x, ... }: x + 1) [ { x = 1; } { x = 2; y = 9; } ]")
        "pattern closure marshals through a native builtin")
    (is (agree? "builtins.functionArgs ({ a, b ? 1 }: a)"))
    (is (agree? "let f = { a ? 1 }: a; in builtins.functionArgs f")
        "functionArgs through a variable")
    (is (agree? "let g = builtins.functionArgs; in g ({ x, y ? 2 }: x)")
        "functionArgs through an alias (MetaBuiltin value, not syntax)")
    (is (agree? "builtins.typeOf builtins.functionArgs"))
    (is (agree? "builtins.isFunction builtins.functionArgs"))
    (is (agree? "({ a }@args: a + args.a) { a = 21; }") "trailing @as")
    (is (agree? "(args@{ a }: a + args.a) { a = 21; }") "leading @as")
    (is (agree? "(args@{ a, b ? args.a + 1 }: b) { a = 10; }")
        "default sees the @ binding (bound before the formals fold)")
    (is (agree? "({ a ? 5 }@args: args) {}")
        "@ binds the ACTUAL argument, defaults excluded")
    (is (agree? "builtins.functionArgs ({ a, b ? 1 }@args: a)")
        "functionArgs reports formals only, not the @ binding")
    (let [row (pnix/verify-source "({ a }: a) {}")]
      (is (= :failed (:status (:eval-result row))))
      (is (= :failed (:status (:px-runtime row)))
          "missing formal holds on the .px lane too"))))

(deftest lowering-lane-builtin-values
  ;; Bare `builtins.X` (and `builtins` itself) in VALUE position: the whole
  ;; builtin set delegates to the evaluator through the bidirectional LAZY
  ;; bridge (slots<->thunks, fns<->:lazy-host-fn), so the lowered lane keeps
  ;; call-by-need through delegated builtins instead of leaving a free
  ;; `builtins` symbol the clj-meta host lane cannot execute.
  (let [agree? (fn [src]
                 (let [row (pnix/verify-source src)]
                   (and (= :ok (:status (:clj-meta-result row)))
                        (= (:value (:eval-result row))
                           (:value (:clj-meta-result row))))))]
    (is (agree? "let g = builtins.map; in g (x: x + 1) [ 1 2 ]"))
    (is (agree? "let m = builtins.map (x: x + 1); in m [ 1 2 ]")
        "partial application curries across the bridge")
    (is (agree? "let b = builtins; in b.map (x: x + 1) [ 5 ]")
        "the whole builtin set is a value")
    (is (agree? "let g = builtins.map; in builtins.length (g (x: 1 / 0) [ 1 2 ])")
        "bridged map stays element-lazy (length never forces)")
    (is (agree? "let g = builtins.head; in g [ 1 (1 / 0) ]")
        "bridged head leaves the tail unforced")
    (is (agree? "let builtins = 5; in builtins")
        "lexical shadowing beats the global table")
    (is (agree? "with { builtins = 3; }; builtins.storeDir")
        "statically-known builtins wins over `with` (was a live divergence)")
    (is (agree? "let b = builtins; in b.functionArgs ({ a ? 1 }: a)")
        "functionArgs override: lowered pattern metadata, not the bridge")
    (is (agree? "builtins.length (builtins.attrNames builtins)")
        "the table carries the evaluator's full key set")
    (is (agree? "let g = builtins.elem; in g 1 [ 1 2 ]"))
    (is (agree? "let t = builtins.typeOf; in t 5"))))

(deftest lowering-lane-lazy-application
  ;; Call-by-need application in the lowered lane: the argument crosses as a
  ;; lazy slot (params already force-slot on read), so an unused erroring
  ;; argument is never forced — Nix semantics, where the previous emission
  ;; `(f arg-form)` inherited Clojure strictness and held.
  (let [agree? (fn [src]
                 (let [row (pnix/verify-source src)]
                   (and (= :ok (:status (:clj-meta-result row)))
                        (= (:value (:eval-result row))
                           (:value (:clj-meta-result row))))))]
    (is (agree? "(x: 1) (1 / 0)"))
    (is (agree? "let f = x: 1; in f (1 / 0)"))
    (is (agree? "(x: y: x) 7 (1 / 0)") "curried, second argument unused")
    (is (agree? "let k = (x: y: x) 7; in k (1 / 0)"))
    (is (agree? "(x: x + 1) ((y: y) 41)") "used arguments still flow")
    (is (agree? "let f = x: builtins.length [ x ]; in f (1 / 0)")
        "unforced argument survives into a lazy list")
    ;; the .px lane applies call-by-need too now (thunked apply-arg).
    (is (= :ok (:status (:px-runtime (pnix/verify-source "(x: 1) (1 / 0)")))))))

(deftest px-lane-lazy-application
  ;; The .px meta evaluator's apply-arg is a thunk (pure recompute-on-force
  ;; call-by-name — values identical to call-by-need): Var reads force_value,
  ;; the native branch forces AT the host boundary (genuine builtins are
  ;; strict), and applyValue keeps passing RAW slots to marshalled closures —
  ;; forcing there evaluated ignored lazy slots (caught live by the
  ;; lazy-zip/mapAttrsToList corpus rows).
  (let [agree? (fn [src]
                 (let [row (pnix/verify-source src)]
                   (and (= :ok (:status (:px-runtime row)))
                        (= (:value (:eval-result row))
                           (:value (:px-runtime row))))))]
    (is (agree? "(x: 1) (1 / 0)"))
    (is (agree? "(x: y: x) 7 (1 / 0)") "curried, second argument unused")
    (is (agree? "let k = (x: y: x) 7; in k (1 / 0)"))
    (is (agree? "let f = x: builtins.length [ x ]; in f (1 / 0)")
        "unforced argument survives into a lazy list")
    (is (agree? "builtins.head (builtins.zipListsWith (a: b: a) [1] [(2 / 0)])")
        "marshalled callbacks still receive RAW slots (ignored stays lazy)")
    (let [row (pnix/verify-source "builtins.tryEval (1 / 0)")]
      (is (= :failed (:status (:eval-result row))))
      (is (= :failed (:status (:px-runtime row)))
          "tryEval still refuses uncatchables (laziness must not hide them)"))))

(deftest scoped-import-scope-semantics
  ;; `scopedImport scope path` (scope FIRST, path SECOND -- verified against
  ;; nix-instantiate 2.34.7). The scope attrs are added on top of the global
  ;; env for the imported module: base globals stay available, scope keys
  ;; shadow, and the scope does NOT propagate through nested plain import.
  ;; The DIRECT lane injects the scope fully; the mirror lanes hold honestly
  ;; for a non-empty scope (scope arrives host-shaped in the direct lane, but
  ;; `.px`-shaped / un-lowered in the others -- a marshalling/lowering
  ;; frontier). An EMPTY scope equals a plain import and agrees on all lanes.
  (let [dv (fn [src mods]
             (:value (:eval-result
                      (pnix/verify-source {:source src :import-modules mods}))))
        row (fn [src mods] (pnix/verify-source {:source src :import-modules mods}))]
    (testing "all four lanes: scope adds bindings on top of globals"
      (let [all (fn [src mods]
                  (let [r (row src mods)]
                    [(get-in r [:eval-result :value])
                     (get-in r [:clj-meta-result :value])
                     (get-in r [:px-runtime :value])]))]
        (is (= [15 15 15] (all "scopedImport { x = 10; y = 5; } ./m" {"./m" "x + y"})))
        (is (= [8 8 8] (all "scopedImport { x = 7; } ./m" {"./m" "builtins.add x 1"}))
            "base globals (builtins) stay available alongside scope")
        (is (= [42 42 42] (all "scopedImport { map = 42; } ./m" {"./m" "map"}))
            "scope shadows a global on name conflict")
        (is (= [6 6 6] (all "scopedImport { x = 1; y = 2; z = 3; } ./m"
                            {"./m" "x + y + z"})))))
    (testing "scope is lazy on ALL lanes (px lazy-scope bridge landed)"
      ;; every lane carries scope keys as lazy slots: the px lane now crosses
      ;; the boundary WHNF-only and the host wraps each slot call-by-need
      ;; (px_runtime/bridge-px-scope-value), so an unused erroring key never
      ;; evaluates anywhere -- real Nix scopedImport laziness, 4-lane.
      (is (= 1 (dv "scopedImport { a = 1; boom = 1 / 0; } ./m" {"./m" "a"})))
      (let [r (row "scopedImport { a = 1; boom = 1 / 0; } ./m" {"./m" "a"})]
        (is (= 1 (get-in r [:clj-meta-result :value])))
        (is (= :ok (get-in r [:px-runtime :status]))
            "px keeps the scope lazy across the boundary now")
        (is (= 1 (get-in r [:px-runtime :value])))))
    (testing "scope does NOT propagate through a nested plain import (all lanes)"
      ;; ./outer uses x (from scope) directly AND imports ./inner which also
      ;; references x -- inner must NOT see the scope, so it holds. clj-meta
      ;; munges scope params (x -> x*scope) so the injecting fn cannot capture
      ;; the inlined nested `x`.
      (let [r (row "scopedImport { x = 1; } ./outer"
                   {"./outer" "import ./inner" "./inner" "x"})]
        (is (= :failed (get-in r [:eval-result :status])) "direct: no propagation")
        (is (= :failed (get-in r [:clj-meta-result :status])) "clj-meta: no capture")))
    (testing "scope shadowing `builtins` agrees on every lane"
      (let [r (row "scopedImport { builtins = 5; } ./m" {"./m" "builtins"})]
        (is (= 5 (get-in r [:eval-result :value])) "direct merges it")
        (is (= 5 (get-in r [:px-runtime :value])) "px merges it (deep-forced)")
        (is (= 5 (get-in r [:clj-meta-result :value])) "clj-meta reads lexical builtins")
        (is (= :ok (get-in r [:lowering-result :status]))))
      (let [r (row "scopedImport { builtins = { x = 7; }; } ./m"
                   {"./m" "builtins.x"})]
        (is (= 7 (get-in r [:eval-result :value])))
        (is (= 7 (get-in r [:px-runtime :value])))
        (is (= 7 (get-in r [:clj-meta-result :value]))
            "lowering must not rewrite a lexical builtins.x as a host builtin")))
    (testing "non-empty scope collapses on all four lanes"
      (let [r (row "scopedImport { x = 10; y = 5; } ./m" {"./m" "x + y"})]
        (is (= 15 (get-in r [:eval-result :value])) "direct injects")
        (is (= 15 (get-in r [:clj-meta-result :value])) "clj-meta injects")
        (is (= 15 (get-in r [:px-runtime :value])) "px injects (deep-forced)")
        (is (= :mirrors-agree (get-in r [:cross-mirror-verdict :reason])))))
    (testing "empty scope == plain import, agreeing on all four lanes"
      (let [r (row "scopedImport {} ./m" {"./m" "40 + 2"})]
        (is (= :accepted (:status r)))
        (is (= 42 (get-in r [:eval-result :value])))
        (is (= 42 (get-in r [:clj-meta-result :value])))
        (is (= 42 (get-in r [:px-runtime :value])))
        (is (= :mirrors-agree (get-in r [:cross-mirror-verdict :reason])))))
    (testing "scope must be an attrset (path in scope position is held)"
      ;; the old bug passed the PATH first; now that is the scope slot, so a
      ;; path there is a scope-type error, not a resolved import.
      (let [r (row "scopedImport ./m {}" {"./m" "1"})]
        (is (= :failed (get-in r [:eval-result :status])))
        (is (= :scoped-import-scope-not-attrset
               (get-in r [:eval-result :reason])))))))

(deftest cross-lane-try-eval-catchability
  ;; Nix tryEval catches ONLY throw/assert. The lowered emission used to
  ;; `catch Throwable` and answer { success = false; } for EVERYTHING
  ;; (division by zero, missing attrs, abort) — a silent-wrong the direct
  ;; evaluator exposed; the .px lane could not tell throw from any other
  ;; error value. Now: lowered throw/assert carry :pnix/catchable ex-info
  ;; tags (bridged helds propagate catchability); .px errors carry an
  ;; explicit `catchable` flag (mk_catchable_error) honored by its tryEval.
  (let [statuses (fn [src]
                   (let [row (pnix/verify-source src)]
                     [(:status (:eval-result row))
                      (:status (:clj-meta-result row))
                      (:status (:px-runtime row))]))
        agree-ok? (fn [src]
                    (let [row (pnix/verify-source src)]
                      (and (= [:ok :ok :ok] (statuses src))
                           (= (:value (:eval-result row))
                              (:value (:clj-meta-result row))
                              (:value (:px-runtime row))))))]
    (is (agree-ok? "builtins.tryEval (builtins.throw \"boom\")"))
    (is (agree-ok? "builtins.tryEval (assert false; 1)"))
    (is (agree-ok? "builtins.tryEval (throw \"bare\")")
        "bare `throw` var lowers to the catchable value helper")
    (is (agree-ok? "let t = builtins.throw; in builtins.tryEval (t \"x\")")
        "throw through an alias keeps its error class")
    (is (agree-ok? "builtins.tryEval 5"))
    (is (agree-ok? "let r = builtins.tryEval (assert 1 == 2; \"nope\"); in if r.success then r.value else \"caught\""))
    ;; NOT catchable — every lane must refuse, none may answer
    ;; { success = false; }:
    (is (= [:failed :failed :failed] (statuses "builtins.tryEval (1 / 0)")))
    (is (= [:failed :failed :failed] (statuses "builtins.tryEval (builtins.throw 42)")))
    (is (= [:failed :failed :failed] (statuses "builtins.tryEval (builtins.head [ ])")))
    (is (= [:failed :failed :failed] (statuses "builtins.tryEval (builtins.abort \"stop\")")))))

(deftest px-lane-derivation-builtins
  ;; Derivation family in the .px lane by delegation (the .px program runs ON
  ;; this evaluator, so builtins.derivation inside .px IS the single host
  ;; implementation -- no hash canonicalization needed at all).
  (let [agree? (fn [src]
                 (let [row (pnix/verify-source src)]
                   (and (= :ok (:status (:px-runtime row)))
                        (= (:value (:eval-result row))
                           (:value (:px-runtime row))))))
        D (str "builtins.derivation { name = \"t\"; system = \"s\";"
               " builder = \"b\"; outputs = [\"out\" \"dev\"]; }")]
    (is (agree? (str "builtins.unsafeDiscardStringContext (" D ").outPath")))
    (is (agree? (str "builtins.getContext (" D ").outPath")))
    (is (agree? (str "builtins.unsafeDiscardStringContext (" D ").dev.outPath")))
    (is (agree? "builtins.placeholder \"out\""))
    (is (agree? (str "let d = " D "; in builtins.getContext \"${d}/bin\"")))))

(deftest specialize-refuses-position-observing-sources
  ;; Caught LIVE by the tower specialize-residual layer: folding literalizes
  ;; attrsets and erases position metadata, and a residual is different source
  ;; text, so no residual can preserve unsafeGetAttrPos answers. specialize
  ;; refuses honestly instead of changing an observable value.
  (is (= :position-observing-source-not-specializable
         (:reason (specialize/specialize
                   "builtins.unsafeGetAttrPos \"a\" { a = 1; }" {}))))
  (is (= :ok (:status (specialize/specialize "1 + 2" {}))))
  (let [t (tower/run-tower "builtins.unsafeGetAttrPos \"a\" { a = 1; }")]
    (is (= :failed (get-in t [:collapse :status])))
    (is (= :position-observing-source-not-specializable
           (get-in t [:collapse :blocking :reason])))))

(deftest px-lane-ctx-propagation
  ;; The .px lane's + / templates / toString now propagate context. The .px
  ;; ctx machinery was REPLACED by host-builtin delegation (the .px program
  ;; runs on this evaluator, so builtins.hasContext/... ARE the single
  ;; implementation) — the earlier .px reimplementation only worked while the
  ;; host predicates were thunk-opaque, and hardening them exposed the drift.
  (let [agree? (fn [src]
                 (let [row (pnix/verify-source src)]
                   (and (= :ok (:status (:px-runtime row)))
                        (= (:value (:eval-result row))
                           (:value (:px-runtime row))))))
        L (str "let c = builtins.appendContext \"a\""
               " { \"/nix/store/p\" = { path = true; }; }; in ")]
    (is (agree? (str L "builtins.getContext (c + c)")))
    (is (agree? (str L "builtins.unsafeDiscardStringContext (c + \"!\")")))
    (is (agree? (str L "builtins.getContext \"pre ${c} post\"")))
    (is (agree? (str L "builtins.hasContext (builtins.toString c)")))
    (is (agree? "builtins.getContext (builtins.appendContext \"a\" { \"/d.drv\" = { allOutputs = true; outputs = [\"out\"]; }; })"))
    (is (agree? "\"a\" + \"b\"") "plain + regresses")
    (is (agree? "let x = 3; in \"v=${builtins.toString x}\"")
        "plain templates regress")))

(deftest lowering-lane-ctx-propagation
  ;; The last lowering piece of string-context: lowered + and string templates
  ;; now propagate context through shared evaluator accessors (plus /
  ;; template-join helpers), and interpolate/coerce pass contextful strings
  ;; through instead of mangling the tagged map.
  (let [agree? (fn [src]
                 (let [row (pnix/verify-source src)]
                   (and (= :ok (:status (:clj-meta-result row)))
                        (= (:value (:eval-result row))
                           (:value (:clj-meta-result row))))))
        L (str "let c = builtins.appendContext \"a\""
               " { \"/nix/store/p\" = { path = true; }; }; in ")]
    (is (agree? (str L "builtins.getContext (c + c)")))
    (is (agree? (str L "builtins.unsafeDiscardStringContext (c + \"!\")")))
    (is (agree? (str L "builtins.getContext \"pre ${c} post\"")))
    (is (agree? (str L "builtins.hasContext (builtins.toString c)")))
    (is (agree? "\"a\" + \"b\"") "plain strings regress")
    ;; Phase D: string + non-string holds on BOTH lanes (host leak removed)
    (let [row (pnix/verify-source "\"a\" + 1")]
      (is (not= :ok (get-in row [:eval-result :status])))
      (is (not= :ok (get-in row [:clj-meta-result :status]))))
    (is (agree? "let x = 3; in \"v=${builtins.toString x}\"")
        "plain templates regress")))

(deftest lowering-lane-derivation-builtins
  ;; Derivation-family lift, lowering side, via the generic host-builtin
  ;; delegation (single implementation, zero drift). The .px lane remains the
  ;; derivation frontier and keeps the tower held-probe honest.
  (let [agree? (fn [src]
                 (let [row (pnix/verify-source src)]
                   (and (= :ok (:status (:clj-meta-result row)))
                        (= (:value (:eval-result row))
                           (:value (:clj-meta-result row))))))
        D (str "builtins.derivation { name = \"t\"; system = \"s\";"
               " builder = \"b\"; outputs = [\"out\" \"dev\"]; }")]
    (is (agree? (str "(" D ").outPath")))
    (is (agree? (str "(" D ").dev.outPath")) "multi-output sub-derivation")
    (is (agree? "builtins.derivationStrict { name = \"x\"; system = \"s\"; builder = \"b\"; }"))
    (is (agree? "builtins.placeholder \"out\""))
    (is (agree? (str "builtins.getContext (" D ").outPath"))
        "outPath context kinds agree lane-to-lane")
    (is (= :store-path-purity-gated
           (:reason (lowering/lower-ast
                     (:ast (parser/parse-source "builtins.storePath \"/x\""))))))))

(deftest lowering-lane-string-context-builtins
  ;; Frontier lift, lowering side: hasContext/getContext/
  ;; unsafeDiscardStringContext/appendContext now lower to runtime helpers
  ;; that DELEGATE to the evaluator's own builtins (zero semantic drift by
  ;; construction). The .px runtime lane remains the declared frontier.
  (let [lane-agrees?
        (fn [src]
          (let [row (pnix/verify-source src)
                lane (first (filter #(= :pnix-clj-lowering-clj-meta (:lane %))
                                    (:lane-summary row)))]
            (and (= :ok (:status lane))
                 (= (:value (:eval-result row))
                    (:value (:clj-meta-result row))))))
        MK "builtins.appendContext \"a\" { \"/nix/store/p\" = { path = true; }; }"]
    (is (lane-agrees? "builtins.hasContext \"x\""))
    (is (lane-agrees? (str "builtins.hasContext (" MK ")")))
    (is (lane-agrees? (str "builtins.getContext (" MK ")")))
    (is (lane-agrees? (str "builtins.unsafeDiscardStringContext (" MK ")")))
    (is (lane-agrees?
         "builtins.getContext (builtins.appendContext \"a\" { \"/d.drv\" = { allOutputs = true; }; })"))
    (is (lane-agrees?
         "builtins.hasContext (builtins.appendContext \"a\" { \"/p\" = {}; })")
        "empty info no-op agrees across lanes")))

(deftest cached-eval-content-addressed
  ;; Roadmap M6: purity + determinism make content-addressed memoization
  ;; sound. The key is the position-stripped AST hash, so formatting variants
  ;; share an entry; impure/held/callable results always bypass — the cache
  ;; can skip recomputation but never change an answer.
  (cached-eval/clear-eval-cache!)
  (testing "content addressing: formatting variants share one entry"
    (is (= :miss (get-in (cached-eval/cached-eval "1 + 2") [:cache :status])))
    (is (= :hit (get-in (cached-eval/cached-eval "  1   +   2  ") [:cache :status])))
    (is (= :hit (get-in (cached-eval/cached-eval "(1 + 2)") [:cache :status])))
    (is (= 3 (:value (cached-eval/cached-eval "1 + 2")))))
  (testing "guards bypass instead of caching"
    (is (= :statically-impure
           (get-in (cached-eval/cached-eval "builtins.getEnv \"HOME\"")
                   [:cache :reason])))
    (is (= :result-not-ok
           (get-in (cached-eval/cached-eval "1 / 0") [:cache :reason])))
    (is (= :value-not-cacheable
           (get-in (cached-eval/cached-eval "x: x") [:cache :reason]))))
  (testing "the report cross-checks cached == fresh over a corpus sample"
    (let [{:keys [status total accepted]} (cached-eval/report)]
      (is (= :ok status))
      (is (= total accepted)))))

(deftest synthesize-reverse-projection
  ;; Roadmap M3: whitelisted Clojure expression core -> pnix, deny-by-default,
  ;; verified end-to-end: clj-meta evaluates the ORIGINAL form and the
  ;; synthesized pnix must collapse through the M2 tower to the same value.
  (testing "core forms project and verify across every lane"
    (let [{:keys [status total accepted rejected rows]} (synthesize/report)]
      (is (= :ok status))
      (is (= total accepted))
      (is (zero? rejected))
      (is (every? #(= :ok (:bytecode-determinism %)) rows))))
  (testing "the sequential/recursive let trap is refused, not mistranslated"
    (is (= :sequential-let-not-projectable
           (:reason (synthesize/form->pnix (quote (let [x 1 x (+ x 1)] x))))))
    (is (= :sequential-let-not-projectable
           (:reason (synthesize/form->pnix (quote (let [a (+ b 1) b 2] a)))))))
  (testing "host machinery is statically denied"
    (is (= :non-projectable-form
           (:reason (synthesize/form->pnix (quote (.length "abc"))))))
    (is (= :non-projectable-form
           (:reason (synthesize/form->pnix (quote clojure.core/println))))))
  (testing "a simple projection round-trips by value"
    (let [p (synthesize/form->pnix (quote (let [x 40 y 2] (+ x y))))]
      (is (= :ok (:status p)))
      (is (= 42 (:value (pnix/eval-source (:source p))))))))

(deftest safe-eval-sandbox-tier
  ;; Roadmap M5: sandbox-by-design (runtime purity gates) + explicit limits as
  ;; structured verdicts, reusing the M1 fuel seam.
  (testing "pure evaluation passes through"
    (is (= 7 (:value (safe-eval/safe-eval "1 + 2 * 3")))))
  (testing "fuel exhaustion is a verdict, not a crash"
    (let [r (safe-eval/safe-eval "let f = x: f x; in f 1" {:fuel 4096})]
      (is (= :suspended (:status r)))
      (is (= :fuel (:limit-exceeded r)))))
  (testing "a runtime purity gate is tagged :impure"
    (let [r (safe-eval/safe-eval "builtins.getEnv \"HOME\"")]
      (is (= :suspended (:status r)))
      (is (= :effect-denied (get-in r [:error :class])))
      (is (= :impure (:limit-exceeded r)))))
  (testing "pure-only? refuses statically-impure sources before evaluating"
    (let [r (safe-eval/safe-eval "builtins.readFile \"/etc/passwd\""
                                 {:pure-only? true})]
      (is (= :failed (:status r)))
      (is (= :effect-denied (:reason r)))
      (is (= :impure (:limit-exceeded r))))
    (is (= 3 (:value (safe-eval/safe-eval "builtins.length [ 1 2 3 ]"
                                          {:pure-only? true})))))
  (testing "static-purity-check walks without evaluating"
    (let [r (safe-eval/static-purity-check
             "if c then builtins.readFile p else builtins.getEnv \"X\"")]
      (is (false? (:pure? r)))
      (is (= 2 (count (:impure-uses r)))))
    (is (true? (:pure? (safe-eval/static-purity-check "let a = 1; in a + 2"))))
    (is (false? (:pure? (safe-eval/static-purity-check "builtins.${k} \"/x\"")))
        "dynamic builtins access is conservatively impure"))
  (testing "the report is green"
    (let [{:keys [status total accepted]} (safe-eval/report)]
      (is (= :ok status))
      (is (= total accepted)))))

(deftest capabilities-index-generated-and-drift-checked
  ;; Roadmap M4: the capability index is derived only from code, renders
  ;; deterministically, and the committed doc matches a fresh render (the
  ;; same check the gate runs).
  (let [idx (capabilities/index)]
    (is (= (mapv name receipt/lane-order) (:lanes idx)))
    (is (pos? (:builtin-count idx)))
    (is (some #{"specialize"} (:report-artifacts idx)))
    (is (some #{"tower"} (:report-artifacts idx)))
    (is (contains? (:public-api idx) "pnix-clj.specialize"))
    (is (= (capabilities/render idx) (capabilities/render (capabilities/index)))
        "render is deterministic"))
  (is (= :ok (:status (capabilities/check)))
      "committed docs/CAPABILITIES.md matches a fresh render"))

(deftest wiki-registry-generated-and-gate-checked
  ;; The project WIKI (docs/WIKI.md) is machine-generated from the
  ;; report-artifact registry + resources/pnix_clj/roadmap.edn, and gate-checked
  ;; for BOTH drift and integrity (a :landed roadmap item that names a
  ;; :capability must have that report-kind actually wired -- no false "done").
  (testing "capability registry covers every report-artifact kind"
    (let [reg (wiki/capability-registry)
          kinds (set (map :kind reg))]
      (is (= (count reg) (count report-artifact/supported-kinds)))
      (is (contains? kinds "futamura"))
      (is (contains? kinds "specialize"))
      (is (contains? kinds "tower"))))
  (testing "roadmap integrity: every landed item's capability is wired"
    (is (= :ok (:status (wiki/integrity)))))
  (testing "roadmap registers landed pillars AND planned research findings"
    (let [items (:items (wiki/roadmap))
          by-id (into {} (map (juxt :id identity) items))]
      (is (= :landed (:status (by-id :m1-specialize))))
      (is (= :landed (:status (by-id :f1-futamura-2nd))))
      (is (= :futamura (:capability (by-id :f1-futamura-2nd))))
      (is (= :landed (:status (by-id :f8-weval-ir-pe))))
      (is (= :weval (:capability (by-id :f8-weval-ir-pe))))
      ;; F7b stays the registered-but-HELD research example (owner sign-off)
      (is (= :planned (:status (by-id :f7b-self-applicable-specializer))))))
  (testing "render is deterministic and the committed doc matches"
    (is (= (wiki/render) (wiki/render)))
    (is (= :ok (:status (wiki/check)))
        "committed docs/WIKI.md matches a fresh render + passes integrity")))

(deftest tower-single-entrypoint-collapse
  ;; Roadmap M2: one call climbs read -> emit-roundtrip -> direct-eval ->
  ;; lowering -> clj-meta host -> px-runtime -> pnix-mirror and collapses.
  ;; Repackages existing run-source lanes; adds only the emit-roundtrip layer.
  (testing "a fully-supported source collapses across all layers"
    (let [t (tower/run-tower "let x = 40; in x + 2")]
      (is (= :collapsed (get-in t [:collapse :status])))
      (is (= 42 (get-in t [:collapse :value])))
      (is (= 8 (count (:layers t))))
      (is (some #(= :specialize-residual (:layer %)) (:layers t)))
      (is (every? :ok? (:pairs t)))))
  (testing "a frontier source degrades honestly (held, with blocking layer)"
    ;; the probe walks the frontier as lifts land: ... -> import-with-modules
    ;; -> scopedImport scope injection -> erroring-unused-scope (now collapses
    ;; 4-lane via the px lazy-scope bridge) -> the remaining frontier is the
    ;; module-free import (no module map wired), held at direct eval.
    (let [t (tower/run-tower "import ./mod.px")]
      (is (= :failed (get-in t [:collapse :status])))
      (is (= :direct-eval (get-in t [:collapse :blocking :layer])))))
  (testing "the previous probes now collapse"
    ;; px-scope-laziness lifted: an unused ERRORING scope key stays lazy on
    ;; every lane, so the old held-probe collapses to the module value.
    (let [t (tower/run-tower {:source "scopedImport { a = 1; boom = 1 / 0; } ./m.px"
                              :import-modules {"./m.px" "a"}})]
      (is (= :collapsed (get-in t [:collapse :status])))
      (is (= 1 (get-in t [:collapse :value]))))
    (let [t (tower/run-tower
             "let f = { a ? 1 }: a; in builtins.functionArgs f")]
      (is (= :collapsed (get-in t [:collapse :status])))
      (is (= {"a" true} (get-in t [:collapse :value]))))
    (let [t (tower/run-tower "({ a }@args: a + args.a) { a = 21; }")]
      (is (= :collapsed (get-in t [:collapse :status])))
      (is (= 42 (get-in t [:collapse :value]))))
    (let [t (tower/run-tower "let g = builtins.map; in g (x: x + 1) [ 1 2 ]")]
      (is (= :collapsed (get-in t [:collapse :status])))
      (is (= [2 3] (get-in t [:collapse :value]))))
    (let [t (tower/run-tower "(x: 1) (1 / 0)")]
      (is (= :collapsed (get-in t [:collapse :status])))
      (is (= 1 (get-in t [:collapse :value]))))
    ;; select directly on a tryEval application: the emitted `try` sits in
    ;; expression position (operand stack non-empty) — clj-meta now hoists
    ;; tries into zero-arg fn calls exactly like Clojure Compiler.java.
    (let [t (tower/run-tower
             "(builtins.tryEval (builtins.throw \"x\")).success")]
      (is (= :collapsed (get-in t [:collapse :status])))
      (is (= false (get-in t [:collapse :value]))))
    ;; import WITH an in-memory module map collapses across every lane now
    ;; that the tower threads modules through the whole climb.
    (let [t (tower/run-tower {:source "(import ./five.px) + 10"
                              :import-modules {"./five.px" "5"}})]
      (is (= :collapsed (get-in t [:collapse :status])))
      (is (= 15 (get-in t [:collapse :value]))))
    ;; scopedImport scope injection collapses on all four lanes (direct merge,
    ;; clj-meta fn-injection with munged params, px deep-force).
    (let [t (tower/run-tower {:source "scopedImport { x = 10; y = 5; } ./m.px"
                              :import-modules {"./m.px" "x + y"}})]
      (is (= :collapsed (get-in t [:collapse :status])))
      (is (= 15 (get-in t [:collapse :value])))))
  (testing "string-context sources now collapse (the lifted frontier)"
    (is (= :collapsed
           (get-in (tower/run-tower
                    "builtins.hasContext (builtins.appendContext \"x\" { \"/p\" = { path = true; }; })")
                   [:collapse :status]))))
  (testing "the report collapses the whole mirror-pair corpus"
    (let [{:keys [status total accepted rejected failure-probe]} (tower/report)]
      (is (= :ok status))
      (is (= total accepted))
      (is (zero? rejected))
      (is (:classified? failure-probe)))))

(deftest property-fuzzer-cross-lane-collapse
  ;; deep-research F3: property-based generative differential fuzzing
  ;; (test.check): random pnix expressions must collapse to one value across
  ;; the four substrates; any divergence SHRINKS to a minimal source. This
  ;; capability immediately found two real bugs: clj-meta nested-let same-name
  ;; shadowing (fixed below) and host-parser let/if-as-operator-RHS leniency
  ;; (Nix rejects it; filed as host-parser-let-if-rhs).
  (let [{:keys [status pass? cross-lane-pass? specializer-pass? cache-pass?
                specializer-proven-arith-pass? machine-pass?
                num-tests-run smallest-failing-source]}
        (property-fuzzer/report {:num-tests 60 :seed 42})]
    (is (= :ok status)
        (str "generative divergence found; smallest="
             (pr-str smallest-failing-source)))
    (is pass?)
    (is cross-lane-pass? "generated exprs collapse across 4 substrates")
    (is specializer-pass? "partial evaluation preserves meaning under any split")
    (is cache-pass? "content-addressed cache never changes a value")
    (is specializer-proven-arith-pass?
        "specializer PROVEN meaning-preserving for arithmetic (all values)")
    ;; M7h: the machine agrees with the evaluator on RANDOM sources too —
    ;; the shared machine differential corpus lives in machine/differential-corpus; the
    ;; fifth property sweeps the space generatively (exact ok-and-held
    ;; agreement, stronger than value collapse) with shrinking.
    (is machine-pass?
        (str "machine⇄evaluator generative divergence; smallest="
             (pr-str smallest-failing-source)))
    (is (pos? num-tests-run)))
  (testing "clj-meta nested lazy-letrec with a SHADOWED name compiles"
    ;; the regression the fuzzer found: an outer lazy-letrec must not rewrite a
    ;; same-named inner binding's refs to the outer cell.
    (is (= 0 (get-in (pnix/verify-source
                      "(let v = 0; in (v + (let v = 0; in (v + 0))))") [:eval-result :value])))
    (is (= 3 (get-in (pnix/verify-source
                      "(let v = 1; in (let v = 2; in (let v = 3; in v)))") [:eval-result :value])))
    (let [r (pnix/verify-source "(let v = 5; in (let v = 6; in v))")]
      (is (= :accepted (:status r)))
      (is (= 6 (get-in r [:clj-meta-result :value])) "clj-meta shadowing"))))

(deftest bool-proof-truth-table-equivalence
  ;; PROVEN boolean equivalence by EXHAUSTIVE truth-table evaluation -- a
  ;; complete proof over the finite boolean domain (companion to arith-proof).
  (let [{:keys [status total accepted rejected]} (bool-proof/report)]
    (is (= :ok status))
    (is (= total accepted))
    (is (zero? rejected)))
  (testing "classic identities are PROVEN"
    (is (= :proven (:status (bool-proof/prove-equivalent "(!(a && b))" "((!a) || (!b))"))))
    (is (= :proven (:status (bool-proof/prove-equivalent "(a -> b)" "((!a) || b)"))))
    (is (= :proven (:status (bool-proof/prove-equivalent "(a && (b || c))"
                                                         "((a && b) || (a && c))")))))
  (testing "non-equivalences are refuted with a witnessing assignment"
    (let [r (bool-proof/prove-equivalent "a && b" "a || b")]
      (is (= :refuted (:status r)))
      (is (map? (:assignment r)))))
  (testing "honest boundary: too many vars -> :unprovable, not a false proof"
    (let [r (bool-proof/prove-equivalent
             "(a && b && c && d && e && f && g && h && i)" "false")]
      (is (= :unprovable (:status r)))
      (is (= :too-many-vars (:reason r))))))

(deftest replay-reverifies-persisted-witness
  ;; REPLAY/AUDIT: a persisted §15 witness is independently re-verified in a
  ;; fresh run -- the §8/§9 determinism guarantee checked ACROSS process, from
  ;; durable evidence alone.
  (let [{:keys [status total accepted rejected]} (replay/report)]
    (is (= :ok status))
    (is (= total accepted))
    (is (zero? rejected)))
  (testing "a persisted witness reproduces identically from disk"
    (let [dir (str (System/getProperty "java.io.tmpdir") "/pnix-replay-t-" (System/nanoTime))
          d (witnessed-run/run-witnessed-durable "let x = 40; in x + 2" dir)
          ps (persist/open-persistent-store dir)
          r (replay/replay-witness ps (get-in d [:persisted :witness-id]))]
      (is (= :reproduced (:verdict r)))
      (is (empty? (:diffs r)))
      (is (= (:term-hash (:original r)) (:term-hash (:fresh r))))
      (doseq [f (reverse (file-seq (clojure.java.io/file dir)))] (.delete f))))
  (testing "a missing witness is reported, not silently reproduced"
    (let [dir (str (System/getProperty "java.io.tmpdir") "/pnix-replay-m-" (System/nanoTime))
          ps (persist/open-persistent-store dir)]
      (is (= :missing (:verdict (replay/replay-witness ps "nope"))))
      (doseq [f (reverse (file-seq (clojure.java.io/file dir)))] (.delete f)))))

(deftest persist-durable-content-addressed-store
  ;; DURABLE backing: §3 terms + §5 events persisted to disk content-addressed,
  ;; reverified on load -- the evidence trail survives across runs.
  (let [{:keys [status total accepted rejected]} (persist/report)]
    (is (= :ok status))
    (is (= total accepted))
    (is (zero? rejected)))
  (testing "term persist/reload is content-addressed + integrity-checked; alpha-equal terms share one file"
    (let [dir (str (System/getProperty "java.io.tmpdir") "/pnix-persist-test-" (System/nanoTime))
          ps (persist/open-persistent-store dir)
          a (:ast (parser/parse-source "let a = 1; in a"))
          b (:ast (parser/parse-source "let z = 1; in z"))
          ha (persist/persist-term! ps a)
          hb (persist/persist-term! ps b)]
      (is (= ha hb) "alpha-equivalent terms -> same content address")
      (is (= (persist/load-term ps ha) (cas/canonical-form a)))
      (doseq [f (reverse (file-seq (clojure.java.io/file dir)))] (.delete f))))
  (testing "events persist append-only and reload with an intact chain"
    (let [dir (str (System/getProperty "java.io.tmpdir") "/pnix-persist-ev-" (System/nanoTime))
          ps (persist/open-persistent-store dir)
          mem (store/open-store)]
      (store/append! mem :eval/run {:source-hash "s1"})
      (store/append! mem :eval/run {:source-hash "s2"})
      (persist/persist-events! ps mem)
      (let [{:keys [store verify]} (persist/load-events ps)]
        (is (= 2 (count (store/events store))))
        (is (= :intact (:status verify)))
        (is (= (store/head-hash mem) (store/head-hash store))))
      (doseq [f (reverse (file-seq (clojure.java.io/file dir)))] (.delete f)))))

(deftest cegis-counterexample-guided-refinement
  ;; candidate GENERATOR #2: CEGIS -- the verifier drives the generator. A
  ;; probe divergence vs the reference becomes a counterexample that
  ;; strengthens the examples; on survival arith-proof upgrades to :proven.
  (let [{:keys [status total accepted rejected]} (cegis/report)]
    (is (= :ok status))
    (is (= total accepted))
    (is (zero? rejected)))
  (testing "a colliding seed forces refinement, then converges PROVEN"
    (let [r (cegis/cegis-synthesize {:vars ["x"] :reference "x + 2" :seed-probe 0})]
      (is (= :converged (:status r)))
      (is (> (:iterations r) 1) "the counterexample drove at least one refinement")
      (is (= :proven (:proof-status r)) "arith-proof upgraded the survivor")
      (is (nil? (cegis/counterexample (:candidate r) "x + 2" "x" cegis/default-probes)))))
  (testing "an unreachable reference exhausts honestly (no fabrication)"
    (is (= :exhausted (:status (cegis/cegis-synthesize
                                {:vars ["x"] :reference "x * x + 1"
                                 :max-size 2 :max-iters 4})))))
  (testing "a converged candidate feeds self-improve as a HELD proposal"
    (let [r (cegis/cegis-and-propose (store/open-store)
                                     {:vars ["x"] :reference "2 * x + 3"})]
      (is (= :converged (:status r)))
      (is (:all-held? r) "no auto-promotion"))))

(deftest generate-observational-enumerative-synthesis
  ;; candidate GENERATOR #1: observational-equivalence-reduced bottom-up
  ;; enumeration (Escher). A value-vector match is a HEURISTIC propose, fed to
  ;; self-improve as a HELD proposal. Closes the self-* loop with a real generator.
  (let [{:keys [status total accepted rejected]} (generate/report)]
    (is (= :ok status))
    (is (= total accepted))
    (is (zero? rejected)))
  (let [spec {:vars ["x"]
              :examples [{:in {"x" 1} :out 2} {:in {"x" 2} :out 3} {:in {"x" 3} :out 4}]
              :max-size 3}
        syn (generate/synthesize spec)]
    (testing "synthesizes x+1 from examples, deduped observationally"
      (is (some #(= [2 3 4] (generate/value-vector % (:examples spec))) (:matches syn)))
      (is (< (:classes syn) (:enumerated syn)) "observational dedup shrinks the space"))
    (testing "matches are observational only (reproduce the examples by construction)"
      (is (every? #(= [2 3 4] (generate/value-vector % (:examples spec))) (:matches syn))))
    (testing "generator #3: canonical pre-pruning is SOUND and effective"
      (is (= (:matches syn)
             (:matches (generate/synthesize (assoc spec :canonical-prune? false))))
          "pruning provably changes nothing but the cost")
      (is (pos? (:pruned-proven syn)) "proven duplicates skipped before eval")
      (is (< (:evaluated syn) (:enumerated syn))))
    (testing "matches feed self-improve as HELD proposals (no auto-promotion)"
      (let [s (store/open-store)
            round (generate/synthesize-and-propose s spec)]
        (is (seq (:proposals round)))
        (is (:all-held? round))
        (is (some #(= :admitted (:witness-status %)) (:proposals round)))))))

(deftest self-improve-loop-body-holds-for-owner
  ;; The self-* loop BODY: evaluate candidates, witness + gate each, rank
  ;; best-first. GENERATION-AGNOSTIC; every proposal stays HELD (no
  ;; auto-promotion) -- a review queue for the owner, never auto-applied.
  (let [{:keys [status total accepted rejected]} (self-improve/report)]
    (is (= :ok status))
    (is (= total accepted))
    (is (zero? rejected)))
  (testing "a round evaluates every candidate and holds all under default policy"
    (let [s (store/open-store)
          round (self-improve/evaluate-round
                 s [{:target :a :new-source "(x: x + 1) 41" :rationale "inc"}
                    {:target :b :new-source "x: x" :rationale "bad"}])]
      (is (= 2 (count (:proposals round))))
      (is (:all-held? round) "no auto-promotion")
      (is (= :admitted (:witness-status (:best round))) "best has an admitted witness")
      (is (= :held (:decision (:best round))) "but is still held for the owner")
      (is (= 1 (count (store/events-of s :self-improve/round)))))))

(deftest self-mod-gate-no-auto-promotion
  ;; §14.3: the constitution's NO-AUTO-PROMOTION invariant as a runtime gate.
  ;; An admitted witness is HELD by default; only an explicit owner act promotes.
  (let [{:keys [status total accepted rejected]} (self-mod-gate/report)]
    (is (= :ok status))
    (is (= total accepted))
    (is (zero? rejected)))
  (testing "an admitted self-modification is HELD by default (no auto-promotion)"
    (let [s (store/open-store)
          d (self-mod-gate/propose-and-gate
             s {:target :f :new-source "(x: x + 1) 41" :rationale "t"} :owner-hold)]
      (is (= :held (:decision d)))
      (is (= :no-auto-promotion-owner-required (:reason d)))))
  (testing "only an explicit :owner-authorized policy promotes an admitted witness"
    (let [s (store/open-store)
          d (self-mod-gate/propose-and-gate
             s {:target :f :new-source "(x: x + 1) 41" :rationale "t"} :owner-authorized)]
      (is (= :admitted (:decision d)))))
  (testing "an unknown policy fails closed to :held"
    (let [s (store/open-store)
          p (self-mod-gate/propose! s {:target :f :new-source "(x: x + 1) 41" :rationale "t"})]
      (is (= :held (:decision (self-mod-gate/decide s p :whatever)))))))

(deftest witnessed-run-spine-integration
  ;; SPINE INTEGRATION: one run ties the evidence-store spine to the capability
  ;; pillars -- tower collapse + mirror chain + determinism recorded in ONE §5
  ;; log, keyed by §3 term-hash under an §8 snapshot, admitted by a §15 witness.
  (let [{:keys [status total accepted rejected]} (witnessed-run/report)]
    (is (= :ok status))
    (is (= total accepted))
    (is (zero? rejected)))
  (testing "a deterministic program is ADMITTED with full spine evidence"
    (let [w (witnessed-run/run-witnessed "let x = 40; in x + 2")]
      (is (= :admitted (:status w)))
      (is (= :agree (:collapse w)))
      (is (:chain-converged? w))
      (is (= :ok (:determinism w)))
      (is (:log-intact? w))
      (is (= #{:tower/collapse :mirror/run :purity/run}
             (set (map second (:events w))))
          "one log records the whole pipeline")))
  (testing "the Futamura residual is content-addressed by (term-hash+snapshot)"
    (is (= (:residual-key (witnessed-run/run-witnessed "1 + 2 * 3"))
           (:residual-key (witnessed-run/run-witnessed "1 + 2 * 3")))))
  (testing "a durable witnessed run persists a replayable evidence trail"
    (let [dir (str (System/getProperty "java.io.tmpdir") "/pnix-wr-t-" (System/nanoTime))
          d (witnessed-run/run-witnessed-durable "let x = 40; in x + 2" dir)]
      (is (= :admitted (:status d)))
      (is (string? (get-in d [:persisted :term-hash])))
      (is (pos? (get-in d [:persisted :events-written])))
      (is (= :intact (:status (:verify (persist/load-events
                                        (persist/open-persistent-store dir))))))
      (doseq [f (reverse (file-seq (clojure.java.io/file dir)))] (.delete f)))))

(deftest witness-schema-and-admission-lattice
  ;; §15 (BUILD 8th, CAPSTONE): the witness integrates the whole spine -- it
  ;; binds a result to its term hash (§3), runtime pin (§8), determinism
  ;; evidence events (§5/§9). The admission lattice refuses invalid transitions
  ;; so a result can never be admitted without passing candidacy + evidence.
  (let [{:keys [status total accepted rejected]} (witness/report)]
    (is (= :ok status))
    (is (= total accepted))
    (is (zero? rejected)))
  (testing "a witness binds result to term-hash + snapshot + versions + evidence"
    (let [snap (snapshot/make-snapshot)
          w (witness/witness-eval "let x = 40; in x + 2" :snapshot snap)]
      (is (string? (:witness/id w)))
      (is (string? (:term-hash w)))
      (is (= (:snapshot/id snap) (:snapshot/id w)))
      (is (:evaluator-version w))
      (is (seq (:evidence-events w)))
      (is (= :ok (:status w)))))
  (testing "admission lattice: held -> candidate -> evidence -> admitted"
    (is (= :admitted (:status (witness/admit "1 + 2 * 3"))))
    (is (witness/valid-transition? :held :candidate))
    (is (witness/valid-transition? :candidate :admitted))
    (is (not (witness/valid-transition? :held :admitted)) "cannot skip candidacy")
    (is (:refused-transition
         (witness/status-transition {:status :admitted} :held))
        "terminal status is final")))

(deftest mirror-chain-repeated-run-convergence
  ;; §6.6-6.7 (BUILD 7th): the TEMPORAL axis -- run the same source repeatedly,
  ;; every result must match the first (self-evaluation stability), recorded as
  ;; §5 events; a divergence would be a :mirror/chain-drift pinned to the first
  ;; divergent run (complements the per-run cross-lane collapse).
  (let [{:keys [status total accepted rejected]} (mirror-chain/report)]
    (is (= :ok status))
    (is (= total accepted))
    (is (zero? rejected)))
  (testing "repeated runs converge + record :mirror/run events, chain intact"
    (let [log (store/open-store)
          c (mirror-chain/mirror-chain! "let x = 40; in x + 2" {:runs 5 :store log})]
      (is (:chain-converged? c))
      (is (= 1 (count (store/events-of log :mirror/run))))
      (is (zero? (count (store/events-of log :mirror/chain-drift))))
      (is (= :intact (:status (store/verify-chain log))))))
  (testing "convergence is reflexive on a deterministic source"
    (is (mirror-chain/converge? "({ a }@args: a + args.a) { a = 21; }"))))

(deftest search-content-event-similarity
  ;; §17 (+ §3c): content-address + event + structural-similarity search.
  ;; Similarity (skeleton/distance) is a HEURISTIC proposal, confirmed by the
  ;; §3 exact check -- the same propose-then-confirm discipline as the hash.
  (let [{:keys [status total accepted rejected]} (search/report)]
    (is (= :ok status))
    (is (= total accepted))
    (is (zero? rejected)))
  (let [p #(:ast (parser/parse-source %))]
    (testing "skeleton blanks leaves; same shape != same term"
      (is (= (search/skeleton (p "1 + 2")) (search/skeleton (p "3 + 4"))))
      (is (not (cas/structurally-equivalent? (p "1 + 2") (p "3 + 4")))))
    (testing "free-vars tracks lambda + let binders"
      (is (= #{"y"} (search/free-vars (p "x: x + y"))))
      (is (= #{"z"} (search/free-vars (p "let a = z; in a")))))
    (testing "similar proposes candidates; the exact check confirms"
      (let [hits (search/similar-terms (p "a + b")
                                       [(p "x + y") (p "1 * 2") (p "a + b")] 0.5)]
        (is (= 2 (count hits)) "two same-shape sums proposed")
        (is (= 1 (count (filter :confirmed-equivalent? hits))) "one confirmed")))))

(deftest purity-determinism-as-events
  ;; §9 (BUILD 5th): determinism is WITNESSED by actual re-run + diff (not
  ;; assumed), recorded as §5 events, pinned to the §8 snapshot; nondeterminism
  ;; would fail closed pinned to the first divergent run (the §15 anchor).
  (let [{:keys [status total accepted rejected]} (purity/report)]
    (is (= :ok status))
    (is (= total accepted))
    (is (zero? rejected)))
  (testing "repeated eval is deterministic and recorded as a :purity/run event"
    (let [log (store/open-store)
          r (purity/purity-check! "let x = 40; in x + 2" {:runs 5 :store log})]
      (is (= :ok (:status r)))
      (is (= 1 (count (store/events-of log :purity/run))))))
  (testing "mutation isolation: later unrelated commits do not change the result"
    (is (= :ok (:status (purity/mutation-isolation!
                         "1 + 2 * 3" (snapshot/make-snapshot))))))
  (testing "threaded determinism stress: concurrent evals agree"
    (is (= :ok (:status (purity/threaded-stress "builtins.length [ 1 2 3 ]" 8))))))

(deftest snapshot-runtime-pin-fail-closed
  ;; §8 (BUILD 4th): a snapshot pins the runtime (evaluator + host lane);
  ;; resolve-under-snapshot FAILS CLOSED when the runtime does not match --
  ;; the determinism precondition for content-addressed reuse (Frankenbuild).
  (let [{:keys [status total accepted rejected]} (snapshot/report)]
    (is (= :ok status))
    (is (= total accepted))
    (is (zero? rejected)))
  (testing "snapshot id is a deterministic content hash of the runtime pin"
    (is (= (:snapshot/id (snapshot/make-snapshot))
           (:snapshot/id (snapshot/make-snapshot))))
    (is (snapshot/runtime-matches? (snapshot/make-snapshot))))
  (testing "resolve under a MATCHING snapshot returns the value + snapshot id"
    (let [snap (snapshot/make-snapshot)
          r (snapshot/resolve-under-snapshot "40 + 2" snap)]
      (is (= 42 (:value r)))
      (is (= (:snapshot/id snap) (:snapshot/id r)))))
  (testing "a STALE snapshot fails closed with a precise mismatch reason"
    (let [stale (assoc (snapshot/make-snapshot)
                       :evaluator-version "different-evaluator")
          gate (snapshot/assert-snapshot-runtime-match! stale)
          r (snapshot/resolve-under-snapshot "1 + 2" stale)]
      (is (= :failed (:status gate)))
      (is (= :snapshot-evaluator-version-mismatch (:reason gate)))
      (is (= :failed (:status r)) "resolve refuses under a stale snapshot"))))

(deftest reflect-host-snapshots
  ;; §10 + §13.1 (BUILD 3rd): deterministic Clojure/JVM reflection snapshots --
  ;; the host-varying inputs the §8 snapshot pins. Pure EDN, sorted, no identity.
  (let [{:keys [status total accepted rejected]} (reflect/report)]
    (is (= :ok status))
    (is (= total accepted))
    (is (zero? rejected)))
  (testing "snapshots are deterministic (two calls identical) + pure EDN"
    (is (= (reflect/reflection-snapshot) (reflect/reflection-snapshot)))
    (let [vs (reflect/var-snapshot 'clojure.core/map)]
      (is (= "clojure.core" (:ns vs)))
      (is (false? (:macro vs))))
    (is (:macro (reflect/var-snapshot 'clojure.core/when)) "macro flagged"))
  (testing "classpath + JVM version pin the host lane, stably"
    (is (pos? (:entry-count (reflect/classpath-snapshot))))
    (is (:clojure (reflect/jvm-version-id)))
    (is (= (reflect/host-lane-id) (reflect/host-lane-id))))
  (testing "namespace-diff"
    (is (= {:added ["c"] :removed ["a"]}
           (reflect/namespace-diff ["a" "b"] ["b" "c"])))))

(deftest store-append-only-event-log
  ;; §5 (BUILD 2nd): append-only, tamper-evident (hash-chain) event log = a
  ;; verifying trace. Values stay in §3/cached-eval; the log holds hashes +
  ;; pure-EDN, rejecting hermeticity contamination.
  (let [{:keys [status total accepted rejected]} (store/report)]
    (is (= :ok status))
    (is (= total accepted))
    (is (zero? rejected)))
  (testing "append-only + monotonic seq + tamper-evident chain"
    (let [s (store/open-store)]
      (store/append! s :eval/run {:source-hash "a" :result-hash "1"})
      (store/append! s :eval/run {:source-hash "b" :result-hash "2"})
      (is (= 2 (count (store/events s))))
      (is (= :intact (:status (store/verify-chain s))))))
  (testing "pointer movement is itself an event, folded to latest"
    (let [s (store/open-store)]
      (store/set-pointer! s :head "h1")
      (store/set-pointer! s :head "h2")
      (is (= "h2" (store/get-pointer s :head)))
      (is (= 2 (count (store/events-of s :pointer/moved))))))
  (testing "hermeticity: contamination is rejected, log does not grow"
    (let [s (store/open-store)]
      (is (= :rejected (:status (store/append! s :x {:t (java.util.Date.)}))))
      (is (= :rejected (:status (store/append! s :x {:f (fn [] 1)}))))
      (is (zero? (count (store/events s)))))))

(deftest cas-content-addressed-term-store
  ;; §3 (BUILD 1st of the evidence-store spine): canonical normalization +
  ;; content-addressed term store. THE principle: a content hash is a PROPOSE
  ;; filter, CONFIRMED by exact structural equality -- never trusted alone.
  (let [{:keys [status total accepted rejected hash-agrees-with-structural?
                store-roundtrip-ok?]} (cas/report)]
    (is (= :ok status))
    (is (= total accepted))
    (is (zero? rejected))
    (is hash-agrees-with-structural?)
    (is store-roundtrip-ok?))
  (testing "order-independent binder groups canonicalize identically"
    (let [a (:ast (parser/parse-source "{ a = 1; b = 2; }"))
          b (:ast (parser/parse-source "{ b = 2; a = 1; }"))
          c (:ast (parser/parse-source "let a = 1; b = 2; in a + b"))
          d (:ast (parser/parse-source "let b = 2; a = 1; in a + b"))]
      (is (cas/structurally-equivalent? a b) "attrset order")
      (is (= (cas/term-hash a) (cas/term-hash b)))
      (is (cas/structurally-equivalent? c d) "recursive let order")))
  (testing "distinct terms are NOT merged; the store dedups + confirms"
    (cas/clear-store!)
    (is (= :stored (:status (cas/put-term! (:ast (parser/parse-source "{ a = 1; b = 2; }"))))))
    (is (= :hit (:status (cas/put-term! (:ast (parser/parse-source "{ b = 2; a = 1; }"))))))
    (is (= :stored (:status (cas/put-term! (:ast (parser/parse-source "{ a = 2; }"))))))
    (is (= 2 (cas/term-count))))
  (testing "§3b ALPHA-equivalence: alpha-renamed terms dedup, with shadowing"
    (is (cas/alpha-equivalent? (:ast (parser/parse-source "x: x"))
                               (:ast (parser/parse-source "y: y"))))
    (is (cas/alpha-equivalent? (:ast (parser/parse-source "let a = 1; in a"))
                               (:ast (parser/parse-source "let z = 1; in z"))))
    (is (cas/alpha-equivalent? (:ast (parser/parse-source "x: (x: x)"))
                               (:ast (parser/parse-source "y: (z: z)")))
        "correct shadowing")
    (is (not (cas/alpha-equivalent? (:ast (parser/parse-source "x: x + y"))
                                    (:ast (parser/parse-source "z: z + w"))))
        "free vars kept -- distinct")
    (is (not (cas/alpha-equivalent? (:ast (parser/parse-source "{ a = 1; }"))
                                    (:ast (parser/parse-source "{ z = 1; }"))))
        "attrset labels observable -- kept")
    (cas/clear-store!)
    (cas/put-term! (:ast (parser/parse-source "x: x")))
    (is (= :hit (:status (cas/put-term! (:ast (parser/parse-source "y: y")))))
        "store dedups alpha-equivalent lambdas"))
  (testing "the CAS purity guard rejects non-EDN contamination"
    (is (cas/pure-term? {:op :int :value 1}))
    (is (not (cas/pure-term? {:op :fn :f (fn [] 1)})))
    (is (not (cas/pure-term? (atom 1))))))

(deftest arith-proof-proven-equivalence
  ;; PROVEN (not tested) equivalence for the arithmetic fragment via canonical
  ;; polynomial normalization -- upgrades specialize (M1) soundness to a proof
  ;; over ALL variable values, honest :unprovable on non-arithmetic residuals.
  (let [{:keys [status total accepted rejected rows]} (arith-proof/report)]
    (is (= :ok status))
    (is (= total accepted))
    (is (zero? rejected))
    (is (every? #(= :proven (:proof-status %)) rows)))
  (testing "algebraic identities are PROVEN, non-equivalences refuted"
    (is (arith-proof/equivalent? "(x + 1) * (x - 1)" "x * x - 1"))
    (is (arith-proof/equivalent? "2 * x + 3 * x" "5 * x"))
    (is (arith-proof/equivalent? "x + y" "y + x"))          ; commutativity
    (is (arith-proof/equivalent? "(x + y) + z" "x + (y + z)")) ; associativity
    (is (not (arith-proof/equivalent? "x + 1" "x + 2")))
    (is (not (arith-proof/equivalent? "x * x" "x"))))
  (testing "specialize meaning is PROVEN for arithmetic, ALL dynamic values"
    (let [r (arith-proof/prove-specialize-meaning "a * x + b" {"a" 4 "b" 10})]
      (is (= :proven (:status r)))
      (is (= ["x"] (:dynamic-vars r)))))
  (testing "honest boundary: a non-arithmetic residual is :unprovable, not false"
    (let [r (arith-proof/prove-specialize-meaning "if (x < 1) then x else 0" {})]
      (is (= :unprovable (:status r))))))

(deftest form-analysis-ast-pass-lane
  ;; deep-research F4: a Clojure-on-Clojure AST-pass substrate over
  ;; tools.analyzer.jvm (a manipulable AST Python/Hy has no equivalent of).
  ;; Classifies a Clojure form's static host-interop surface (pure-core verdict)
  ;; and structures unresolvable forms as :held rather than throwing.
  (let [{:keys [status total accepted rejected]} (form-analysis/report)]
    (is (= :ok status))
    (is (= total accepted))
    (is (zero? rejected)))
  (testing "pure numeric/core forms are pure-core (no host surface)"
    (let [a (form-analysis/analyze-form '(+ (* 2 3) 4))]
      (is (= :ok (:status a)))
      (is (:pure-core? a))
      (is (empty? (:host-interop a)))))
  (testing "host interop is surfaced with class + method"
    (let [a (form-analysis/analyze-form '(.toUpperCase "hi"))]
      (is (= :ok (:status a)))
      (is (false? (:pure-core? a)))
      (is (= 1 (count (:host-interop a))))
      (is (= "java.lang.String" (:class (first (:host-interop a)))))))
  (testing "a `new` is host interop, arithmetic static-calls are not"
    (is (false? (:pure-core? (form-analysis/analyze-form '(java.util.ArrayList.)))))
    (is (true? (:pure-core? (form-analysis/analyze-form '(reduce + 0 [1 2 3]))))))
  (testing "an unresolvable form is a structured analysis failure"
    (let [a (form-analysis/analyze-form 'definitely-not-a-var)]
      (is (= :failed (:status a)))
      (is (= :form-does-not-analyze (:reason a))))))

(deftest synthesize-form-analysis-convergence
  ;; Cross-capability soundness (converge domains): synthesize (M3) projects a
  ;; whitelisted Clojure CORE to pnix; form-analysis (F4) independently, via the
  ;; JVM analyzer, classifies a form's host-interop surface. They must agree --
  ;; a form synthesize projects to PURE pnix must be host-free, and a
  ;; host-touching form must be refused. Two independent judgments pinning each
  ;; other; a divergence would be a real soundness bug in one of them.
  (testing "synthesize ACCEPTS => form-analysis says pure-core (host-free)"
    (doseq [{:keys [id form]} synthesize/cases]
      (let [s (synthesize/form->pnix form)
            a (form-analysis/analyze-form form)]
        (is (= :ok (:status s)) (str id " projects"))
        (is (= :ok (:status a)) (str id " analyzes"))
        (is (:pure-core? a)
            (str id " projectable-to-pnix form must be host-free; host="
                 (pr-str (:host-interop a)))))))
  (testing "host-interop => form-analysis flags it AND synthesize refuses"
    (doseq [form ['(.toUpperCase "hi") '(java.util.ArrayList.)
                  '(System/getProperty "user.home")]]
      (let [a (form-analysis/analyze-form form)
            s (synthesize/form->pnix form)]
        (is (false? (:pure-core? a)) (str (pr-str form) " has host interop"))
        (is (not= :ok (:status s))
            (str (pr-str form) " must not project to pure pnix"))))))

(deftest futamura-second-projection
  ;; Roadmap: the Futamura ladder. gen (2nd projection) is the program-
  ;; agnostic pnix->JVM-bytecode compiler; cogen (3rd projection, F7) is the
  ;; COMPILER GENERATOR, both built the cogen-free way (currying, Latifi
  ;; DLS'19); Gluck PEPM'09's collapse (a 4th projection yields nothing new)
  ;; is mechanized for the curried construction, and the classical
  ;; spec(spec,spec) self-application stays an explicit HELD boundary (F7b).
  (let [{:keys [status total accepted rejected rows
                compiler-fixed-across-programs?
                first-projection-specialization-varies?
                distinct-compiler-ids distinct-first-projection-residuals
                cogen-fixed-across-programs? cogen-extensions-vary-per-program?
                fourth-projection-collapse
                third-projection] :as rep} (futamura/report)]
    (is (= :ok status))
    (is (= total accepted))
    (is (zero? rejected))
    (testing "the ladder agrees: interp == P1 == P2 == P3"
      (is (every? #(= (:direct-value %) (:first-projection-value %)) rows))
      (is (every? #(= (:first-projection-value %) (:second-projection-value %)) rows))
      (is (every? #(= (:second-projection-value %) (:third-projection-value %)) rows))
      (is (every? #(= :ok (:second-projection-bytecode-determinism %)) rows)))
    (testing "gen is a COMPILER: one compiler-id across every program"
      (is compiler-fixed-across-programs?)
      (is (= 1 distinct-compiler-ids)))
    (testing "the 1st projection is a per-program SPECIALIZATION: residual varies"
      (is first-projection-specialization-varies?)
      (is (> distinct-first-projection-residuals 1)))
    (testing "cogen is a COMPILER GENERATOR: one cogen-id, per-program extensions"
      (is cogen-fixed-across-programs?)
      (is cogen-extensions-vary-per-program?))
    (testing "Gluck collapse: re-deriving the generator yields nothing new"
      (is (:all-agree? fourth-projection-collapse))
      (is (:cogen-id-stable? fourth-projection-collapse))
      (is (= :by-construction-for-curried-route
             (:proof-kind fourth-projection-collapse))
          "honest label: curried-route theorem, not classical self-application"))
    (testing "3rd projection built (curried); self-application HELD with proof anchor"
      (is (= :built-curried-route (:status third-projection)))
      (is (= :f7b-self-applicable-specializer
             (get-in third-projection [:held-boundary :tracked])))
      (is (= :genuine-proof-not-heuristic
             (get-in third-projection [:proof-anchor :kind]))))
    (testing "Jones-optimality: measured, no interpreter floor (F2)"
      (let [w (futamura/jones-optimality-witness)]
        (is (:bounded? w))
        (is (= :jones-optimal-no-interpreter-floor (:verdict w)))
        ;; compiled form scales WITH the program (grows as ops are added)
        (is (apply < (map :compiled-form-size (:rows w)))))))
  ;; the generating extension is reusable: the SAME gen compiles different
  ;; programs, and each compiled artifact agrees with direct evaluation.
  (let [gen (futamura/generating-extension)
        run (fn [src env] (get-in ((:compile gen) src env) [:invoked :value]))]
    (is (= 7 (run "a + b" {"a" 3 "b" 4})))
    (is (= 20 (run "x * y" {"x" 4 "y" 5})))
    (is (= (:compiler-id gen) (:compiler-id (futamura/generating-extension)))
        "gen is a fixed compiler -- deterministic compiler-id"))
  ;; the compiler GENERATOR is reusable: one cogen yields per-program
  ;; generating extensions whose compiled artifacts agree with direct eval.
  (let [cg (futamura/cogen)
        gen-a ((:generate cg) "a + b")
        gen-m ((:generate cg) "x * y")]
    (is (= 7 (get-in ((:compile gen-a) {} {"a" 3 "b" 4}) [:invoked :value])))
    (is (= 20 (get-in ((:compile gen-m) {} {"x" 4 "y" 5}) [:invoked :value])))
    (is (not= (:extension-id gen-a) (:extension-id gen-m))
        "per-program extensions -- a generator, not one compiler")
    (is (= (:cogen-id cg) (:cogen-id (futamura/cogen)))
        "cogen is fixed -- deterministic cogen-id")))

(deftest specialize-futamura-first-slice
  ;; Roadmap M1: partial evaluator. The report runs differential verification
  ;; per case: eval(residual, dynamics) == eval(source, statics+dynamics).
  (let [{:keys [status total accepted rejected rows futamura-rows]} (specialize/report)]
    (is (= :ok status))
    (is (= total accepted))
    (is (zero? rejected))
    (is (every? :meaning-preserved? rows))
    (testing "futamura projection: residual -> lowering -> clj-meta bytecode"
      (is (pos? (count futamura-rows)))
      (is (every? #(= :accepted (:status %)) futamura-rows))
      (is (every? #(= :ok (:bytecode-determinism %)) futamura-rows))))
  (testing "recursive-let folding (pnix-hy A4 lessons, same-language residual)"
    (let [sp (specialize/specialize "let x = 5; in let y = x + 1; x = 10; in y" {})]
      (is (:fully-static? sp))
      (is (= 11 (:value sp))))
    (let [sp (specialize/specialize "let b = a + 1; a = 2; in b" {})]
      (is (= 3 (:value sp)))))
  (testing "static substitution + if pruning drops the dead branch"
    (let [sp (specialize/specialize "if flag then a + 1 else a - 1" {"flag" true})]
      (is (not (:fully-static? sp)))
      (is (not (str/includes? (:residual-source sp) "else")))
      (is (not (str/includes? (:residual-source sp) "flag")))))
  (testing "a fully-applied call folds capture-free (x=100 static would give 101)"
    (let [sp (specialize/specialize "(x: x + 1) 5" {"x" 100})]
      (is (:fully-static? sp))
      (is (= 6 (:value sp)))))
  (testing "builtin and higher-order calls fold under fuel"
    (is (= 3 (:value (specialize/specialize "builtins.length [ 1 2 3 ]" {}))))
    (is (= 10 (:value (specialize/specialize
                       "builtins.foldl' (a: b: a + b) 0 [ 1 2 3 4 ]" {})))))
  (testing "a divergent fold burns fuel into a gap, not a hang"
    (let [sp (specialize/specialize "let f = x: f x; in f 1" {})]
      (is (not (:fully-static? sp)))
      (is (some #(= :fold-fuel-exhausted (:reason %)) (:gaps sp)))
      (is (= specialize/default-fold-fuel (:fold-fuel sp)))))
  (testing "per-call :fold-fuel option is honored and cache-key distinguishing"
    (let [src "let f = x: f x; in f 1"
          low (specialize/specialize src {} {:fold-fuel 16})
          def (specialize/specialize src {})]
      (is (= 16 (:fold-fuel low)))
      (is (some #(and (= :fold-fuel-exhausted (:reason %))
                      (= 16 (:fuel %)))
                (:gaps low)))
      (is (= specialize/default-fold-fuel (:fold-fuel def))))
    (specialize/clear-specialize-cache!)
    (let [src "builtins.length [ 1 2 3 ]"
          a (specialize/specialize-cached src {} {:fold-fuel 64})
          b (specialize/specialize-cached src {} {:fold-fuel 64})
          c (specialize/specialize-cached src {} {:fold-fuel 128})]
      (is (= :miss (get-in a [:cache :status])))
      (is (= :hit (get-in b [:cache :status])))
      (is (= :miss (get-in c [:cache :status])) "different fold-fuel differs")
      (is (= 3 (:value a) (:value b) (:value c)))))
  (testing "partial select picks a static entry from a mixed attrset"
    (let [sp (specialize/specialize "{ s = a + 1; d = y; }.s" {"a" 1})]
      (is (= 2 (:value sp)))))
  (testing "specialize-cached: content-addressed, statics-distinguishing, harmless"
    (specialize/clear-specialize-cache!)
    (let [r1 (specialize/specialize-cached "let x = 40; in x + d" {"d" 2})
          r2 (specialize/specialize-cached "let x = 40; in x + d" {"d" 2})
          r3 (specialize/specialize-cached "let x = 40;   in x + d" {"d" 2})
          r4 (specialize/specialize-cached "let x = 40; in x + d" {"d" 3})]
      (is (= :miss (get-in r1 [:cache :status])))
      (is (= :hit (get-in r2 [:cache :status])))
      (is (= :hit (get-in r3 [:cache :status])) "formatting variants share")
      (is (= :miss (get-in r4 [:cache :status])) "different statics differ")
      (is (= (dissoc r1 :cache) (dissoc r2 :cache) (dissoc r3 :cache))))
    (is (= :static-env-not-data
           (get-in (specialize/specialize-cached "x" {"x" (fn [])})
                   [:cache :reason]))))
  (testing "non-bool static if condition records a gap, never folds (A15)"
    (let [sp (specialize/specialize "if 1 then 2 else 3" {})]
      (is (not (:fully-static? sp)))
      (is (some #(= :if-non-bool-condition (:reason %)) (:gaps sp)))))
  (testing "non-data static env is held"
    (is (= :static-env-not-data
           (:reason (specialize/specialize "x" {"x" (fn [])})))))
  (testing "reusable host artifact: one compile, many dynamics (application equality)"
    (let [art (specialize/specialize-to-host-artifact
               "x + y" {"x" 40} ["y"])]
      (is (= :ok (:status art)))
      (is (fn? (:fn art)))
      (is (= ["y"] (:dynamic-names art)))
      (let [a (specialize/invoke-host-artifact art {"y" 2})
            b (specialize/invoke-host-artifact art {"y" 3})
            c (specialize/invoke-host-artifact art {"y" 2})]
        (is (= :ok (:status a) (:status b) (:status c)))
        (is (= 42 (:value a)))
        (is (= 43 (:value b)))
        (is (= 42 (:value c)) "same artifact reused")))
    (let [art (specialize/specialize-to-host-artifact
               "if flag then a + 1 else a - 1" {"flag" true} ["a"])
          r (specialize/invoke-host-artifact art {"a" 10})]
      (is (= :ok (:status art)))
      (is (= 11 (:value r))))
    (let [art (specialize/specialize-to-host-artifact
               "builtins.length [ 1 2 3 ]" {} [])
          r (specialize/invoke-host-artifact art {})]
      (is (= :ok (:status art)))
      (is (= 3 (:value r))))
    (let [art (specialize/specialize-to-host-artifact "x + y" {"x" 40} ["y"])]
      (is (= :ok (:status art)))
      (is (= :missing-dynamic
             (:reason (specialize/invoke-host-artifact art {}))))
      (is (= :dynamics-not-data
             (:reason (specialize/invoke-host-artifact art {"y" (fn [])}))))
      (is (= :ok (:status (specialize/invoke-host-artifact art {"y" 0}))))
      (is (= 40 (:value (specialize/invoke-host-artifact art {"y" 0})))))))

(deftest evaluator-string-context-core
  ;; Pure simulation of Nix string context (Completeness Roadmap item 3, first
  ;; slice). Context-free strings stay plain JVM Strings (zero change); a
  ;; contextful string is the tagged {"__pnix_value_kind" "string-context"} map,
  ;; created today by builtins.appendContext (derivations come next). Consumers
  ;; that are not context-aware are DENIED BY DEFAULT
  ;; (held :string-context-frontier) so context is never silently dropped.
  (let [mk (str "builtins.appendContext \"a\""
                " { \"/nix/store/p1\" = { path = true; }; }")]
    (testing "plain strings have no context"
      (is (= false (:value (pnix/eval-source "builtins.hasContext \"x\""))))
      (is (= {} (:value (pnix/eval-source "builtins.getContext \"x\"")))))
    (testing "appendContext creates a contextful string"
      (is (= {"__pnix_value_kind" "string-context"
              "string" "a"
              "context" ["/nix/store/p1"]}
             (:value (pnix/eval-source mk))))
      (is (= true (:value (pnix/eval-source
                           (str "builtins.hasContext (" mk ")")))))
      (is (= {"/nix/store/p1" {"path" true}}
             (:value (pnix/eval-source
                      (str "builtins.getContext (" mk ")"))))))
    (testing "unsafeDiscardStringContext strips context, keeps content"
      (is (= "a" (:value (pnix/eval-source
                          (str "builtins.unsafeDiscardStringContext (" mk ")")))))
      (is (= false (:value (pnix/eval-source
                            (str "builtins.hasContext"
                                 " (builtins.unsafeDiscardStringContext (" mk "))"))))))
    (testing "+ concatenates content and unions context"
      (is (= {"/nix/store/p1" {"path" true} "/nix/store/p2" {"path" true}}
             (:value (pnix/eval-source
                      (str "builtins.getContext ((" mk ")"
                           " + (builtins.appendContext \"b\""
                           " { \"/nix/store/p2\" = { path = true; }; }))")))))
      (is (= "a!" (:value (pnix/eval-source
                           (str "builtins.unsafeDiscardStringContext ((" mk ") + \"!\")"))))))
    (testing "string interpolation carries context through the template"
      (is (= {"__pnix_value_kind" "string-context"
              "string" "xay"
              "context" ["/nix/store/p1"]}
             (:value (pnix/eval-source (str "let s = " mk "; in \"x${s}y\""))))))
    (testing "toString keeps context"
      (is (= true (:value (pnix/eval-source
                           (str "builtins.hasContext (builtins.toString (" mk "))"))))))
    (testing "a contextful string is a string, not an attrset"
      (is (= "string" (:value (pnix/eval-source (str "builtins.typeOf (" mk ")")))))
      (is (= true (:value (pnix/eval-source (str "builtins.isString (" mk ")")))))
      (is (= false (:value (pnix/eval-source (str "builtins.isAttrs (" mk ")"))))))
    (testing "equality compares content only (context ignored, like Nix)"
      (is (= true (:value (pnix/eval-source (str "(" mk ") == \"a\"")))))
      (is (= false (:value (pnix/eval-source (str "(" mk ") == \"b\""))))))
    (testing "non-context-aware builtins are held, not silently wrong"
      ;; stringLength/substring/concatStringsSep graduated to the allowlist in
      ;; the context-aware batch (see evaluator-context-aware-string-builtins);
      ;; these two are still at the frontier.
      ;; graduated so far: length/predicates/substring/case/concat/replace/
      ;; match/split/toJSON/toString/stringToCharacters/splitString/fromJSON +
      ;; structural list ops; these remain at the frontier.
      (is (= :string-context-frontier
             (:reason (pnix/eval-source (str "builtins.baseNameOf (" mk ")")))))
      (is (= :string-context-frontier
             (:reason (pnix/eval-source
                       (str "let s = " mk "; in builtins.concatMapStringsSep \",\" (x: x) [ s ]"))))))
    (testing "argument type errors are held"
      (is (= :has-context-argument-not-string
             (:reason (pnix/eval-source "builtins.hasContext 5"))))
      (is (= :append-context-context-not-attrset
             (:reason (pnix/eval-source "builtins.appendContext \"a\" 5")))))
    (testing "context-free behavior is unchanged (plain String results)"
      (is (= "ab" (:value (pnix/eval-source "\"a\" + \"b\""))))
      (is (string? (:value (pnix/eval-source "\"a\" + \"b\""))))
      ;; Phase D: string + non-string is a TYPE ERROR (the old "a1" was a
      ;; Clojure host leak, removed 2026-07-07)
      (is (= :string-coercion (:reason (pnix/eval-source "\"a\" + 1"))))
      (is (string? (:value (pnix/eval-source "let s = \"q\"; in \"pre ${s} post\"")))))))

(deftest evaluator-strict-audit-records-without-changing-behavior
  ;; Phase A recorded what WOULD fail under strict typing; since Phase D
  ;; (2026-07-07, owner doctrine: truthiness was a Clojure host leak) strict
  ;; IS the semantics, so the audited constructs also HOLD -- the audit
  ;; events and the held reasons must line up one-to-one.
  (let [audit (fn [src] (pnix/eval-source-strict-audit src))]
    (testing "non-bool if/assert/! and string+non-string + are recorded AND held"
      (let [{:keys [result strict-violations]} (audit "if 5 then 1 else 2")]
        (is (= :non-bool-if-condition (:reason result)))
        (is (= [:if] (map :construct strict-violations)))
        (is (= :non-bool-condition (:issue (first strict-violations))))
        (is (= :int (:value-type (first strict-violations)))))
      (let [{:keys [result strict-violations]} (audit "!5")]
        (is (= :non-bool-not-operand (:reason result)))
        (is (= [:not] (map :construct strict-violations))))
      (let [{:keys [result strict-violations]} (audit "assert 5; 42")]
        (is (= :non-bool-assert-condition (:reason result)))
        (is (= [:assert] (map :construct strict-violations))))
      (let [{:keys [result strict-violations]} (audit "\"a\" + 1")]
        (is (= :string-coercion (:reason result)))
        (is (= :string-coercion (:issue (first strict-violations))))
        (is (= :string (:left-type (first strict-violations))))
        (is (= :int (:right-type (first strict-violations)))))
      (let [{:keys [strict-violations]} (audit "builtins.stringLength [1 2]")]
        (is (= :builtin (:construct (first strict-violations))))
        (is (= :stringLength (:builtin (first strict-violations))))
        (is (= :non-string-argument (:issue (first strict-violations)))))
      (let [{:keys [strict-violations]} (audit "builtins.concatStringsSep \"-\" [1 \"a\"]")]
        (is (= :non-string-list-element (:issue (first strict-violations))))
        (is (= 0 (:index (first strict-violations)))))
      (let [{:keys [strict-violations]} (audit "builtins.substring (-1) 2 \"abc\"")]
        (is (= :substring (:builtin (first strict-violations))))
        (is (= :negative-start (:issue (first strict-violations)))))
      (let [{:keys [strict-violations]} (audit "1 - \"x\"")]
        (is (= :binary (:construct (first strict-violations))))
        (is (= :non-number-operand (:issue (first strict-violations))))))
    (testing "type-correct operations record nothing"
      (doseq [src ["if true then 1 else 2" "!false" "\"a\" + \"b\"" "1 + 2"]]
        (is (empty? (:strict-violations (audit src))) (str "clean: " src))))
    (testing "default eval-source holds the same constructs (Phase D)"
      (is (= :non-bool-if-condition (:reason (pnix/eval-source "if 5 then 1 else 2"))))
      (is (= :string-coercion (:reason (pnix/eval-source "\"a\" + 1")))))))

(deftest evaluator-strict-mode-holds-audited-constructs
  ;; Phase C: strict mode is opt-in. It turns the Phase-A audit events into
  ;; held errors while leaving default eval-source lenient.
  (doseq [[source reason] [["if 5 then 1 else 2" :non-bool-if-condition]
                           ["!5" :non-bool-not-operand]
                           ["assert 5; 42" :non-bool-assert-condition]
                           ["\"a\" + 1" :string-coercion]
                           ["builtins.stringLength [1 2]" :string-builtin-non-string]
                           ["builtins.concatStringsSep \"-\" [1 \"a\"]"
                            :string-list-builtin-non-string-element]
                           ["builtins.substring (-1) 2 \"abc\"" :substring-negative-start]
                           ["1 - \"x\"" :arithmetic-non-number]]]
    (let [strict-result (pnix/eval-source-strict source)]
      (is (= :failed (:status strict-result)) source)
      (is (= reason (:reason strict-result)) source)))
  (doseq [source ["if true then 1 else 2" "!false" "assert true; 42"
                  "\"a\" + \"b\"" "1 + 2"]]
    (is (= :ok (:status (pnix/eval-source-strict source))) source))
  ;; Phase D: the default holds identically -- strict IS pnix's semantics
  (is (= :non-bool-if-condition (:reason (pnix/eval-source "if 5 then 1 else 2"))))
  (is (= :string-coercion (:reason (pnix/eval-source "\"a\" + 1")))))

(deftest strict-audit-report-classifies-current-corpus
  ;; Phase B: classify the current fixture corpus without changing behavior.
  ;; The full report also includes repo-owned runtime .px files; this test keeps
  ;; the gate fast by evaluating fixture sources and separately checking that the
  ;; runtime inventory is wired into the default source set.
  (let [fixture-report (strict-audit/report {:include-runtime? false})
        all-source-rows (strict-audit/source-rows)]
    (is (= :strict-audit-report (:kind fixture-report)))
    (is (= :audit-only-no-behavior-change (:policy fixture-report)))
    (is (= 245 (:source-count fixture-report)))
    (is (= 238 (:strict-ok fixture-report)))
    (is (= 0 (:strict-violation fixture-report)))
    (is (= 7 (:held fixture-report)))
    (is (= 0 (:violation-count fixture-report)))
    (is (= {:ground-truth-oracle 20
            :mirror-pair 199
            :mirror-error 4
            :stage7-core 5
            :import-module 1
            :forward-reference 6
            :rust-grounded 10}
           (:source-family-counts fixture-report)))
    (is (= 278 (count all-source-rows)))
    (is (some #(= :px-runtime (:source-family %)) all-source-rows))))

(deftest strict-gate-runs-current-strict-ok-fixtures
  (let [gate (strict-audit/strict-gate-report {:include-runtime? false})]
    (is (= :strict-gate-report (:kind gate)))
    (is (= 245 (:classified-source-count gate)))
    (is (= 238 (:strict-ok-source-count gate)))
    (is (= 238 (:checked gate)))
    (is (= 238 (:ok gate)))
    (is (= 0 (:failed gate)))
    (is (nil? (:first-failed gate)))))

(deftest pnix-evaluation-determinism-report-hashes-current-corpus
  (let [report (determinism/report {:runs 2 :include-runtime? false})]
    (is (= :pnix-evaluation-determinism-report (:kind report)))
    (is (= :pnix-clj.evaluation-determinism.v0 (:schema report)))
    (is (= :repeat-parse-eval-hash-stability (:policy report)))
    (is (= 2 (:runs-per-source report)))
    (is (= 245 (:source-count report)))
    (is (= 245 (:stable report)))
    (is (= 0 (:unstable report)))
    (is (nil? (:first-unstable report)))
    (is (= {:ground-truth-oracle 20
            :mirror-pair 199
            :mirror-error 4
            :stage7-core 5
            :import-module 1
            :forward-reference 6
            :rust-grounded 10}
           (:source-family-counts report)))
    (is (every? :stable? (:rows report)))
    (is (every? #(= 2 (count (:sample-hashes %))) (:rows report)))
    (is (every? #(every? (fn [h] (= 64 (count h))) (:sample-hashes %))
                (:rows report)))))

(deftest pnix-evaluation-coverage-report-measures-current-corpus
  (let [report (coverage/report {:include-runtime? false})]
    (is (= :pnix-evaluation-coverage-report (:kind report)))
    (is (= :pnix-clj.evaluation-coverage.v0 (:schema report)))
    (is (= :dynamic-evaluator-coverage (:policy report)))
    (is (= 245 (:source-count report)))
    (is (pos? (get-in report [:summary :op :covered])))
    (is (pos? (get-in report [:summary :builtin :covered])))
    (is (pos? (get-in report [:summary :binary-operator :covered])))
    (is (pos? (get-in report [:summary :branch :covered])))
    (is (<= (get-in report [:summary :op :covered])
            (get-in report [:summary :op :total])))
    (is (contains? (get-in report [:totals :op]) :int))
    (is (contains? (get-in report [:totals :op]) :binary))
    (is (contains? (get-in report [:totals :builtin]) :length))
    (is (contains? (get-in report [:totals :branch]) :if/then))
    (is (every? #(contains? % :coverage-event-counts) (:rows report)))))

(deftest grammar-fuzzer-differential-gate-runs-generated-programs
  (let [report (grammar-fuzzer/report {:positive-count 5
                                       :error-count 2
                                       :seed 0})]
    (is (= :pnix-grammar-fuzzer-report (:kind report)))
    (is (= :pnix-clj.grammar-fuzzer-report.v0 (:schema report)))
    (is (= :generated-programs-through-run-source-differential-gate
           (:policy report)))
    (is (= 7 (:source-count report)))
    (is (= 5 (:positive-count report)))
    (is (= 2 (:error-count report)))
    (is (= 7 (:ok report)))
    (is (= 0 (:failed report)))
    (is (nil? (:first-failed report)))
    (is (= {:accepted 5 :held 2} (:actual-status-counts report)))
    (is (every? #(= :ok (:status %)) (:rows report)))))

(deftest optional-live-oracle-is-gated-and-compares-when-available
  (let [skipped (live-oracle/report {:discover? false})
        fake-oracle (fn [source]
                      (select-keys (pnix/eval-source source)
                                   [:status :reason :value]))
        compared (live-oracle/report {:positive-count 3
                                      :seed 0
                                      :oracle-fn fake-oracle})]
    (is (= :pnix-live-oracle-report (:kind skipped)))
    (is (= :skipped (:status skipped)))
    (is (= :live-oracle-command-not-found (:reason skipped)))
    (is (= :pnix-live-oracle-report (:kind compared)))
    (is (= :pnix-clj.live-oracle-report.v0 (:schema compared)))
    (is (= :optional-reference-nix-json-oracle (:policy compared)))
    (is (= :ok (:status compared)))
    (is (= 3 (:source-count compared)))
    (is (= 3 (:matched compared)))
    (is (= 0 (:mismatched compared)))
    (is (= 0 (:pnix-held compared)))
    (is (= 0 (:oracle-held compared)))))

(deftest forward-reference-frontier-corpus
  ;; The reclassified home (see rec-forward-reference-taxonomy.md) for rec/let
  ;; forward references. R1 lifted the valid forward-reference rows across the
  ;; clj-meta and .px runtime lanes; deterministic error rows fail.
  (let [{:keys [cases lifted-lanes]}
        (edn/read-string
         (slurp (io/resource "pnix_clj/forward_reference/cases.edn")))]
    (is (seq cases))
    (doseq [{:keys [name source class expected-eval]} cases]
      (testing name
        (let [row (pnix/verify-source source)
              eval-result (:eval-result row)
              lane-by (into {} (map (juxt :lane identity) (:lane-summary row)))]
          (is (= (:status expected-eval) (:status eval-result))
              (str name ": evaluator status"))
          (when (contains? expected-eval :value)
            (is (= (:value expected-eval) (:value eval-result))
                (str name ": evaluator value")))
          (when (:reason expected-eval)
            (is (= (:reason expected-eval) (:reason eval-result))
                (str name ": evaluator reason")))
          (if (= :forward-ok class)
            (do
              (is (= :accepted (:status row))
                  (str name ": all lanes accepted after R1 lift"))
              (is (= :all-lanes-agree (:reason row))
                  (str name ": all lanes agree after R1 lift"))
              (doseq [lane lifted-lanes]
                (is (= :ok (:status (lane-by lane)))
                    (str name ": lifted lane " lane " must be ok")))
              (is (= (:value expected-eval)
                     (get-in row [:clj-meta-result :value]))
                  (str name ": clj-meta value"))
              (is (= (:value expected-eval)
                     (get-in row [:px-runtime :value]))
                  (str name ": px-runtime value")))
            (doseq [lane lifted-lanes]
              (is (= :failed (:status (lane-by lane)))
                  (str name ": semantic error lane " lane " must fail")))))))))

(deftest forward-reference-report-records-r1-lift
  (let [report (report-artifact/report-for :forward-reference)
        rows (:rows report)
        row-by-id (into {} (map (juxt :source-id identity) rows))]
    (is (= :forward-reference-lift-report (:kind report)))
    (is (= :forward-reference-lift-fixture-set (:fixture-kind report)))
    (is (= 6 (:fixture-count report)))
    (is (= 6 (:accepted report)))
    (is (= 0 (:held report)))
    (is (= 0 (:rejected report)))
    (is (= 3 (:forward-ok-count report)))
    (is (= 3 (:semantic-error-count report)))
    (is (= {:forward-reference-contract-satisfied 6}
           (:reason-counts report)))
    (is (nil? (:first-frontier report)))
    (is (= :accepted
           (get-in row-by-id [:forward-reference/rec-forward-ok :top-status])))
    (is (= :accepted
           (get-in row-by-id [:forward-reference/let-forward-ok :top-status])))
    (is (= :accepted
           (get-in row-by-id [:forward-reference/rec-mutual-ok :top-status])))
    (is (= :failed
           (get-in row-by-id [:forward-reference/rec-cycle :top-status])))
    (is (= :failed
           (get-in row-by-id [:forward-reference/let-cycle :top-status])))
    (is (= :failed
           (get-in row-by-id [:forward-reference/rec-unbound :top-status])))
    (is (= 64 (count (:receipt-hash report))))))

(deftest evaluator-rec-attrset-mutual-recursion
  ;; rec attrsets are the same recursive scope as let: a binding can reference a
  ;; sibling defined later (forward reference) and mutual recursion resolves.
  ;; This mirrors eval-let. See rec-forward-reference-taxonomy.md for the
  ;; multi-lane reclassification that this fix required.
  (testing "forward references and mutual recursion resolve"
    (is (= 1 (:value (pnix/eval-source "rec { x = y; y = 1; }.x"))))
    (is (= {"a" 11 "b" 10} (:value (pnix/eval-source "rec { a = b + 1; b = 10; }"))))
    (is (= {"a" 5 "b" 5 "c" 5} (:value (pnix/eval-source "rec { a = b; b = c; c = 5; }")))))
  (testing "backward references still resolve"
    (is (= {"a" 1 "b" 1} (:value (pnix/eval-source "rec { a = 1; b = a; }")))))
  (testing "recursive closures defined in a rec set work"
    (is (= 6 (:value (pnix/eval-source
                      "(rec { f = x: if x == 0 then 0 else x + f (x - 1); }).f 3")))))
  (testing "a rec cycle is infinite recursion (matching let), not unbound-var"
    (is (= :infinite-recursion (:reason (pnix/eval-source "rec { a = a + 1; }.a")))))
  (testing "a genuinely missing name is still unbound-var"
    (is (= :unbound-var (:reason (pnix/eval-source "rec { a = z + 1; }.a")))))
  (testing "inherit, paths, and non-rec sets are unaffected (regression)"
    (is (= {"x" 5 "y" 6} (:value (pnix/eval-source "let x = 5; in rec { inherit x; y = x + 1; }"))))
    (is (= {"a" {"b" 1} "c" 11} (:value (pnix/eval-source "rec { a.b = 1; c = a.b + 10; }"))))
    (is (= {"a" 1 "b" 2} (:value (pnix/eval-source "{ a = 1; b = 2; }"))))
    (is (= :unbound-var (:reason (pnix/eval-source "{ a = 1; b = a; }"))))))

(deftest evaluator-fold-predicate-builtins-reject-non-lists
  ;; all/any/foldl' (and the foldl alias) iterate their list argument; a string
  ;; could leak characters through a fold accumulator, so they now hold.
  (testing "string arguments are held"
    (is (= :all-arg-not-list (:reason (pnix/eval-source "builtins.all (x: true) \"abc\""))))
    (is (= :any-arg-not-list (:reason (pnix/eval-source "builtins.any (x: true) \"abc\""))))
    (is (= :foldl-arg-not-list (:reason (pnix/eval-source "builtins.foldl' (a: b: b) 0 \"abc\""))))
    (is (= :foldl-arg-not-list (:reason (pnix/eval-source "builtins.foldl (a: b: b) 0 \"abc\"")))))
  (testing "list arguments still work (regression)"
    (is (= true (:value (pnix/eval-source "builtins.all (x: x > 0) [1 2 3]"))))
    (is (= true (:value (pnix/eval-source "builtins.any (x: x > 2) [1 2 3]"))))
    (is (= 10 (:value (pnix/eval-source "builtins.foldl' (a: b: a + b) 0 [1 2 3 4]"))))
    (is (= true (:value (pnix/eval-source "builtins.all (x: false) []"))))
    (is (= 7 (:value (pnix/eval-source "builtins.foldl' (a: b: a + b) 7 []"))))))

(deftest evaluator-more-list-builtins-reject-non-lists
  ;; reverseList/take/drop/unique/elemAt/partition also leaked raw characters
  ;; when given a string; they now hold.
  (testing "string arguments are held"
    (is (= :reverse-list-arg-not-list (:reason (pnix/eval-source "builtins.reverseList \"abc\""))))
    (is (= :take-arg-not-list (:reason (pnix/eval-source "builtins.take 2 \"abc\""))))
    (is (= :drop-arg-not-list (:reason (pnix/eval-source "builtins.drop 1 \"abc\""))))
    (is (= :unique-arg-not-list (:reason (pnix/eval-source "builtins.unique \"abc\""))))
    (is (= :elem-at-arg-not-list (:reason (pnix/eval-source "builtins.elemAt \"abc\" 1"))))
    (is (= :partition-arg-not-list (:reason (pnix/eval-source "builtins.partition (x: true) \"abc\"")))))
  (testing "list arguments still work (regression)"
    (is (= [3 2 1] (:value (pnix/eval-source "builtins.reverseList [1 2 3]"))))
    (is (= [1 2] (:value (pnix/eval-source "builtins.take 2 [1 2 3 4]"))))
    (is (= [2 3] (:value (pnix/eval-source "builtins.drop 1 [1 2 3]"))))
    (is (= [1 2 3] (:value (pnix/eval-source "builtins.unique [1 1 2 3 3]"))))
    (is (= 8 (:value (pnix/eval-source "builtins.elemAt [9 8 7] 1"))))
    (is (= {"right" [3 4] "wrong" [1 2]}
           (:value (pnix/eval-source "builtins.partition (x: x > 2) [1 2 3 4]")))))
  (testing "partition forces elements like Nix even if the predicate ignores them"
    (doseq [source ["builtins.partition (x: true) [ (1 / 0) ]"
                   "builtins.partition (x: false) [ (1 / 0) ]"]]
      (let [r (pnix/eval-source source)]
        (is (= :failed (:status r)) source)
        (is (= :eval-binary-failed (:reason r)) source)))))

(deftest evaluator-list-builtins-reject-non-lists
  ;; map/filter/concatMap/sort iterate their list argument; passing a string
  ;; used to leak raw host characters as pnix values. They now hold instead.
  (testing "a string argument is held, not iterated into characters"
    (is (= :map-arg-not-list (:reason (pnix/eval-source "builtins.map (x: x) \"abc\""))))
    (is (= :filter-arg-not-list (:reason (pnix/eval-source "builtins.filter (x: true) \"abc\""))))
    (is (= :concat-map-arg-not-list (:reason (pnix/eval-source "builtins.concatMap (x: [x]) \"abc\""))))
    (is (= :sort-arg-not-list (:reason (pnix/eval-source "builtins.sort (a: b: true) \"abc\"")))))
  (testing "an attrset argument is also held"
    (is (= :map-arg-not-list (:reason (pnix/eval-source "builtins.map (x: x) { a = 1; }")))))
  (testing "list arguments still work (regression)"
    (is (= [2 3 4] (:value (pnix/eval-source "builtins.map (x: x + 1) [1 2 3]"))))
    (is (= [2 3] (:value (pnix/eval-source "builtins.filter (x: x > 1) [1 2 3]"))))
    (is (= [1 1 2 2] (:value (pnix/eval-source "builtins.concatMap (x: [x x]) [1 2]"))))
    (is (= [1 2 3] (:value (pnix/eval-source "builtins.sort (a: b: a < b) [3 1 2]"))))
    (is (= [] (:value (pnix/eval-source "builtins.map (x: x) []")))))
  (testing "++ requires list operands (oracle: expected a list but found null/int)"
    ;; Pre-fix: Clojure (concat xs nil) treated null as empty → wrong VALUE.
    (is (= :concat-operand-not-list (:reason (pnix/eval-source "[] ++ null"))))
    (is (= :concat-operand-not-list (:reason (pnix/eval-source "null ++ []"))))
    (is (= :concat-operand-not-list (:reason (pnix/eval-source "[1] ++ null"))))
    (is (= :concat-operand-not-list (:reason (pnix/eval-source "[1] ++ 2"))))
    (is (= :concat-operand-not-list (:reason (pnix/eval-source "[] ++ \"x\""))))
    (is (= [1 2 3] (:value (pnix/eval-source "[1] ++ [2 3]"))))
    (is (= [] (:value (pnix/eval-source "[] ++ []")))))
  (testing "// requires attrset operands (oracle: expected a set but found null/list)"
    ;; Pre-fix: Clojure (merge nil m)/(merge m nil) treated null as empty →
    ;; wrong VALUE; (merge [] m) also leaked a list-shaped result.
    (is (= :update-operand-not-attrset (:reason (pnix/eval-source "null // { a = 1; }"))))
    (is (= :update-operand-not-attrset (:reason (pnix/eval-source "{ a = 1; } // null"))))
    (is (= :update-operand-not-attrset (:reason (pnix/eval-source "null // null"))))
    (is (= :update-operand-not-attrset (:reason (pnix/eval-source "1 // { a = 1; }"))))
    (is (= :update-operand-not-attrset (:reason (pnix/eval-source "{ a = 1; } // 2"))))
    (is (= :update-operand-not-attrset (:reason (pnix/eval-source "[] // { a = 1; }"))))
    (is (= :update-operand-not-attrset (:reason (pnix/eval-source "true // { a = 1; }"))))
    (is (= :update-operand-not-attrset (:reason (pnix/eval-source "\"x\" // { a = 1; }"))))
    (is (= {"a" 1 "b" 2} (:value (pnix/eval-source "{ a = 1; } // { b = 2; }"))))
    (is (= {"a" 2} (:value (pnix/eval-source "{ a = 1; } // { a = 2; }")))))
  (testing "attrNames/attrValues require attrsets (oracle: null was wrong VALUE [])"
    (is (= :attr-names-arg-not-attrset (:reason (pnix/eval-source "builtins.attrNames null"))))
    (is (= :attr-names-arg-not-attrset (:reason (pnix/eval-source "builtins.attrNames 1"))))
    (is (= :attr-values-arg-not-attrset (:reason (pnix/eval-source "builtins.attrValues null"))))
    (is (= :attr-values-arg-not-attrset (:reason (pnix/eval-source "builtins.attrValues 1"))))
    (is (= ["a" "b"] (:value (pnix/eval-source "builtins.attrNames { b = 2; a = 1; }"))))
    (is (= [1 2] (:value (pnix/eval-source "builtins.attrValues { b = 2; a = 1; }")))))
  (testing "elem requires a list (oracle: null was wrong VALUE false)"
    (is (= :elem-arg-not-list (:reason (pnix/eval-source "builtins.elem 1 null"))))
    (is (= :elem-arg-not-list (:reason (pnix/eval-source "builtins.elem 1 \"ab\""))))
    (is (= true (:value (pnix/eval-source "builtins.elem 2 [1 2 3]"))))
    (is (= false (:value (pnix/eval-source "builtins.elem 9 [1 2 3]")))))
  (testing "genList length is a non-negative int (oracle: -1/float were wrong VALUE)"
    (is (= :gen-list-length-negative (:reason (pnix/eval-source "builtins.genList (x: x) (-1)"))))
    (is (= :gen-list-length-not-int (:reason (pnix/eval-source "builtins.genList (x: x) 1.5"))))
    (is (= :gen-list-length-not-int (:reason (pnix/eval-source "builtins.genList (x: x) true"))))
    (is (= [] (:value (pnix/eval-source "builtins.genList (x: x) 0"))))
    (is (= [0 1 2] (:value (pnix/eval-source "builtins.genList (x: x) 3")))))
  (testing "string/version builtins reject non-strings (oracle wrong-VALUE class)"
    (is (= :from-json-arg-not-string (:reason (pnix/eval-source "builtins.fromJSON 1"))))
    (is (= :from-json-arg-not-string (:reason (pnix/eval-source "builtins.fromJSON true"))))
    (is (= 1 (:value (pnix/eval-source "builtins.fromJSON \"1\""))))
    (is (= :compare-versions-args-not-string
           (:reason (pnix/eval-source "builtins.compareVersions 1 2"))))
    (is (= -1 (:value (pnix/eval-source "builtins.compareVersions \"1\" \"2\""))))
    (is (= :path-string-arg-not-string (:reason (pnix/eval-source "builtins.dirOf 1"))))
    (is (= :path-string-arg-not-string (:reason (pnix/eval-source "builtins.baseNameOf 1"))))
    (is (= :version-string-arg-not-string (:reason (pnix/eval-source "builtins.parseDrvName 1"))))
    (is (= :version-string-arg-not-string (:reason (pnix/eval-source "builtins.splitVersion 1"))))
    (is (= "b" (:value (pnix/eval-source "builtins.baseNameOf \"a/b\"")))))
  (testing "toJSON rejects functions (oracle: cannot convert a function to JSON)"
    (is (= :to-json-cannot-convert-function
           (:reason (pnix/eval-source "builtins.toJSON (x: x)"))))
    (is (= "null" (:value (pnix/eval-source "builtins.toJSON null"))))
    (is (= "true" (:value (pnix/eval-source "builtins.toJSON true")))))
  (testing "catAttrs / listToAttrs type checks (oracle wrong-VALUE class)"
    (is (= :cat-attrs-arg-not-list
           (:reason (pnix/eval-source "builtins.catAttrs \"a\" null"))))
    (is (= [1 2] (:value (pnix/eval-source
                          "builtins.catAttrs \"a\" [ { a = 1; } { a = 2; } ]"))))
    (is (= :list-to-attrs-element-not-attrset
           (:reason (pnix/eval-source "builtins.listToAttrs [ 1 ]"))))
    (is (= {"a" 1} (:value (pnix/eval-source
                            "builtins.listToAttrs [ { name = \"a\"; value = 1; } ]")))))
  (testing "hasAttr/intersectAttrs/mapAttrs/groupBy reject null (wrong VALUE → {})"
    ;; `?` is false on non-sets; hasAttr stays strict (oracle).
    (is (= :has-attr-arg-not-attrset
           (:reason (pnix/eval-source "builtins.hasAttr \"a\" null"))))
    (is (= false (:value (pnix/eval-source "null ? a"))))
    (is (= true (:value (pnix/eval-source "builtins.hasAttr \"a\" { a = 1; }"))))
    (is (= :intersect-attrs-left-not-attrset
           (:reason (pnix/eval-source "builtins.intersectAttrs null { a = 1; }"))))
    (is (= :intersect-attrs-right-not-attrset
           (:reason (pnix/eval-source "builtins.intersectAttrs { a = 1; } null"))))
    (is (= {"b" 3} (:value (pnix/eval-source
                            "builtins.intersectAttrs { a = 1; b = 2; } { b = 3; c = 4; }"))))
    (is (= :map-attrs-arg-not-attrset
           (:reason (pnix/eval-source "builtins.mapAttrs (n: v: v) null"))))
    (is (= {"a" 1} (:value (pnix/eval-source "builtins.mapAttrs (n: v: v) { a = 1; }"))))
    (is (= :group-by-arg-not-list
           (:reason (pnix/eval-source "builtins.groupBy (x: x) null"))))
    (is (= {"g" [1 2]}
           (:value (pnix/eval-source "builtins.groupBy (x: \"g\") [ 1 2 ]")))))
  (testing "zipAttrsWith/genericClosure/elemAt/replaceStrings type edges"
    (is (= :zip-attrs-with-arg-not-list
           (:reason (pnix/eval-source "builtins.zipAttrsWith (n: vs: vs) null"))))
    (is (= {"a" [1 2] "b" [3]}
           (:value (pnix/eval-source
                    "builtins.zipAttrsWith (n: vs: vs) [ { a = 1; } { a = 2; b = 3; } ]"))))
    (is (= :generic-closure-arg-not-attrset
           (:reason (pnix/eval-source "builtins.genericClosure 1"))))
    (is (= [] (:value (pnix/eval-source
                       "builtins.genericClosure { startSet = []; operator = x: []; }"))))
    (is (= :elem-at-index-not-int
           (:reason (pnix/eval-source "builtins.elemAt [1 2] 1.0"))))
    (is (= 2 (:value (pnix/eval-source "builtins.elemAt [1 2] 1"))))
    (is (= :replace-strings-length-mismatch
           (:reason (pnix/eval-source
                     "builtins.replaceStrings [\"a\"] [\"b\" \"c\"] \"a\""))))
    (is (= "b" (:value (pnix/eval-source
                        "builtins.replaceStrings [\"a\"] [\"b\"] \"a\"")))))
  (testing "catAttrs name must be string; getAttr set must be attrset"
    (is (= :cat-attrs-name-not-string
           (:reason (pnix/eval-source "builtins.catAttrs 1 [ { \"1\" = 2; } ]"))))
    (is (= :cat-attrs-name-not-string
           (:reason (pnix/eval-source "builtins.catAttrs null [ ]"))))
    (is (= [1 2] (:value (pnix/eval-source
                          "builtins.catAttrs \"a\" [ { a = 1; } { a = 2; } ]"))))
    (is (= :get-attr-arg-not-attrset
           (:reason (pnix/eval-source "builtins.getAttr \"a\" null"))))
    (is (= 1 (:value (pnix/eval-source "builtins.getAttr \"a\" { a = 1; }")))))
  (testing "baseNameOf root and trailing-slash (oracle: \"/\" → \"\")"
    (is (= "" (:value (pnix/eval-source "builtins.baseNameOf \"/\""))))
    (is (= "" (:value (pnix/eval-source "builtins.baseNameOf \"\""))))
    (is (= "a" (:value (pnix/eval-source "builtins.baseNameOf \"/a/\""))))
    (is (= "a" (:value (pnix/eval-source "builtins.baseNameOf \"a/\""))))
    (is (= "c" (:value (pnix/eval-source "builtins.baseNameOf \"a/b/c\""))))
    (is (= "/" (:value (pnix/eval-source "builtins.dirOf \"/\""))))))

(deftest evaluator-tryeval-only-catches-throw-assert
  ;; Nix tryEval catches only throw and assert; abort, type errors, division by
  ;; zero, and out-of-bounds errors propagate.
  (testing "throw and assert are caught as success=false"
    (is (= {"success" false "value" false}
           (:value (pnix/eval-source "builtins.tryEval (throw \"x\")"))))
    (is (= {"success" false "value" false}
           (:value (pnix/eval-source "builtins.tryEval (assert false; 1)")))))
  (testing "abort and eval errors are NOT caught"
    (is (= :abort-builtin-called (:reason (pnix/eval-source "builtins.tryEval (abort \"x\")"))))
    (is (= :head-of-empty-list (:reason (pnix/eval-source "builtins.tryEval (builtins.head [])"))))
    (is (= :failed (:status (pnix/eval-source "builtins.tryEval (1 / 0)"))))
    (is (= :failed (:status (pnix/eval-source "builtins.tryEval ([1] + [2])")))))
  (testing "successful evaluation reports success=true with the value"
    (is (= {"success" true "value" 42} (:value (pnix/eval-source "builtins.tryEval 42"))))
    (is (= true (:value (pnix/eval-source "(builtins.tryEval (1 + 2)).success"))))
    (is (= 99 (:value (pnix/eval-source
                       "let t = builtins.tryEval (throw \"e\"); in if t.success then t.value else 99"))))))

(deftest evaluator-isattrs-excludes-functions
  ;; Closures and builtins are tagged maps, so isAttrs must not treat them as
  ;; attrsets (a bare map? check did).
  (testing "functions are not attrsets"
    (is (= false (:value (pnix/eval-source "builtins.isAttrs (x: x)"))))
    (is (= false (:value (pnix/eval-source "builtins.isAttrs builtins.add")))))
  (testing "attrsets still are"
    (is (= true (:value (pnix/eval-source "builtins.isAttrs { a = 1; }"))))
    (is (= true (:value (pnix/eval-source "builtins.isAttrs {}")))))
  (testing "non-maps are not attrsets"
    (is (= false (:value (pnix/eval-source "builtins.isAttrs 5"))))
    (is (= false (:value (pnix/eval-source "builtins.isAttrs [1 2]"))))
    (is (= false (:value (pnix/eval-source "builtins.isAttrs \"x\"")))))
  (testing "typeOf and isFunction agree"
    (is (= "lambda" (:value (pnix/eval-source "builtins.typeOf (x: x)"))))
    (is (= "set" (:value (pnix/eval-source "builtins.typeOf { a = 1; }"))))
    (is (= true (:value (pnix/eval-source "builtins.isFunction (x: x)"))))))

(deftest evaluator-compare-versions-nix-rules
  ;; compareVersions follows Nix's special component rules: "pre" is older than
  ;; an absent component, an absent component is older than a real one, and a
  ;; numeric component is newer than a non-numeric one.
  (let [cv (fn [a b] (:value (pnix/eval-source
                              (format "builtins.compareVersions \"%s\" \"%s\"" a b))))]
    (testing "pre-release is older than the release"
      (is (= 1 (cv "1.0" "1.0-pre")))
      (is (= -1 (cv "1.0-pre" "1.0")))
      (is (= 0 (cv "1.0-pre" "1.0-pre")))
      (is (= -1 (cv "1.0-pre" "1.0-rc1"))))
    (testing "an absent component is older than a present one"
      (is (= -1 (cv "1.0" "1.0.0")))
      (is (= 1 (cv "1.0.0" "1.0"))))
    (testing "numeric beats non-numeric; otherwise lexical"
      (is (= 1 (cv "1.1" "1.a")))
      (is (= -1 (cv "1.a" "1.1")))
      (is (= -1 (cv "1.0a" "1.0b"))))
    (testing "ordinary numeric comparison (regression)"
      (is (= -1 (cv "1.0" "1.1")))
      (is (= 1 (cv "2.0" "1.9")))
      (is (= 0 (cv "1.0" "1.0"))))))

(deftest evaluator-replacestrings-empty-needle
  ;; An empty `from` matches at every position including the end, so
  ;; replaceStrings [""] ["X"] "ab" is "XaXbX". Non-empty needles are a single
  ;; left-to-right pass (unchanged).
  (testing "empty needle inserts the replacement around every character"
    (is (= "XaXbX" (:value (pnix/eval-source "builtins.replaceStrings [\"\"] [\"X\"] \"ab\""))))
    (is (= "-" (:value (pnix/eval-source "builtins.replaceStrings [\"\"] [\"-\"] \"\""))))
    (is (= "XaX" (:value (pnix/eval-source "builtins.replaceStrings [\"\" \"a\"] [\"X\" \"Y\"] \"a\"")))))
  (testing "non-empty needles still do a single pass (regression)"
    (is (= "bXnXnX" (:value (pnix/eval-source "builtins.replaceStrings [\"a\"] [\"X\"] \"banana\""))))
    (is (= "Xc" (:value (pnix/eval-source "builtins.replaceStrings [\"ab\" \"a\"] [\"X\" \"Y\"] \"abc\""))))
    (is (= "b" (:value (pnix/eval-source "builtins.replaceStrings [\"a\" \"b\"] [\"b\" \"c\"] \"a\""))))
    (is (= "abc" (:value (pnix/eval-source "builtins.replaceStrings [\"z\"] [\"Q\"] \"abc\""))))))

(deftest lowering-lane-string-and-json-edges
  ;; These edges were once evaluator-only: the clj-meta lowering lane used raw
  ;; `subs` or fell back through an unbound `builtins` symbol.
  (doseq [[source expected]
          [["builtins.substring 10 2 \"abc\"" ""]
           ["builtins.substring 1 (-1) \"abcd\"" "bcd"]
           ["builtins.replaceStrings [\"\"] [\"X\"] \"ab\"" "XaXbX"]
           ["builtins.replaceStrings [\"a\" \"b\"] [\"b\" \"c\"] \"a\"" "b"]
           ["builtins.fromJSON \"{\\\"a\\\":1,\\\"b\\\":[2,3]}\""
            {"a" 1 "b" [2 3]}]]]
  (let [receipt (pnix/verify-source source)]
    (is (= :accepted (:status receipt)) source)
    (is (= expected (get-in receipt [:eval-result :value])) source)
    (is (= expected (get-in receipt [:clj-meta-result :value])) source)
    (is (= :agree (get-in receipt [:cross-mirror-verdict :equivalence]))
        source))))

(deftest lowering-lane-core-expressions
  ;; Core forms route through the clj-meta host-execution lane without making
  ;; proof receipts a precondition, including `let`/`if`/lambda/call forms.
  (doseq [[source expected]
          [["let x = 1; y = 2; in x + y" 3]
           ["if true then 42 else (1 / 0)" 42]
           ["let id = x: x; in id 41" 41]
           ["let max = x: y: if x < y then y else x; in max 3 9" 9]
           ["let add = x: y: x + y; in (add 7) 8" 15]]]
    (let [receipt (pnix/compile-source source)]
      (is (= :ok (:status receipt)) source)
      (is (= :pnix-source-host-execution-ready (:reason receipt)) source)
      (is (= expected (get-in receipt [:clj-meta-result :value])) source))))

(deftest evaluator-list-builtin-guards
  ;; head/tail/init error on an empty list and on a non-list, matching Nix,
  ;; instead of silently returning nil/[]; length requires a list.
  (testing "empty list is held, not silently nil/[]"
    (is (= :head-of-empty-list (:reason (pnix/eval-source "builtins.head []"))))
    (is (= :tail-of-empty-list (:reason (pnix/eval-source "builtins.tail []"))))
    (is (= :last-of-empty-list (:reason (pnix/eval-source "builtins.last []"))))
    (is (= :init-of-empty-list (:reason (pnix/eval-source "builtins.init []")))))
  (testing "non-list argument is held"
    (is (= :head-not-list (:reason (pnix/eval-source "builtins.head \"abc\""))))
    (is (= :tail-not-list (:reason (pnix/eval-source "builtins.tail \"abc\""))))
    (is (= :last-not-list (:reason (pnix/eval-source "builtins.last \"abc\""))))
    (is (= :init-not-list (:reason (pnix/eval-source "builtins.init \"abc\""))))
    (is (= :length-not-list (:reason (pnix/eval-source "builtins.length \"abc\"")))))
  (testing "list guard errors do not silently succeed in mirror lanes"
    (doseq [[source expected-reason]
            [["builtins.head []" :head-of-empty-list]
             ["builtins.tail []" :tail-of-empty-list]
             ["builtins.last []" :last-of-empty-list]
             ["builtins.init []" :init-of-empty-list]
             ["builtins.head \"abc\"" :head-not-list]
             ["builtins.tail \"abc\"" :tail-not-list]
             ["builtins.last \"abc\"" :last-not-list]
             ["builtins.init \"abc\"" :init-not-list]
             ["builtins.length \"abc\"" :length-not-list]]]
      (let [receipt (pnix/verify-source source)]
        (is (= :failed (:status receipt)) source)
        (is (= expected-reason (get-in receipt [:eval-result :reason])) source)
        (is (= :failed (get-in receipt [:clj-meta-result :status])) source)
        (is (= :failed (get-in receipt [:px-runtime :status])) source))))
  (testing "valid list operations still work (regression)"
    (is (= 10 (:value (pnix/eval-source "builtins.head [10 20]"))))
    (is (= 20 (:value (pnix/eval-source "builtins.last [10 20]"))))
    (is (= [20 30] (:value (pnix/eval-source "builtins.tail [10 20 30]"))))
    (is (= [1 2] (:value (pnix/eval-source "builtins.init [1 2 3]"))))
    (is (= 3 (:value (pnix/eval-source "builtins.length [1 2 3]"))))
    (is (= 0 (:value (pnix/eval-source "builtins.length []"))))
    (is (= [] (:value (pnix/eval-source "builtins.tail [42]"))))))

(deftest evaluator-nix-equality
  ;; Nix equality compares numbers across int/float (1 == 1.0), recurses into
  ;; lists/attrsets, and short-circuits shared nested values by identity.
  (testing "numbers compare by value across int and float"
    (is (= true (:value (pnix/eval-source "1 == 1.0"))))
    (is (= false (:value (pnix/eval-source "1 != 1.0"))))
    (is (= false (:value (pnix/eval-source "1 == 2"))))
    (is (= true (:value (pnix/eval-source "[1] == [1.0]"))))
    (is (= true (:value (pnix/eval-source "{ a = 1; } == { a = 1.0; }"))))
    (is (= true (:value (pnix/eval-source "builtins.eq 1 1.0"))))
    (is (= true (:value (pnix/eval-source "builtins.elem 1.0 [1 2 3]"))))
    (is (= false (:value (pnix/eval-source "builtins.elem (1 / 0) []")))))
  (testing "functions are unequal as scalars but shared nested values short-circuit"
    (is (= false (:value (pnix/eval-source "let f = x: x; in f == f"))))
    (is (= true (:value (pnix/eval-source "let f = x: x; in [f] == [f]"))))
    (is (= false (:value (pnix/eval-source
                          "let f = x: x; g = x: x; in [f] == [g]")))))
  (testing "shared list/attr identity follows the exact Nix forcing boundary"
    (doseq [[source expected]
            [["let l = [ (x: x) ]; in l == l" true]
             ["let a = { f = x: x; }; in a == a" true]
             ["let l = [ (x: x) ]; in l == [ (x: x) ]" false]
             ["let a = { f = x: x; }; in a == { f = x: x; }" false]
             ["let l = [ throw \"x\" ]; in l == l" true]
             ["let f = x: x; in builtins.elem f [ f ]" true]
             ["let f = x: x; g = h: [ h ]; in (g f) == (g f)" true]
             ["let f = x: x; g = h: [ h 0 ]; in (g f) < (g f)" false]
             [(str "let forward = n: h: if n == 0 then [ h ] "
                   "else forward (n - 1) h; f = x: x; in "
                   "(forward 80 f) == (forward 80 f)") true]]]
      (let [row (pnix/verify-source source)]
        (is (= expected (get-in row [:eval-result :value])) source)
        (is (= expected (:value (machine/eval-source source))) source)
        (is (= expected (get-in row [:clj-meta-result :value])) source)
        (is (= expected (get-in row [:px-runtime :value])) source)))
    (let [source "let a = { f = throw \"x\"; }; in a == a"
          row (pnix/verify-source source)
          machine-result (machine/eval-source source)]
      (is (= :throw-builtin-called (get-in row [:eval-result :reason])))
      (is (= :throw-builtin-called (:reason machine-result)))
      (is (= :failed (get-in row [:clj-meta-result :status])))
      (is (= :throw-builtin-called
             (get-in row [:clj-meta-result :error :class])))
      (is (= :failed (get-in row [:px-runtime :status]))))
    (let [source "let l = [ (throw \"x\") ]; in l == l"
          row (pnix/verify-source source)
          machine-result (machine/eval-source source)]
      (is (= :throw-builtin-called (get-in row [:eval-result :reason])))
      (is (= :throw-builtin-called (:reason machine-result)))
      (is (= :failed (get-in row [:clj-meta-result :status])))
      (is (= :throw-builtin-called
             (get-in row [:clj-meta-result :error :class])))
      (is (= :failed (get-in row [:px-runtime :status])))))
  (testing "ordinary equality still holds (regression)"
    (is (= true (:value (pnix/eval-source "[1 2] == [1 2]"))))
    (is (= true (:value (pnix/eval-source "{ a = [1 { b = 2; }]; } == { a = [1 { b = 2; }]; }"))))
    (is (= false (:value (pnix/eval-source "{ a = 1; } == { b = 1; }"))))
    (is (= false (:value (pnix/eval-source "[1] == [1 2]"))))
    (is (= false (:value (pnix/eval-source "true == 1"))))
    (is (= true (:value (pnix/eval-source "null == null"))))))

(deftest evaluator-implication-operator
  ;; `->` is logical implication (!a || b): lowest precedence, right
  ;; associative, short-circuits on a false antecedent.
  (testing "truth table"
    (is (= true (:value (pnix/eval-source "true -> true"))))
    (is (= false (:value (pnix/eval-source "true -> false"))))
    (is (= true (:value (pnix/eval-source "false -> true"))))
    (is (= true (:value (pnix/eval-source "false -> false")))))
  (testing "a false antecedent short-circuits the consequent"
    (is (= true (:value (pnix/eval-source "false -> (1 / 0 == 0)")))))
  (testing "right associative"
    (is (= true (:value (pnix/eval-source "false -> true -> false"))))
    (is (= false (:value (pnix/eval-source "true -> true -> false")))))
  (testing "lower precedence than || and comparison"
    (is (= true (:value (pnix/eval-source "false || false -> true"))))
    (is (= true (:value (pnix/eval-source "1 < 2 -> 3 < 4")))))
  (testing "subtraction and > are unaffected by the -> token"
    (is (= 2 (:value (pnix/eval-source "5 - 3"))))
    (is (= -7 (:value (pnix/eval-source "0 - 7"))))
    (is (= true (:value (pnix/eval-source "5 > 3"))))))

(deftest evaluator-attrset-builtins-batch
  ;; Previously-missing attrset builtins: mapAttrs', genAttrs, nameValuePair,
  ;; foldlAttrs, addErrorContext, unsafeGetAttrPos.
  (testing "mapAttrs' rebuilds an attrset from name/value pairs"
    (is (= {"a!" 2 "b!" 3}
           (:value (pnix/eval-source
                    "builtins.mapAttrs' (k: v: { name = k + \"!\"; value = v + 1; }) { a = 1; b = 2; }"))))
    (is (= {"same" 1}
           (:value (pnix/eval-source
                    "builtins.mapAttrs' (k: v: { name = \"same\"; value = v; }) { a = 1; b = 2; }")))
        "first occurrence of a duplicate name wins"))
  (testing "mapAttrs' rejects a pair missing name/value"
    (is (= :map-attrs-prime-bad-pair
           (:reason (pnix/eval-source "builtins.mapAttrs' (k: v: { foo = 1; }) { a = 1; }")))))
  (testing "genAttrs builds { name = f name; ... }"
    (is (= {"a" "ax" "b" "bx"}
           (:value (pnix/eval-source "builtins.genAttrs [\"a\" \"b\"] (n: n + \"x\")")))))
  (testing "nameValuePair"
    (is (= {"name" "k" "value" 42}
           (:value (pnix/eval-source "builtins.nameValuePair \"k\" 42")))))
  (testing "foldlAttrs folds over keys and values"
    (is (= 6 (:value (pnix/eval-source
                      "builtins.foldlAttrs (acc: k: v: acc + v) 0 { a = 1; b = 2; c = 3; }"))))
    (is (= "ab" (:value (pnix/eval-source
                         "builtins.foldlAttrs (acc: k: v: acc + k) \"\" { a = 1; b = 2; }")))))
  (testing "addErrorContext is an identity passthrough"
    (is (= 99 (:value (pnix/eval-source "builtins.addErrorContext \"ctx\" 99"))))))

(deftest evaluator-unsafe-get-attr-pos-surfaces-parser-spans
  (testing "direct attr key positions are retained"
    (is (= {"span" [32 33] "start" 32 "end" 33}
           (:value (pnix/eval-source
                    "builtins.unsafeGetAttrPos \"a\" { a = 1; }")))))
  (testing "nested dotted attr path positions are retained on nested attrsets"
    (is (= {"span" [35 36] "start" 35 "end" 36}
           (:value (pnix/eval-source
                    "builtins.unsafeGetAttrPos \"b\" ({ a.b = 1; }.a)")))))
  (testing "missing or synthetic position returns null"
    (is (nil? (:value (pnix/eval-source
                       "builtins.unsafeGetAttrPos \"z\" { a = 1; }"))))))

(deftest evaluator-cur-pos-surfaces-var-parser-span
  (is (= {"span" [0 8] "start" 0 "end" 8}
         (:value (pnix/eval-source "__curPos"))))
  (is (= {"span" [8 16] "start" 8 "end" 16}
         (:value (pnix/eval-source "let p = __curPos; in p")))))

(deftest evaluator-tostring-coercion
  ;; builtins.toString follows Nix's coerceMore coercion, which differs from
  ;; boolToString and from string interpolation.
  (testing "scalars"
    (is (= "42" (:value (pnix/eval-source "builtins.toString 42"))))
    ;; D4 oracle correction: Nix formats floats as %.6f, not shortest-repr.
    (is (= "3.140000" (:value (pnix/eval-source "builtins.toString 3.14"))))
    (is (= "hi" (:value (pnix/eval-source "builtins.toString \"hi\"")))))
  (testing "booleans coerce to 1/empty (not true/false)"
    (is (= "1" (:value (pnix/eval-source "builtins.toString true"))))
    (is (= "" (:value (pnix/eval-source "builtins.toString false")))))
  (testing "null coerces to empty"
    (is (= "" (:value (pnix/eval-source "builtins.toString null")))))
  (testing "lists join element coercions with spaces, recursively"
    (is (= "1 2 3" (:value (pnix/eval-source "builtins.toString [1 2 3]"))))
    (is (= "1 2 3" (:value (pnix/eval-source "builtins.toString [[1] [2 3]]"))))
    (is (= "1 1 x" (:value (pnix/eval-source "builtins.toString [1 true \"x\"]")))))
  (testing "an attrset coerces via outPath"
    (is (= "/nix/store/x"
           (:value (pnix/eval-source "builtins.toString { outPath = \"/nix/store/x\"; }")))))
  (testing "incoercible values are held"
    (is (= :to-string-builtin-failed
           (:reason (pnix/eval-source "builtins.toString { a = 1; }"))))
    (is (= :to-string-builtin-failed
           (:reason (pnix/eval-source "builtins.toString (x: x)"))))))

(deftest evaluator-split-interleaves-groups
  ;; Nix `split` returns substrings between matches interleaved with each
  ;; match's capture-group list (empty list when the regex has no groups), not
  ;; a plain string split.
  (testing "no capture groups yields empty-list separators"
    (is (= ["x" [] "y" [] "z"] (:value (pnix/eval-source "builtins.split \"a\" \"xayaz\""))))
    (is (= ["a" [] "b" [] "c"] (:value (pnix/eval-source "builtins.split \"[0-9]+\" \"a12b34c\"")))))
  (testing "capture groups appear as lists between the pieces"
    (is (= ["" ["a"] "c"] (:value (pnix/eval-source "builtins.split \"(a)b\" \"abc\""))))
    (is (= ["" ["a" "b"] "c"] (:value (pnix/eval-source "builtins.split \"(a)(b)\" \"abc\"")))))
  (testing "no match returns the whole string as the only piece"
    (is (= ["abc"] (:value (pnix/eval-source "builtins.split \"z\" \"abc\"")))))
  (testing "a full match leaves empty pieces on both sides"
    (is (= ["" [] ""] (:value (pnix/eval-source "builtins.split \"x\" \"x\""))))))

(deftest evaluator-posix-regex-classes-nix-parity
  ;; Nix 2.34.7 uses ASCII POSIX ERE named classes. Java Pattern has the same
  ;; classes behind \p{...}, but does not recognize the `[[:name:]]` spelling.
  (testing "all standard ASCII named classes translate without Unicode drift"
    (doseq [[class-name sample]
            [["alnum" "Az09"]
             ["alpha" "Az"]
             ["blank" " \t"]
             ["cntrl" "\t"]
             ["digit" "09"]
             ["graph" "Az!9"]
             ["lower" "az"]
             ["print" " Az!9"]
             ["punct" "!?"]
             ["space" " \t"]
             ["upper" "AZ"]
             ["xdigit" "aF09"]]]
      (let [pattern (evaluator/nix-regex-pattern
                     (str "[[:" class-name ":]]+"))]
        (is (= sample (re-matches pattern sample)) class-name)))
    (is (nil? (re-matches (evaluator/nix-regex-pattern "[[:alpha:]]+")
                          (str (char 0xE9)))))
    (is (nil? (re-matches (evaluator/nix-regex-pattern "[[:space:]]+")
                          (str (char 0xA0))))))
  (testing "only named tokens nested in an outer bracket class are translated"
    (is (= "[:space:]"
           (.pattern (evaluator/nix-regex-pattern "[:space:]"))))
    (is (= "[:bogus:]"
           (.pattern (evaluator/nix-regex-pattern "[:bogus:]"))))
    (is (thrown? java.util.regex.PatternSyntaxException
                 (evaluator/nix-regex-pattern "[[:bogus:]]")))
    (is (= "[\\p{Alpha}_]+"
           (.pattern (evaluator/nix-regex-pattern "[[:alpha:]_]+"))))
    (is (= [] (:value (pnix/eval-source
                       "builtins.match \"[:space:]\" \":\""))))
    (let [invalid (pnix/eval-source
                   "builtins.match \"[[:bogus:]]\" \"b\"")]
      (is (= :failed (:status invalid)))
      (is (= :invalid-regex
             (get-in invalid [:error :class])))))
  (testing "match trim and split agree in evaluator, machine, and lowered lanes"
    (doseq [[source expected]
            [["builtins.match \"[[:space:]]*(.*[^[:space:]])[[:space:]]*\" \" ?x \""
              ["?x"]]
             ["builtins.split \"[[:space:]]+\" \"a \\tb\""
              ["a" [] "b"]]]]
      (let [receipt (pnix/verify-source source)]
        (is (= expected (get-in receipt [:eval-result :value])) source)
        (is (= expected (:value (machine/eval-source source))) source)
        (is (= expected (get-in receipt [:clj-meta-result :value])) source))))
  (testing "contextful subjects use the same translation in direct evaluator paths"
    (let [prefix "let s = builtins.appendContext "
          context " { \"/p\" = { path = true; }; }; in "]
      (is (= ["?x"]
             (:value (pnix/eval-source
                      (str prefix "\" ?x \"" context
                           "builtins.match "
                           "\"[[:space:]]*(.*[^[:space:]])[[:space:]]*\" s")))))
      (is (= ["a" [] "b"]
             (:value (pnix/eval-source
                      (str prefix "\"a \\tb\"" context
                           "builtins.split \"[[:space:]]+\" s"))))))))

(deftest evaluator-fromjson-parses-json
  ;; fromJSON must parse JSON, not EDN: compact `{"a":1}` is the integer 1, not
  ;; the keyword :1 that edn/read-string would produce.
  (testing "compact objects parse numbers as numbers"
    (is (= {"a" 1} (:value (pnix/eval-source "builtins.fromJSON \"{\\\"a\\\":1}\""))))
    (is (= {"a" 1 "b" [2 3] "c" true "d" nil}
           (:value (pnix/eval-source
                    "builtins.fromJSON \"{\\\"a\\\":1,\\\"b\\\":[2,3],\\\"c\\\":true,\\\"d\\\":null}\"")))))
  (testing "scalars and arrays"
    (is (= [1 2 3] (:value (pnix/eval-source "builtins.fromJSON \"[1,2,3]\""))))
    (is (= 42 (:value (pnix/eval-source "builtins.fromJSON \"42\""))))
    (is (= "hi" (:value (pnix/eval-source "builtins.fromJSON \"\\\"hi\\\"\"")))))
  (testing "toJSON/fromJSON round-trips a pnix value"
    (is (= {"x" 1 "y" [2 3]}
           (:value (pnix/eval-source
                    "builtins.fromJSON (builtins.toJSON { x = 1; y = [2 3]; })")))))
  (testing "a parsed object can be selected"
    (is (= 1 (:value (pnix/eval-source "(builtins.fromJSON \"{\\\"a\\\":1}\").a")))))
  (testing "invalid JSON is held, not crashed"
    (is (= :from-json-builtin-failed
           (:reason (pnix/eval-source "builtins.fromJSON \"{not json\""))))))

(deftest evaluator-ordering-comparisons
  ;; `<`/`>`/`<=`/`>=` order numbers numerically, strings and lists
  ;; lexicographically (a proper prefix is smaller); incomparable operands are
  ;; held rather than crashing.
  (testing "numbers (regression)"
    (is (= true (:value (pnix/eval-source "1 < 2"))))
    (is (= false (:value (pnix/eval-source "2 < 1"))))
    (is (= true (:value (pnix/eval-source "1 <= 1"))))
    (is (= true (:value (pnix/eval-source "3 >= 2"))))
    (is (= true (:value (pnix/eval-source "1.5 < 2")))))
  (testing "strings compare lexicographically"
    (is (= true (:value (pnix/eval-source "\"a\" < \"b\""))))
    (is (= false (:value (pnix/eval-source "\"b\" < \"a\""))))
    (is (= true (:value (pnix/eval-source "\"ab\" < \"ac\"")))))
  (testing "lists compare lexicographically with prefix ordering"
    (is (= true (:value (pnix/eval-source "[1 2] < [1 3]"))))
    (is (= true (:value (pnix/eval-source "[1] < [1 2]"))))
    (is (= false (:value (pnix/eval-source "[1 2] < [1]"))))
    (is (= false (:value (pnix/eval-source "[2] < [1 9]"))))
    (is (= true (:value (pnix/eval-source "[1 2] <= [1 2]"))))
    (is (= true (:value (pnix/eval-source "[[1] [2]] < [[1] [3]]")))))
  (testing "incomparable operands are held, not crashes"
    (is (= :eval-binary-failed (:reason (pnix/eval-source "{ a = 1; } < { a = 2; }"))))
    (is (= :eval-binary-failed (:reason (pnix/eval-source "true < false"))))
    (is (= :eval-binary-failed (:reason (pnix/eval-source "1 < \"a\""))))))

(deftest evaluator-nested-attr-paths
  ;; Dotted attribute paths on the LHS of attrset bindings (`a.b.c = v`) build
  ;; and merge nested attrsets, matching Nix.
  (testing "paths sharing a prefix merge into nested attrsets"
    (is (= {"a" {"b" 1 "c" 2}} (:value (pnix/eval-source "{ a.b = 1; a.c = 2; }"))))
    (is (= {"a" {"b" {"c" 5}}} (:value (pnix/eval-source "{ a.b.c = 5; }"))))
    (is (= {"a" {"b" {"c" 1 "d" 2} "e" 3}}
           (:value (pnix/eval-source "{ a.b.c = 1; a.b.d = 2; a.e = 3; }")))))
  (testing "a nested path can be selected back"
    (is (= 9 (:value (pnix/eval-source "{ a.b.c = 9; }.a.b.c")))))
  (testing "dynamic keys work inside a path"
    (is (= {"a" {"x" 7}} (:value (pnix/eval-source "let k = \"x\"; in { a.${k} = 7; }")))))
  (testing "rec attrsets see merged nested attrsets"
    (is (= {"a" {"b" 1} "c" 11}
           (:value (pnix/eval-source "rec { a.b = 1; c = a.b + 10; }")))))
  (testing "conflicting definitions are held at PARSE time (D10, like real Nix)"
    ;; real Nix reports `attribute 'a.b' already defined at <pos>` from the
    ;; parser's addAttr merge, so these hold as parse results now
    (doseq [src ["{ a.b = 1; a.b = 2; }" "{ a = 1; a.b = 2; }"]]
      (let [r (pnix/eval-source src)]
        (is (= :failed (:status r)) src)
        (is (= :unsupported-syntax (:reason r)) src))))
  (testing "string keys are not split on the dot"
    (is (= {"a.b" 1} (:value (pnix/eval-source "{ \"a.b\" = 1; }"))))))

(deftest evaluator-dynamic-select-has-attr
  ;; Bare `${ expr }` dynamic keys in select (`s.${e}`) and has-attr
  ;; (`s ? ${e}`) positions, including the `or` fallback.
  (testing "dynamic select reads the computed attribute"
    (is (= 7 (:value (pnix/eval-source "let k = \"a\"; in { a = 7; }.${k}"))))
    (is (= 5 (:value (pnix/eval-source
                      "let n = \"x\"; s = { pre_x = 5; }; in s.${\"pre_\" + n}")))))
  (testing "dynamic select with or falls back when the attr is missing"
    (is (= 1 (:value (pnix/eval-source "let k = \"a\"; in { a = 1; }.${k} or 99"))))
    (is (= 99 (:value (pnix/eval-source "let k = \"z\"; in { a = 1; }.${k} or 99")))))
  (testing "dynamic has-attr tests the computed attribute"
    (is (= true (:value (pnix/eval-source "let k = \"a\"; in { a = 1; } ? ${k}"))))
    (is (= false (:value (pnix/eval-source "let k = \"z\"; in { a = 1; } ? ${k}")))))
  (testing "static select/has-attr regress cleanly"
    (is (= 9 (:value (pnix/eval-source "{ a = { b = 9; }; }.a.b"))))
    (is (= 42 (:value (pnix/eval-source "{ a = 1; }.z or 42"))))
    (is (= true (:value (pnix/eval-source "{ a = 1; } ? a"))))))

(deftest evaluator-default-scope-builtins
  ;; Nix binds a fixed subset of builtins unprefixed at the top level; the rest
  ;; require the `builtins.` prefix. This checks the subset is reachable, the
  ;; prefix still works, and non-default builtins are NOT leaked unprefixed.
  (testing "default-scope builtins are reachable unprefixed"
    (is (= "5" (:value (pnix/eval-source "toString 5"))))
    (is (= "v=3" (:value (pnix/eval-source "let x = 3; in \"v=${toString x}\""))))
    (is (= [2 3 4] (:value (pnix/eval-source "map (x: x + 1) [1 2 3]"))))
    (is (= true (:value (pnix/eval-source "isNull null"))))
    (is (= "c.txt" (:value (pnix/eval-source "baseNameOf \"/a/b/c.txt\""))))
    (is (= "/a/b" (:value (pnix/eval-source "dirOf \"/a/b/c.txt\""))))
    (is (= {"b" 2} (:value (pnix/eval-source
                            "removeAttrs { a = 1; b = 2; } [\"a\"]")))))
  (testing "throw/abort surface as held, not crashes"
    (is (= :throw-builtin-called (:reason (pnix/eval-source "throw \"boom\""))))
    (is (= :abort-builtin-called (:reason (pnix/eval-source "abort \"stop\"")))))
  (testing "the builtins. prefix still resolves the same functions"
    (is (= "9" (:value (pnix/eval-source "builtins.toString 9")))))
  (testing "non-default builtins are not leaked unprefixed"
    (is (= :unbound-var (:reason (pnix/eval-source "head [1 2]"))))))

(deftest evaluator-dynamic-attrset-key
  ;; Bare `${ expr }` dynamic attribute keys in attrset literals. The `${`
  ;; tokenizes as punctuation (no regex-group renumbering) and the parser reads
  ;; the inner expression through the matching `}`.
  (testing "a dynamic key evaluates its expression to the attribute name"
    (is (= {"foo" 42} (:value (pnix/eval-source "let k = \"foo\"; in { ${k} = 42; }"))))
    (is (= {"pre_x" 1} (:value (pnix/eval-source
                                "let n = \"x\"; in { ${\"pre_\" + n} = 1; }")))))
  (testing "a dynamic key can be selected back"
    (is (= 7 (:value (pnix/eval-source "let k = \"a\"; in { ${k} = 7; }.a")))))
  (testing "dynamic and static keys mix in one attrset"
    (is (= {"a" 1 "b" 2} (:value (pnix/eval-source
                                  "let k = \"b\"; in { a = 1; ${k} = 2; }")))))
  (testing "string interpolation is unaffected by the ${ punctuation token"
    (is (= "v=3" (:value (pnix/eval-source "let x = 3; in \"v=${builtins.toString x}\""))))))

(deftest evaluator-let-inherit
  ;; `inherit` in let bindings. inherit-from `(e) a b` resolves e in the
  ;; recursive let scope; plain `inherit x` copies x from the enclosing scope
  ;; (so it must NOT self-reference into a cycle).
  (testing "inherit (e) a b selects from e in the recursive scope"
    (is (= 11 (:value (pnix/eval-source
                       "let s = { a = 5; b = 6; }; in let inherit (s) a b; in a + b"))))
    (is (= 1 (:value (pnix/eval-source
                      "let s = { a = 1; }; inherit (s) a; in a")))))
  (testing "plain inherit copies the enclosing binding without cycling"
    (is (= 9 (:value (pnix/eval-source "let x = 9; in let inherit x; in x"))))
    (is (= 101 (:value (pnix/eval-source
                        "let x = 100; in (y: let inherit x; in x + y) 1")))))
  (testing "inherit mixes with ordinary recursive bindings"
    (is (= 7 (:value (pnix/eval-source
                      "let x = 2; in let inherit x; y = x + 3; in x + y")))))
  (testing "plain inherit with no enclosing binding is held, not looped"
    (let [r (pnix/eval-source "let inherit z; in z")]
      (is (= :failed (:status r)))
      (is (= :unbound-var (:reason r))))))

(deftest evaluator-in-memory-import
  ;; Axis 1, item 4: wire `import <target>` to an in-memory pnix module map
  ;; (no filesystem). Default behavior (no modules) stays held, preserving the
  ;; :import-evaluation-not-wired contract.
  (testing "no modules: import stays held with the wired-default reason"
    (let [r (pnix/eval-source "import ./m")]
      (is (= :failed (:status r)))
      (is (= :import-evaluation-not-wired (:reason r)))))
  (testing "no modules: scopedImport stays held with the same wired-default contract"
    (let [r (pnix/eval-source "scopedImport {} ./m")]
      (is (= :failed (:status r)))
      (is (= :import-evaluation-not-wired (:reason r)))))
  (testing "in-memory module resolves and evaluates"
    (is (= 3 (:value (pnix/eval-source-with-imports "import ./m" {"./m" "1 + 2"}))))
    (is (= 15 (:value (pnix/eval-source-with-imports
                       "(import ./m) + 10" {"./m" "5"}))))
    (is (= 99 (:value (pnix/eval-source-with-imports
                       "let p = import ./m; in p.x" {"./m" "{ x = 99; }"})))))
  (testing "nested import resolves transitively"
    (is (= 7 (:value (pnix/eval-source-with-imports
                      "import ./a" {"./a" "import ./b" "./b" "7"})))))
  (testing "nested relative imports resolve from the importing module"
    (let [modules {"./dir/main.px" "B: { v = import ./sibling.px; }"
                   "./dir/sibling.px" "42"
                   ;; A colliding outer sibling must not win.
                   "./sibling.px" "13"}]
      (is (= 42 (:value (pnix/eval-source-with-imports
                         "((import ./dir/main.px) builtins).v" modules))))
      (is (= :ok (:status
                  (binding [lowering/*import-modules* modules]
                    (lowering/lower-ast
                     (:ast (parser/parse-source
                            "((import ./dir/main.px) builtins).v")))))))
      (let [receipt (pnix/verify-source
                     {:source "((import ./dir/main.px) builtins).v"
                      :import-modules modules})]
        (is (= 42 (get-in receipt [:eval-result :value])))
        (is (= 42 (get-in receipt [:clj-meta-result :value])))
        (is (= 42 (get-in receipt [:px-runtime :value]))))))
  (testing "a returned closure may re-import its own cached module"
    (let [modules {"./a.px" "{ value = 42; f = x: (import ./a.px).value; }"}]
      (is (= 42 (:value (pnix/eval-source-with-imports
                         "(import ./a.px).f 0" modules))))))
  (testing "run-source threads in-memory imports through evaluator and lowering"
    (let [r (pnix/verify-source {:source-id :import/in-memory-host-lanes
                              :source "import ./m"
                              :import-modules {"./m" "1 + 2"}})]
      (is (= :accepted (:status r)))
      (is (= 3 (get-in r [:eval-result :value])))
      (is (= :ok (get-in r [:lowering-result :status])))
      (is (= 3 (get-in r [:clj-meta-result :value])))
      (is (= :ok (get-in r [:clj-meta-result :status])))
      (is (= :ok (get-in r [:px-runtime :status])))
      (is (= :px-runtime-source-execution-ok (get-in r [:px-runtime :reason])))
      (is (= 3 (get-in r [:px-runtime :value])))
      (is (= :mirrors-agree (get-in r [:cross-mirror-verdict :reason])))))
  (testing "scopedImport with an EMPTY scope == plain import (scope FIRST, path
           SECOND -- verified against nix-instantiate 2.34.7; the old fixtures
           had the arguments swapped, encoding the resolved bug)"
    (is (= 3 (:value (pnix/eval-source-with-imports
                      "scopedImport {} ./m" {"./m" "1 + 2"}))))
    (let [r (pnix/verify-source {:source-id :scopedImport/in-memory-host-lanes
                              :source "scopedImport {} ./m"
                              :import-modules {"./m" "1 + 2"}})]
      (is (= :accepted (:status r)))
      (is (= 3 (get-in r [:eval-result :value])))
      (is (= :ok (get-in r [:lowering-result :status])))
      (is (= 3 (get-in r [:clj-meta-result :value])))
      (is (= :ok (get-in r [:clj-meta-result :status])))
      (is (= :ok (get-in r [:px-runtime :status])))
      (is (= :px-runtime-source-execution-ok (get-in r [:px-runtime :reason])))
      (is (= 3 (get-in r [:px-runtime :value])))
      (is (= :mirrors-agree (get-in r [:cross-mirror-verdict :reason])))))

  (testing "run-source propagates in-memory import failures through px-runtime frontier"
    (let [r (pnix/verify-source {:source-id :import/in-memory-missing-runtime
                              :source "import ./missing"
                              :import-modules {"./m" "1"}})]
      (is (= :failed (:status r)))
      (is (= :import-module-not-found (get-in r [:px-runtime :reason])))
      (is (= :import-module-not-found
             (get-in r [:px-runtime :error :class])))))

  (testing "run-source rejects in-memory import cycles through px-runtime frontier"
    (let [r (pnix/verify-source {:source-id :import/in-memory-cycle-runtime
                              :source "import ./a"
                              :import-modules {"./a" "import ./b"
                                              "./b" "import ./a"}})]
      (is (= :failed (:status r)))
      (is (= :import-module-cycle (get-in r [:px-runtime :reason])))
      (is (= :import-cycle
             (get-in r [:px-runtime :error :class])))))
  (testing "lowering import resolver reports missing modules and cycles"
    (let [missing (binding [lowering/*import-modules* {"./m" "1"}]
                    (lowering/lower-ast (:ast (parser/parse-source
                                               "import ./missing"))))
          cycle (binding [lowering/*import-modules* {"./a" "import ./b"
                                                     "./b" "import ./a"}]
                  (lowering/lower-ast (:ast (parser/parse-source
                                             "import ./a"))))]
      (is (= :failed (:status missing)))
      (is (= :import-module-not-found (:reason missing)))
      (is (= :failed (:status cycle)))
      (is (= :import-cycle (:reason cycle)))))
  (testing "lowering scopedImport validates arity, scope, and literal target
           (scope FIRST, path SECOND)"
    (let [lower (fn [src] (binding [lowering/*import-modules* {"./m" "x"}]
                            (lowering/lower-ast (:ast (parser/parse-source src)))))
          ;; empty scope, non-literal path -> target-not-literal
          nonliteral (lower "scopedImport {} (./m + \"\")")
          ;; a non-empty scope now lowers by injection (scope keys become
          ;; force-on-read parameters of the imported module)
          injected (lower "scopedImport { x = 1; } ./m")
          ;; lexical builtins follows the same injection path as any key
          shadow (lower "scopedImport { builtins = 1; } ./m")
          arity (lower "scopedImport ./m")]
      (is (= :failed (:status nonliteral)))
      (is (= :scoped-import-target-not-literal (:reason nonliteral)))
      (is (= :ok (:status injected)))
      (is (= :ok (:status shadow)))
      (is (= :failed (:status arity)))
      (is (= :scoped-import-arity-mismatch (:reason arity)))))
  (testing "select-or keeps import as a callable default, not a swallowed special form"
    (is (= 7 (:value (pnix/eval-source-with-imports
                      "{ a = 1; }.b or import ./m" {"./m" "7"}))))
    (let [r (pnix/eval-source-with-imports
             "{ a = 1; }.a or import ./m" {"./m" "7"})]
      (is (= :failed (:status r)))
      (is (= :call-target-not-callable (:reason r)))))
  (testing "unknown target is held, not crashed"
    (let [r (pnix/eval-source-with-imports "import ./missing" {"./m" "1"})]
      (is (= :failed (:status r)))
      (is (= :import-module-not-found (:reason r)))))
  (testing "import cycle is held, not looped"
    (let [r (pnix/eval-source-with-imports
             "import ./a" {"./a" "import ./b" "./b" "import ./a"})]
      (is (= :failed (:status r)))
      (is (= :import-cycle (:reason r))))))

(deftest evaluator-builtin-breadth-batch-14
  ;; Fourteenth builtin-breadth batch (Axis 1, item 5): count, zipListsWith,
  ;; boolToString.
  (testing "count"
    (is (= 3 (:value (pnix/eval-source "builtins.count (x: x > 2) [1 2 3 4 5]"))))
    (is (= 0 (:value (pnix/eval-source "builtins.count (x: x > 9) [1 2 3]")))))
  (testing "zipListsWith zips up to the shorter list"
    (is (= [11 22 33]
           (:value (pnix/eval-source
                    "builtins.zipListsWith (a: b: a + b) [1 2 3] [10 20 30 40]")))))
  (testing "boolToString"
    (is (= "true" (:value (pnix/eval-source "builtins.boolToString true"))))
    (is (= "false" (:value (pnix/eval-source "builtins.boolToString false"))))))

(deftest evaluator-impure-builtins-are-purity-gated
  ;; Host filesystem/env effects are allowed by default (interactive parity with
  ;; nix-instantiate) but purity-gated when evaluator/*pure-eval* is true.
  (doseq [[source reason effect]
          [["builtins.pathExists \"./x\"" :path-exists-purity-gated :path-exists]
           ["builtins.readFile \"./x\"" :read-file-purity-gated :file-read]
           ["builtins.readDir \"./x\"" :read-dir-purity-gated :directory-read]
           ["builtins.getEnv \"HOME\"" :get-env-purity-gated :env-read]]]
    (let [r (binding [evaluator/*pure-eval* true]
              (pnix/eval-source source))]
      (is (= :failed (:status r)) source)
      (is (= reason (:reason r)) source)
      (is (= effect (get-in r [:error :evidence :effect])) source))))

(deftest lowering-builtin-frontier-notes
  ;; Builtins that remain on the lowering frontier are held with frontier-specific
  ;; reasons. These should not silently fall through as :unsupported-lowering-op.
  (doseq [[source reason]
          [["builtins.getAttr \"a\" { b = 1; }" :get-attr-lowering-not-wired]
           ["builtins.getEnv \"HOME\"" :get-env-purity-gated]
           ["builtins.pathExists \"./x\"" :path-exists-purity-gated]
           ["builtins.pnixMounts 1" :pnix-mounts-extension-not-wired]
           ["builtins.readFile \"./x\"" :read-file-purity-gated]
           ["builtins.readDir \"./x\"" :read-dir-purity-gated]
           ["builtins.unsafeGetAttrPos \"a\" { a = 1; }"
            :unsafe-get-attr-pos-lowering-not-wired]]]
    (let [r (pnix/lower-source source)]
      (is (= :failed (:status r)) source)
      (is (= reason (:reason r)) source))))

(deftest evaluator-domain-extension-stubs-are-not-nix-coverage
  ;; These are pnix/domain extensions, not faithful Nix builtins. Keep them held
  ;; with explicit metadata so they cannot be counted as Nix coverage.
  (doseq [[source reason extension]
          [["builtins.pnixMounts"
            :pnix-mounts-extension-not-wired
            :pnix-mount-runtime]]]
    (let [r (pnix/eval-source source)]
      (is (= :failed (:status r)) source)
      (is (= reason (:reason r)) source)
      (is (= false (get-in r [:error :details :nix-builtin?])) source)
      (is (= extension (get-in r [:error :details :extension])) source))))

(deftest repo-owned-oracle-fixture
  (let [fixture-set (oracle/ground-truth-fixture-set)
        cases (oracle/ground-truth-cases)
        summary (pnix/report cases)]
    (is (= :nix-ground-truth-oracle-set (:kind fixture-set)))
    (is (= "nix-instantiate (Nix) 2.34.7"
           (get-in fixture-set [:lineage :command-version])))
    (is (= 20 (count cases)))
    (is (= 20 (:total summary)))
    (is (= 0 (:rejected summary)))
    (is (= 20 (:accepted summary)))
    (is (= 0 (:held summary)))
    (is (= 42 (get-in (first (:receipts summary)) [:oracle-result :value])))
    (is (= ["a" "b"]
           (get-in (some #(when (= :nix-ground-truth/attr-names
                                   (:source-id %))
                            %)
                         (:receipts summary))
                   [:oracle-result :value])))))

(deftest report-separates-semantic-mismatch-from-held-frontier
  (let [summary (pnix/report [{:source-id :intentional-mismatch
                               :source "42"
                               :oracle-result {:status :ok
                                               :value 41}}])]
    (is (= 1 (:rejected summary)))
    (is (= 0 (:held summary)))
    (is (= :oracle-mismatch (:reason (:first-rejected summary))))
    (is (= 1 (get (:rejected-reason-counts summary) :oracle-mismatch)))
    (is (= :intentional-mismatch
           (:source-id (:first-rejected summary))))))

(deftest runtime-run-plan-is-human-trackable
  (let [plan (px-runtime/runtime-run-plan)
        artifact-roots (set (map :root (px-runtime/runtime-artifacts)))
        imported (set (map :to (:edges plan)))]
    (is (= :px-runtime-run-plan (:kind plan)))
    (is (= :held (:status plan)))
    (is (= "pnix_clj/pnix_runtime" (:resource-root plan)))
    (is (re-find #"resources/pnix_clj/pnix_runtime"
                 (:container-path plan)))
    (is (= "vm.px" (get-in plan [:entry :relative-path])))
    (is (= :px-runtime-boundary (get-in plan [:boundary :kind])))
    (is (= :ok (get-in plan [:boundary :status])))
    (is (= ["pnix-mirror-runtime" "pnixc-pnix" "stdlib"]
           (get-in plan [:boundary :allowed-roots])))
    (is (= true (get-in plan [:boundary :external-runtime-roots-forbidden])))
    (is (= false (get-in plan [:boundary :parent-checkouts-runtime-dependency])))
    (is (= :ok (get-in plan [:entry-parse :status])))
    (is (= :px-runtime-entry-parse-ok (get-in plan [:entry-parse :reason])))
    (is (= :let (get-in plan [:entry-parse :ast-op])))
    (is (= :ok (get-in plan [:bootstrap :status])))
    (is (= :px-runtime-bootstrap-ok (get-in plan [:bootstrap :reason])))
    (is (= 13 (get-in plan [:bootstrap :evaluated-artifact-count])))
    (is (= :pnix-clj.px-runtime.import-cache.v0
           (get-in plan [:bootstrap :import-cache :schema])))
    (is (= :repo-owned-artifact-id
           (get-in plan [:bootstrap :import-cache :policy])))
    (is (= [:root :relative-path]
           (get-in plan [:bootstrap :import-cache :key-fields])))
    (is (= 13 (get-in plan [:bootstrap :import-cache :entry-count])))
    (is (= 13 (get-in plan [:bootstrap :import-cache :miss-count])))
    (is (= 0 (get-in plan [:bootstrap :import-cache :cycle-count])))
    (is (empty? (get-in plan [:bootstrap :import-cycles])))
    (is (= 12 (get-in plan [:bootstrap :value-summary :primitive-count])))
    (is (= true (get-in plan [:bootstrap :value-summary :has-spawn?])))
    (is (= true (get-in plan [:bootstrap :value-summary :has-project?])))
    (is (= true (get-in plan [:bootstrap :value-summary :has-describe?])))
    (is (contains? (set (get-in plan [:bootstrap :value-summary :verbs]))
                   "spawn"))
    (is (contains? (set (get-in plan [:bootstrap :value-summary :verbs]))
                   "project"))
    (is (= :px-runtime-run-plan-ready-source-required (:reason plan)))
    (is (= :pnix-clj.px-runtime.import-graph.v0
           (get-in plan [:import-graph :schema])))
    (is (= true (get-in plan [:import-graph :acyclic?])))
    (is (= 0 (get-in plan [:import-graph :cycle-count])))
    (is (= 0 (get-in plan [:import-graph :missing-edge-count])))
    (is (= :failed-not-recursive-import
           (get-in plan [:import-graph :cycle-policy])))
    (is (>= (:artifact-count plan) 13))
    (is (contains? artifact-roots "pnix-mirror-runtime"))
    (is (contains? artifact-roots "pnixc-pnix"))
    (is (contains? artifact-roots "stdlib"))
    (is (contains? imported "primitives/p1-mirror-identity-registry.px"))
    (is (contains? imported "primitives/p12-mirror-gc-contract.px"))
    (is (empty? (:missing-imports plan)))))

(deftest runtime-import-graph-analysis-detects-cycles
  (let [analysis (px-runtime/import-graph-analysis
                  [{:from-root "root"
                    :from "a.px"
                    :to-root "root"
                    :to "b.px"
                    :status :resolved}
                   {:from-root "root"
                    :from "b.px"
                    :to-root "root"
                    :to "a.px"
                    :status :resolved}
                   {:from-root "root"
                    :from "b.px"
                    :import "./missing.px"
                    :status :missing}])]
    (is (= :pnix-clj.px-runtime.import-graph.v0 (:schema analysis)))
    (is (= false (:acyclic? analysis)))
    (is (= 1 (:cycle-count analysis)))
    (is (= 1 (:missing-edge-count analysis)))
    (is (= :failed-not-recursive-import (:cycle-policy analysis)))
    (is (= [[{:root "root" :relative-path "a.px"}
             {:root "root" :relative-path "b.px"}
             {:root "root" :relative-path "a.px"}]]
           (:cycles analysis)))))

(deftest runtime-import-scanner-includes-scopedImport
  (is (= ["./a"] (px-runtime/imports "import ./a")))
  (is (= ["./a"] (px-runtime/imports "scopedImport ./a {}")))
  (is (= ["./a" "./b"] (px-runtime/imports "import ./a; scopedImport ./b {}")))
  (is (= ["./a" "./b"]
         (px-runtime/imports "import ./a
scopedImport ./b {}")))
  (is (= ["./a" "./b" "./c"]
         (px-runtime/imports "
# line comment
import ./a  # inline comment
scopedImport ./b {}
scopedImport ./c {}"))))

(deftest stage15-control-plan-is-human-trackable
  (let [plan (stage15/control-plan)
        command-ids (set (map :id (:commands plan)))
        compiler-hash (:hash (first (filter #(= "src/pnix/clj_meta/compiler.clj"
                                                (:path %))
                                           (:inputs plan))))
        receipt (pnix/verify-source "42")]
    (is (= :stage15-control-plan (:kind plan)))
    (is (= :held (:status plan)))
    (is (= {:floor 15
            :ceiling :N
            :meaning "clj-meta meta-circular compiler/evaluator stages 15 and above"}
           (:stage-range plan)))
    (is (= [:clojure-mirror
            :pnix-runtime-px
            :pnix-mirror]
           (:mirror-spine plan)))
    (is (= :pnix-runtime-and-pnix-mirror-required
           (:truth-boundary plan)))
    (is (= :read-only-backend (:write-policy plan)))
    (is (= :stage15-gates-not-executed (:reason plan)))
    (is (contains? command-ids :gate))
    (is (contains? command-ids :full-source-stage1))
    (is (= 64 (count compiler-hash)))
    (is (= :stage15-control-plan
           (get-in receipt [:clojure-mirror :stage15-control :kind])))))

(deftest stage15-execution-report-runs-selected-commands
  (let [seen (atom [])
        fake-runner (fn [{:keys [id command purpose]} timeout-ms]
                      (swap! seen conj id)
                      {:id id
                       :command command
                       :purpose purpose
                       :status :ok
                       :reason :stage15-command-ok
                       :exit 0
                       :duration-ms 1
                       :timeout-ms timeout-ms
                       :stdout-hash (apply str (repeat 64 "a"))
                       :stderr-hash (apply str (repeat 64 "b"))})
        report (stage15/execute-plan {:command-ids [:compiler-smoke
                                                    :determinism-policy]
                                      :timeout-ms 1234
                                      :runner fake-runner})]
    (is (= :stage15-execution-report (:kind report)))
    (is (= :pnix-clj.stage15-execution-report.v0 (:schema report)))
    (is (= :ok (:status report)))
    (is (= :stage15-commands-executed (:reason report)))
    (is (= [:compiler-smoke :determinism-policy] @seen))
    (is (= 2 (:selected-command-count report)))
    (is (= 0 (:held-count report)))
    (is (= 1234 (:timeout-ms report)))
    (is (= 64 (count (:receipt-hash report))))))

(deftest rust-grounded-batch-is-repo-owned-and-held
  (let [cases (rust-batch/batch-cases)
        report (rust-batch/report)
        first-receipt (first (:receipts report))
        c02-receipt (first (filter #(= :rust-grounded/c02_strings (:source-id %))
                                   (:receipts report)))
        c04-receipt (first (filter #(= :rust-grounded/c04_attr (:source-id %))
                                   (:receipts report)))
        c03-receipt (first (filter #(= :rust-grounded/c03_list (:source-id %))
                                   (:receipts report)))
        c05-receipt (first (filter #(= :rust-grounded/c05_recurse (:source-id %))
                                   (:receipts report)))
        c06-receipt (first (filter #(= :rust-grounded/c06_nested (:source-id %))
                                   (:receipts report)))
        c07-receipt (first (filter #(= :rust-grounded/c07_builtins (:source-id %))
                                   (:receipts report)))
        c08-receipt (first (filter #(= :rust-grounded/c08_bool (:source-id %))
                                   (:receipts report)))
        c09-receipt (first (filter #(= :rust-grounded/c09_lambda (:source-id %))
                                   (:receipts report)))
        c10-receipt (first (filter #(= :rust-grounded/c10_mixed (:source-id %))
                                   (:receipts report)))
        suite-ids (set (map :id (:required-suites report)))
        suite-source-by-id (into {} (map (juxt :id identity)
                                         (:suite-source-inventory report)))]
    (is (= 10 (count cases)))
    (is (= :rust-grounded-batch-report (:kind report)))
    (is (= :rust-grounded-invariant-manifest (:manifest-kind report)))
    (is (= :rust-grounded-oracle-set (:oracle-kind report)))
    (is (= 3 (:imported-suite-source-count report)))
    (is (= 91 (:imported-rust-test-count report)))
    (is (contains? suite-ids :RUST_EVAL_CORPUS))
    (is (contains? suite-ids :RUST_BUILTIN_CORPUS))
    (is (contains? suite-ids :RUST_OVERFLOW_CORPUS))
    (is (contains? suite-ids :STAGE7_CORE_CASES))
    (is (= [:pnix-clj-evaluator
            :pnix-clj-lowering-clj-meta
            :clojure-stage15-mirror
            :px-runtime-pnix-mirror]
           (:pnix-clj-lanes report)))
    (is (= "pnix_clj/rust_grounded/invariance_corpus"
           (:source-origin report)))
    (is (= "f5ce48fe9a1f7a371d7c9f96a8e0c7366e232437"
           (get-in report [:source-revision :commit])))
    (is (= "bf6344eadc9032486f1983e09123eef2ffec4d2e539bf73c0964f7cff4a718f9"
           (get-in suite-source-by-id [:RUST_EVAL_CORPUS :source-hash])))
    (is (= 41 (get-in suite-source-by-id [:RUST_EVAL_CORPUS :test-count])))
    (is (= 34 (get-in suite-source-by-id [:RUST_BUILTIN_CORPUS :test-count])))
    (is (= 16 (get-in suite-source-by-id [:RUST_OVERFLOW_CORPUS :test-count])))
    (is (every? true? (map :hash-matches?
                           (map suite-source-by-id
                                [:RUST_EVAL_CORPUS
                                 :RUST_BUILTIN_CORPUS
                                 :RUST_OVERFLOW_CORPUS]))))
    (is (contains? (set (get-in suite-source-by-id
                                [:RUST_EVAL_CORPUS :test-names]))
                   "eval_arithmetic"))
    (is (contains? (set (get-in suite-source-by-id
                                [:RUST_BUILTIN_CORPUS :test-names]))
                   "substring_takes_slice_by_char"))
    (is (contains? (set (get-in suite-source-by-id
                                [:RUST_OVERFLOW_CORPUS :test-names]))
                   "builtins_add_overflow_errors_not_panics"))
    (is (= 10 (:fixture-count report)))
    (is (= 10 (:accepted report)))
    (is (= 0 (:rejected report)))
    (is (= 0 (:held report)))
    (is (= 0 (get (:reason-counts report) :rust-grounded-oracle-not-imported 0)))
    (is (= 10 (get (:reason-counts report) :all-lanes-agree)))
    (is (= 0 (get (:reason-counts report) :px-runtime-run-error 0)))
    (is (= 0 (get (:reason-counts report) :px-runtime-run-held 0)))
    (is (= 0 (get (:reason-counts report) :rust-grounded-oracle-unsupported 0)))
    (is (= 0 (get (:reason-counts report) :unsupported-syntax 0)))
    (is (nil? (:first-frontier report)))
    (is (= :let (get-in first-receipt [:ast :op])))
    (is (= :ok (get-in first-receipt [:eval-result :status])))
    (is (= 23 (get-in first-receipt [:eval-result :value "sum"])))
    (is (= 42 (get-in first-receipt [:eval-result :value "prod"])))
    (is (= 23 (get-in first-receipt [:px-runtime :value "sum"])))
    (is (= {"hello" "hello, pnix!"
            "len" 4
            "joined" "a-b-c-pnix"
            "sub" "abc"
            "interp" "n=7 sq=49"}
           (get-in c02-receipt [:eval-result :value])))
    (is (= (get-in c02-receipt [:eval-result :value])
           (get-in c02-receipt [:clj-meta-result :value])))
    (is (= :ok (get-in c04-receipt [:eval-result :status])))
    (is (= {"a" 1 "b" 20 "c" 3}
           (get-in c04-receipt [:eval-result :value "m"])))
    (is (= 3 (get-in c04-receipt [:eval-result :value "pick"])))
    (is (= ["a" "b" "c"]
           (get-in c04-receipt [:eval-result :value "names"])))
    (is (= true (get-in c04-receipt [:eval-result :value "has"])))
    (is (= (get-in c04-receipt [:eval-result :value])
           (get-in c04-receipt [:clj-meta-result :value])))
    (is (= {"len" 20
            "mapped" [1 4 9 16 25 36 49 64 81 100
                      121 144 169 196 225 256 289 324 361 400]
            "filtered" [11 12 13 14 15 16 17 18 19 20]
            "total" 210}
           (get-in c03-receipt [:eval-result :value])))
    (is (= (get-in c03-receipt [:eval-result :value])
           (get-in c03-receipt [:clj-meta-result :value])))
    (is (= (get-in c03-receipt [:eval-result :value])
           (get-in c03-receipt [:px-runtime :value])))
    (is (= {"sum" 125250
            "fib" 6765}
           (get-in c05-receipt [:eval-result :value])))
    (is (= (get-in c05-receipt [:eval-result :value])
           (get-in c05-receipt [:clj-meta-result :value])))
    (is (= (get-in c05-receipt [:eval-result :value])
           (get-in c05-receipt [:px-runtime :value])))
    (let [json-value (get-in c06-receipt [:eval-result :value])]
      (is (str/starts-with? json-value
                            "[{\"id\":1,\"sq\":1,\"tags\":[\"t1\",\"x\"]}"))
      (is (str/includes? json-value
                         "{\"id\":12,\"sq\":144,\"tags\":[\"t12\",\"x\"]}"))
      (is (str/ends-with? json-value "]")))
    (is (= (get-in c06-receipt [:eval-result :value])
           (get-in c06-receipt [:clj-meta-result :value])))
    (is (= (get-in c06-receipt [:eval-result :value])
           (get-in c06-receipt [:px-runtime :value])))
    (is (= {"sorted" [1 2 3 5 8 9]
            "head" 5
            "tail" [2 8 1 9 3]
            "at" 8
            "member" true}
           (get-in c07-receipt [:eval-result :value])))
    (is (= (get-in c07-receipt [:eval-result :value])
           (get-in c07-receipt [:clj-meta-result :value])))
    (is (= ["neg" "zero" "small" "big"]
           (get-in c08-receipt [:eval-result :value])))
    (is (= (get-in c08-receipt [:eval-result :value])
           (get-in c08-receipt [:clj-meta-result :value])))
    (is (= {"c" 21 "a" 3 "curry" 6}
           (get-in c09-receipt [:eval-result :value])))
    (is (= (get-in c09-receipt [:eval-result :value])
           (get-in c09-receipt [:clj-meta-result :value])))
    (is (= {"count" 15
            "total" 240
            "label" "evens-15"
            "squares" {"k2" 4
                       "k4" 16
                       "k6" 36
                       "k8" 64
                       "k10" 100
                       "k12" 144
                       "k14" 196
                       "k16" 256
                       "k18" 324
                       "k20" 400
                       "k22" 484
                       "k24" 576
                       "k26" 676
                       "k28" 784
                       "k30" 900}}
           (get-in c10-receipt [:eval-result :value])))
    (is (= (get-in c10-receipt [:eval-result :value])
           (get-in c10-receipt [:clj-meta-result :value])))
    (is (= :all-lanes-agree (:reason c10-receipt)))
    (is (= :pnix-clj-all-lanes-agree
           (get-in c10-receipt [:oracle-result :authority])))
    (is (= :rust-grounded-oracle-unsupported
           (get-in c10-receipt [:oracle-result :rust-oracle :reason])))
    (is (= :ok (get-in c10-receipt [:px-runtime :status])))
    (is (= (get-in c10-receipt [:eval-result :value])
           (get-in c10-receipt [:px-runtime :value])))
    (is (= "f5ce48fe9a1f7a371d7c9f96a8e0c7366e232437"
           (get-in first-receipt [:source-meta :source-revision :commit])))
    (is (= "f5ce48fe9a1f7a371d7c9f96a8e0c7366e232437"
           (get-in report [:oracle-source-revision :commit])))
    (is (= 64 (count (:fixture-hash (first cases)))))))

(deftest report-artifact-is-persisted-as-edn
  (let [dir (doto (File/createTempFile "pnix-clj-report" "")
              (.delete)
              (.mkdirs))
        artifact (report-artifact/write-report! :smoke (.getPath dir))
        mirror-artifact (report-artifact/write-report! :mirror-pair (.getPath dir))
        forward-artifact (report-artifact/write-report! :forward-reference
                                                         (.getPath dir))
        projection-artifact (report-artifact/write-report! :clojure-projection
                                                           (.getPath dir))
        form-artifact (report-artifact/write-report! :clojure-form
                                                     (.getPath dir))
        determinism-artifact (report-artifact/write-report! :determinism
                                                            (.getPath dir))
        coverage-artifact (report-artifact/write-report! :coverage
                                                         (.getPath dir))
        data (edn/read-string (slurp (:path artifact)))]
    (is (= :smoke (:kind artifact)))
    (is (= 64 (count (:hash artifact))))
    (is (= :pnix-clj-smoke-report (:kind data)))
    (is (= :smoke (:report-artifact/kind data)))
    (is (= 20 (:total data)))
    (is (= 20 (:accepted data)))
    (is (= 0 (:held data)))
    (is (= :mirror-pair (:kind mirror-artifact)))
    (is (= :forward-reference (:kind forward-artifact)))
    (is (= 64 (count (:hash forward-artifact))))
    (is (= :clojure-projection (:kind projection-artifact)))
    (is (= :clojure-form (:kind form-artifact)))
    (is (= :determinism (:kind determinism-artifact)))
    (is (= 64 (count (:hash determinism-artifact))))
    (is (= :coverage (:kind coverage-artifact)))
    (is (= 64 (count (:hash coverage-artifact))))
    (is (= 64 (count (:hash mirror-artifact))))))

(deftest stage7-core-lockins-cross-internal-px-runtime-boundary
  (let [cases (stage7-core/cases)
        report (stage7-core/report)
        receipt-by-id (into {} (map (juxt :source-id identity)
                                    (:receipts report)))]
    (is (= 5 (count cases)))
    (is (= :stage7-core-lockin-report (:kind report)))
    (is (= :stage7-core-lockin-set (:lockin-kind report)))
    (is (= 5 (:fixture-count report)))
    (is (= 5 (:accepted report)))
    (is (= 0 (:rejected report)))
    (is (= 0 (:held report)))
    (is (= 0 (get (:reason-counts report) :px-runtime-run-held 0)))
    (is (= 5 (get (:reason-counts report) :all-lanes-agree)))
    (is (nil? (:first-frontier report)))
    (is (= 42 (get-in receipt-by-id
                      [:stage7-core/lambda :eval-result :value])))
    (is (= (get-in receipt-by-id [:stage7-core/lambda :eval-result :value])
           (get-in receipt-by-id [:stage7-core/lambda :clj-meta-result :value])))
    (is (= 20 (get-in receipt-by-id
                      [:stage7-core/list-builtin :eval-result :value])))
    (is (= 20 (get-in receipt-by-id
                      [:stage7-core/list-builtin :px-runtime :value])))
    (is (= 2 (get-in receipt-by-id
                     [:stage7-core/merge :px-runtime :value])))
    (is (= 2.5 (get-in receipt-by-id
                       [:stage7-core/float-plus :eval-result :value])))
    (is (every? #(= 64 (count (:fixture-hash %))) cases))))


(deftest lane-registry-generated-and-drift-checked
  (testing "committed docs/LANE_REGISTRY.md matches a fresh render"
    (is (= :ok (:status (lane-registry/check)))))
  (testing "lane registry covers the top-level pnix-clj source surface"
    (let [{:keys [status row-count counts]} (lane-registry/check)]
      (is (= :ok status))
      ;; F8 added pnix-clj.weval (proof-only): 71 -> 72
      ;; M7 added pnix-clj.machine (proof-only, the derived abstract machine): 72 -> 73
      ;; Common machine outcome and IO host adapters: 74 -> 76.
      ;; The production checked-i64 kernel is a closed core lane: 76 -> 77.
      ;; The direct clj-meta host executor is core: 77 -> 78.
      ;; The explicit filesystem convenience boundary is core: 78 -> 79.
      ;; The redb adapter remains outside the clean R1 extraction.
      (is (= 79 row-count))
      (is (= {:core 44 :experimental 7 :proof-only 28} counts)))))
