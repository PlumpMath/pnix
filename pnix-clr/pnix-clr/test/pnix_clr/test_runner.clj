(ns pnix-clr.test-runner
  (:require [clojure.test :refer [deftest is run-tests testing]]
            [pnix-clr.evaluator :as evaluator]
            [pnix-clr.host :as host]
            [pnix-clr.json :as json]
            [pnix-clr.outcome :as outcome]
            [pnix-clr.parser :as parser]))

(def ^:private test-root (atom nil))

(defn- root
  []
  (or @test-root (host/default-root)))

(defn- source-result
  [source]
  (evaluator/eval-source
   source
   {:root (root)
    :file (host/combine (root) "corpus" "conformance" "seed-test.px")}))

(defn- result-value
  [result]
  (outcome/value-of result))

(defn- result-error
  [result]
  (outcome/error-of result))

(defn- structured-ex-data
  [f]
  (try
    (f)
    nil
    (catch System.Exception error
      (ex-data error))))

(deftest boolean-language-seed
  (testing "booleans, precedence, strict negation, equality and inequality"
    (is (= [true true false true false]
           (result-value
            (source-result
             "[ (!false) (true || false && false) (true && false) (true != false) (true == false) ]")))))
  (testing "non-boolean operators fail structurally"
    (let [result (source-result "! [ true ]")]
      (is (outcome/failed? result))
      (is (= "type-error" (:class (result-error result)))))))

(deftest recursive-bindings-lambdas-and-selection
  (is (= true
         (result-value
          (source-result
           "let b = rec { id = x: x; x = true; y = x; }; in (b.id b.y)")))))

(deftest with-float-and-structural-equality
  (testing "with scopes names from an attrset"
    (is (= true (result-value (source-result "with { a = true; }; a")))))
  (testing "float literals and typeOf"
    (is (= "float" (result-value (source-result "builtins.typeOf 1.5")))))
  (testing "structural equality for lists and attrsets"
    (is (= true (result-value (source-result "[1 2] == [1 2]"))))
    (is (= true (result-value (source-result "{ a = 1; } == { a = 1; }"))))
    (is (= false (result-value (source-result "{ a = 1; } == { a = 2; }"))))))

(deftest assert-and-inherit
  (testing "assert passes and returns body"
    (is (= 42 (result-value (source-result "assert true; 42")))))
  (testing "assert failure is structural"
    (let [result (source-result "assert false; 1")]
      (is (outcome/failed? result))
      (is (= "assertion-failed" (:class (result-error result))))))
  (testing "assert requires boolean condition"
    (let [result (source-result "assert 1; 1")]
      (is (outcome/failed? result))
      (is (= "non-boolean-condition" (:class (result-error result))))))
  (testing "inherit from enclosing scope into attrset"
    (is (= 7 (result-value
              (source-result "let x = 7; in { inherit x; }.x")))))
  (testing "inherit into let"
    (is (= 9 (result-value
              (source-result "let x = 9; in let inherit x; in x")))))
  (testing "inherit (expr) names"
    (is (= 3 (result-value
              (source-result
               "let s = { a = 3; b = 4; }; in { inherit (s) a; }.a")))))
  (testing "rec inherit binds from outside the rec frame"
    (is (= 5 (result-value
              (source-result
               "let x = 5; in (rec { inherit x; y = x; }).y"))))))

(deftest unsupported-and-import-errors-are-failed
  (let [missing (source-result "import ./definitely-missing.px")
        escaped-root (source-result "import ../../../outside-root.px")
        applied-import (parser/parse-source "import ./module.px 1")]
    (is (= :failed (outcome/kind missing)))
    (is (= "import-module-not-found" (:class (result-error missing))))
    (is (= "module-not-found"
           (get-in (result-error missing) [:evidence :reason])))
    (is (= :failed (outcome/kind escaped-root)))
    (is (= "import-module-not-found" (:class (result-error escaped-root))))
    (is (= "path-outside-root"
           (get-in (result-error escaped-root) [:evidence :reason])))
    (is (= :call (:op applied-import)))
    (is (= :import (get-in applied-import [:function :op])))
    (is (= :path (get-in applied-import [:function :target :op])))
    (is (= :int (get-in applied-import [:argument :op])))))

(deftest production-basic-mechanisms
  (testing "integers, checked arithmetic, division and lazy if"
    (is (= [3 3 42]
           (result-value
            (source-result
             "[ (1 + 2) (7 / 2) (if true then 42 else missing) ]")))))
  (testing "the stable production classes are retained"
    (doseq [[source class-name]
            [["1 / 0" "division-by-zero"]
             ["9223372036854775807 + 1" "integer-overflow"]
             ["if 1 then 2 else 3" "non-boolean-condition"]
             ["1 + false" "type-error"]]]
      (is (= class-name (:class (result-error (source-result source))))))))

(deftest checked-i64-arithmetic-precedence-and-host-range
  (let [min-expr "(0 - 9223372036854775807 - 1)"
        values
        (result-value
         (source-result
          (str "[ (1 - 2 * 3) (10 - 3 - 2) ((-7) * (-6)) "
               "((-7) / 3) (7 / (-3)) ((-7) / (-3)) (-2 * 3) "
               "(" min-expr " * 1) "
               "(3037000499 * 3037000499) ]")))]
    (is (= [-5 5 42 -2 -2 2 -6
            System.Int64/MinValue 9223372030926249001]
           values))
    (is (every? #(instance? System.Int64 %) values)))
  (is (= 1
         (result-value
          (source-result "let f = x: x; in - f 2 + 3"))))
  (is (= 4
         (result-value
          (source-result "let x-1 = 4; in x-1"))))
  (is (= 4 (result-value (source-result "7-3"))))
  (let [ast (parser/parse-source "let f = x: x; in - f 2 + 3")]
    (is (= :plus (get-in ast [:body :operator])))
    (is (= :negate (get-in ast [:body :left :op])))
    (is (= :call (get-in ast [:body :left :value :op]))))
  (let [ast (parser/parse-source "let x = { a = 1; }; in - x ? a")]
    (is (= :has-attr (get-in ast [:body :op])))
    (is (= :negate (get-in ast [:body :target :op]))))
  (let [bare (source-result "[ -1 ]")]
    (is (= :failed (outcome/kind bare)))
    (is (= "syntax-error" (:class (result-error bare)))))
  (is (= [-1] (result-value (source-result "[ (-1) ]"))))
  (let [literal-min (source-result "(-9223372036854775808)")]
    (is (= :failed (outcome/kind literal-min)))
    (is (= "syntax-error" (:class (result-error literal-min))))))

(deftest checked-i64-errors-are-structured-and-left-strict
  (let [min-expr "(0 - 9223372036854775807 - 1)"
        cases
        [["9223372036854775807 + 1" "integer-overflow"]
         [(str min-expr " + (-1)") "integer-overflow"]
         [(str min-expr " - 1") "integer-overflow"]
         ["9223372036854775807 - (-1)" "integer-overflow"]
         ["9223372036854775807 * 2" "integer-overflow"]
         [(str min-expr " * 2") "integer-overflow"]
         [(str min-expr " * (-1)") "integer-overflow"]
         ["9223372036854775807 * (-2)" "integer-overflow"]
         ["3037000500 * 3037000500" "integer-overflow"]
         ["(-3037000500) * (-3037000500)" "integer-overflow"]
         [(str min-expr " / (-1)") "integer-overflow"]
         [(str "- " min-expr) "integer-overflow"]
         ["1 / 0" "division-by-zero"]
         [(str min-expr " / 0") "division-by-zero"]]]
    (doseq [[source class-name] cases]
      (let [result (source-result source)]
        (is (= :failed (outcome/kind result)))
        (is (= "eval" (:phase (result-error result))))
        (is (= class-name (:class (result-error result))))))
    (doseq [source ["- false" "1 - false" "1 * false" "false / 1"]]
      (is (= "type-error"
             (:class (result-error (source-result source))))))
    (is (= "division-by-zero"
           (:class
            (result-error
             (source-result
              "(1 / 0) * (9223372036854775807 + 1)")))))))

(deftest checked-i64-overflow-remains-lazy
  (is (= [7 8]
         (result-value
          (source-result
           (str "let addOverflow = 9223372036854775807 + 1; "
                "subOverflow = (0 - 9223372036854775807 - 1) - 1; "
                "mulOverflow = 9223372036854775807 * 2; "
                "in [ 7 (if false then mulOverflow else 8) ]"))))))

(deftest equality-scalars-lists-and-attrsets
  (is (= [true true true true true true false false true]
         (result-value
          (source-result
           (str "[ (1 == 1) (1 != 2) (\"x\" == \"x\") "
                "(\"x\" != \"y\") (null == null) (null != false) "
                "(1 == true) (1 == \"1\") ((1 == 1) == true) ]")))))
  ;; Structural == is admitted for lists and attrsets (peer parity).
  (is (= true (result-value (source-result "[ 1 ] == [ 1 ]"))))
  (is (= true (result-value (source-result "{ a = 1; } == { a = 1; }"))))
  (is (= false (result-value (source-result "{ a = 1; } == { a = 2; }"))))
  (let [result (source-result "1 == 1 == true")]
    (is (= :failed (outcome/kind result)))
    (is (= "parse" (:phase (result-error result))))
    (is (= "syntax-error" (:class (result-error result))))))

(deftest static-hasattr-application-and-not-precedence
  (let [ast (parser/parse-source "make null ? nested.leaf")]
    (is (= :has-attr (:op ast)))
    (is (= ["nested" "leaf"] (:path ast)))
    (is (= :call (get-in ast [:target :op])))
    (is (= :null (get-in ast [:target :argument :op]))))
  (is (= [true true true false false false true]
         (result-value
          (source-result
           (str "let make = ignored: { present = 1; nested = { leaf = true; }; }; "
                "in [ (make null ? present) (make null ? nested.leaf) "
                "((make null).present == 1) (1 ? a) "
                "({ a = 1; } ? a.b) (! { a = 1; } ? a) (! false ? a) ]"))))))

(deftest static-hasattr-forces-intermediate-but-not-final-values
  (let [temp-root
        (host/combine
         (System.IO.Path/GetTempPath)
         (str "pnix-clr-hasattr-" (System.Guid/NewGuid)))
        entry (host/combine temp-root "entry.px")
        probe (host/combine temp-root "probe.px")
        terminal (host/combine temp-root "terminal.px")
        resolves (atom 0)
        reads (atom 0)
        original-resolve host/resolve-import
        original-read host/read-source]
    (System.IO.Directory/CreateDirectory temp-root)
    (try
      (System.IO.File/WriteAllText
       probe "{ leaf = import ./terminal.px; }")
      (System.IO.File/WriteAllText terminal "true")
      (with-redefs
        [host/resolve-import
         (fn [& args]
           (swap! resolves inc)
           (apply original-resolve args))
         host/read-source
         (fn [path]
           (swap! reads inc)
           (original-read path))]
        (is (= true
               (result-value
                (evaluator/eval-source
                 "{ poison = import ./probe.px; } ? poison"
                 {:root temp-root :file entry}))))
        (is (= [0 0] [@resolves @reads]))
        (reset! resolves 0)
        (reset! reads 0)
        (is (= true
               (result-value
                (evaluator/eval-source
                 "{ nested = import ./probe.px; } ? nested.leaf"
                 {:root temp-root :file entry}))))
        (is (= [1 1] [@resolves @reads]))
        (reset! resolves 0)
        (reset! reads 0)
        (is (= true
               (result-value
                (evaluator/eval-source
                 "{ nested = import ./probe.px; }.nested.leaf"
                 {:root temp-root :file entry}))))
        (is (= [2 2] [@resolves @reads]))
        (reset! resolves 0)
        (reset! reads 0)
        (is (= false
               (result-value
                (evaluator/eval-source
                 "{ nested = import ./probe.px; } ? missing.leaf"
                 {:root temp-root :file entry}))))
        (is (= [0 0] [@resolves @reads])))
      (finally
        (System.IO.Directory/Delete temp-root true)))))

(deftest nominal-machine-outcomes-cannot-be-forged-by-guest-data
  (let [receipt (outcome/self-check)
        guest (result-value
               (source-result "{ status = \"held\"; value = 42; }"))]
    (is (true? (get receipt "all_ok")))
    (is (= outcome/schema (get receipt "schema")))
    (is (false? (get receipt "guest_shape_is_outcome")))
    (is (= {"status" "held" "value" 42} guest))
    (is (not (outcome/machine-outcome? guest)))))

(deftest canonical-json-is-sorted-and-valid-for-control-characters
  (is (= "{\"a\":\"line\\n\\u0000\",\"z\":true}"
         (json/write-json {"z" true "a" (str "line\n" (char 0))})))
  (is (= (str "{\"z\":0,\"" (char 0x00e4) "\":1}")
         (json/write-json {(str (char 0x00e4)) 1 "z" 0})))
  (let [bmp (str (char 0xe000))
        supplementary (System.Char/ConvertFromUtf32 0x10000)]
    (is (= (str "{\"" bmp "\":1,\"" supplementary "\":2}")
           (json/write-json {supplementary 2 bmp 1}))))
  (let [composed (str (char 0x00e9))
        decomposed (str "e" (char 0x0301))
        realized
        (evaluator/realize-value
         {:pnix/type :attrset
          :entries {composed 1 decomposed 2}})]
    (is (= 2 (count realized)))
    (is (= 1 (get realized composed)))
    (is (= 2 (get realized decomposed))))
  ;; Finite floats project as invariant-culture JSON numbers; NaN/Inf stay closed.
  (is (= "1.5" (json/write-json 1.5)))
  (let [result (outcome/capture #(json/write-json Double/NaN))]
    (is (= :failed (outcome/kind result)))
    (is (= "invalid-guest-value" (:class (result-error result))))
    (is (= "json-noncanonical-number"
           (get-in (result-error result) [:evidence :reason])))))

(deftest unexpected-host-exceptions-remain-infrastructure-failures
  (is (thrown? System.Exception
               (outcome/capture
                #(throw (System.Exception. "must remain loud"))))))

(deftest dead-import-mechanisms-never-resolve-or-read
  (let [temp-root
        (host/combine
         (System.IO.Path/GetTempPath)
         (str "pnix-clr-dead-import-" (System.Guid/NewGuid)))
        entry (host/combine temp-root "entry.px")
        probe (host/combine temp-root "probe.px")
        resolves (atom 0)
        reads (atom 0)
        original-resolve host/resolve-import
        original-read host/read-source
        cases
        [["dead if branch"
          "if true then 1 else import ./probe.px"
          1
          "if false then 1 else import ./probe.px"]
         ["unused function argument"
          "(x: 2) (import ./probe.px)"
          2
          "(x: x) (import ./probe.px)"]
         ["unselected attrset field"
          "{ good = 3; bad = import ./probe.px; }.good"
          3
          "{ good = 3; bad = import ./probe.px; }.bad"]]]
    (System.IO.Directory/CreateDirectory temp-root)
    (try
      (System.IO.File/WriteAllText probe "41")
      (with-redefs
        [host/resolve-import
         (fn [& args]
           (swap! resolves inc)
           (apply original-resolve args))
         host/read-source
         (fn [path]
           (swap! reads inc)
           (original-read path))]
        (doseq [[label dead-source dead-value live-source] cases]
          (testing label
            (reset! resolves 0)
            (reset! reads 0)
            (is (= dead-value
                   (result-value
                    (evaluator/eval-source
                     dead-source
                     {:root temp-root :file entry}))))
            (is (= [0 0] [@resolves @reads]))
            (reset! resolves 0)
            (reset! reads 0)
            (is (= 41
                   (result-value
                    (evaluator/eval-source
                     live-source
                     {:root temp-root :file entry}))))
            (is (= [1 1] [@resolves @reads])))))
      (finally
        (System.IO.Directory/Delete temp-root true)))))

(deftest imports-are-path-cached-and-only-active-cycles-fail
  (let [temp-root
        (host/combine
         (System.IO.Path/GetTempPath)
         (str "pnix-clr-import-" (System.Guid/NewGuid)))
        module (host/combine temp-root "module.px")
        failed-module (host/combine temp-root "failed.px")
        cycle-a (host/combine temp-root "cycle-a.px")
        cycle-b (host/combine temp-root "cycle-b.px")
        reads (atom {})
        original-read host/read-source]
    (System.IO.Directory/CreateDirectory temp-root)
    (try
      (System.IO.File/WriteAllText
       module
       "{ value = 1; again = (import ./module.px).value; }")
      (System.IO.File/WriteAllText failed-module "1 / 0")
      (System.IO.File/WriteAllText cycle-a "import ./cycle-b.px")
      (System.IO.File/WriteAllText cycle-b "import ./cycle-a.px")
      (with-redefs
        [host/read-source
         (fn [path]
           (let [path (host/canonical-path path)]
             (swap! reads update path (fnil inc 0))
             (original-read path)))]
        (let [cached (evaluator/eval-file temp-root module)
              cycle (evaluator/eval-file temp-root cycle-a)
              modules (atom {})
              first-failure
              (structured-ex-data
               #(evaluator/eval-file* temp-root failed-module modules))
              second-failure
              (structured-ex-data
               #(evaluator/eval-file* temp-root failed-module modules))]
          (is (= {"again" 1 "value" 1} (result-value cached)))
          (is (= 1 (get @reads (host/canonical-path module))))
          (is (= :failed (outcome/kind cycle)))
          (is (= "resolution" (:phase (result-error cycle))))
          (is (= "import-cycle" (:class (result-error cycle))))
          (is (= :division-by-zero (::outcome/class first-failure)))
          (is (= (::outcome/class first-failure)
                 (::outcome/class second-failure)))
          (is (= 1 (get @reads (host/canonical-path failed-module))))))
      (finally
        (System.IO.Directory/Delete temp-root true)))))

(deftest builtins-and-lib-surface
  (testing "builtins.typeOf"
    (is (= "int" (result-value (source-result "builtins.typeOf 1"))))
    (is (= "string" (result-value (source-result "builtins.typeOf \"x\""))))
    (is (= "bool" (result-value (source-result "builtins.typeOf true"))))
    (is (= "null" (result-value (source-result "builtins.typeOf null"))))
    (is (= "list" (result-value (source-result "builtins.typeOf [ 1 2 ]"))))
    (is (= "set" (result-value (source-result "builtins.typeOf { a = 1; }"))))
    (is (= "lambda" (result-value (source-result "builtins.typeOf (x: x)")))))
  (testing "lib.head and lib.sum"
    (is (= 1 (result-value (source-result "lib.head [ 1 2 3 ]"))))
    (is (= 10 (result-value (source-result "lib.sum [ 1 2 3 4 ]")))))
  (testing "nested attr path + getAttrFromPath"
    (is (= 42
           (result-value
            (source-result
             "builtins.getAttrFromPath [ \"foo\" \"bar\" ] { foo.bar = 42; }"))))
    (is (= 42
           (result-value
            (source-result
             "let s = { foo.bar = 42; }; in s.foo.bar"))))
    (let [ast (parser/parse-source "{ foo.bar = 42; }")]
      (is (= :attrset (:op ast)))
      (is (contains? (:entries ast) "foo"))
      (is (= :attrset (get-in ast [:entries "foo" :op])))
      (is (contains? (get-in ast [:entries "foo" :entries]) "bar")))))

(deftest extended-builtins-maturity-pass
  ;; Oracle: nix-instantiate 2.34.7, or the reference-host implementation where
  ;; the builtin is a pnix extension not present in real Nix (pow/sqrt/exp/ln/
  ;; sin/cos/atan2).
  (testing "math extensions"
    (is (= 1024 (result-value (source-result "builtins.pow 2 10"))))
    (is (= 6 (result-value (source-result "builtins.bitAnd 14 6"))))
    (is (= 14 (result-value (source-result "builtins.bitOr 8 6"))))
    (is (= 8 (result-value (source-result "builtins.bitXor 14 6")))))
  (testing "logic/compare aliases"
    (is (= true (result-value (source-result "builtins.and true true"))))
    (is (= false (result-value (source-result "builtins.and true false"))))
    (is (= true (result-value (source-result "builtins.or false true"))))
    (is (= false (result-value (source-result "builtins.not true"))))
    (is (= true (result-value (source-result "builtins.eq 1 1"))))
    (is (= true (result-value (source-result "builtins.lt 1 2"))))
    (is (= true (result-value (source-result "builtins.le 2 2"))))
    (is (= true (result-value (source-result "builtins.gt 2 1"))))
    (is (= true (result-value (source-result "builtins.ge 2 2")))))
  (testing "attrset helpers"
    (is (= ["a" "b"] (result-value (source-result "builtins.keys { a = 1; b = 2; }"))))
    (is (= [1 2] (result-value (source-result "builtins.values { a = 1; b = 2; }"))))
    (is (= {"a" 1 "b" 2} (result-value (source-result "builtins.merge { a = 1; } { b = 2; }"))))
    (is (= {"a" "a" "b" "b"}
           (result-value (source-result "builtins.genAttrs [\"a\" \"b\"] (n: n)"))))
    (is (= {"name" "n" "value" 1}
           (result-value (source-result "builtins.nameValuePair \"n\" 1"))))
    (is (= ["a"] (result-value (source-result "builtins.mapAttrsToList (n: v: n) { a = 1; }"))))
    (is (= {"a" {"b" 1}}
           (result-value (source-result "builtins.mapAttrsRecursive (p: v: v) { a = { b = 1; }; }"))))
    (is (= 1 (result-value (source-result "builtins.getAttrFromPathOr { a = 1; } [\"a\"] 9"))))
    (is (= 9 (result-value (source-result "builtins.getAttrFromPathOr { a = 1; } [\"z\"] 9"))))
    (is (= "foo" (result-value (source-result "builtins.getName \"foo-1.0\""))))
    (is (= "1.0" (result-value (source-result "builtins.getVersion \"foo-1.0\""))))
    (is (= {"a" false "b" true}
           (result-value (source-result "builtins.functionArgs ({ a, b ? 1 }: a)")))))
  (testing "list helpers"
    (is (= [3 4] (result-value (source-result "builtins.drop 2 [ 1 2 3 4 ]"))))
    (is (= [1 2] (result-value (source-result "builtins.take 2 [ 1 2 3 4 ]"))))
    (is (= [0 1 2] (result-value (source-result "builtins.cons 0 [ 1 2 ]"))))
    (is (= [1 2 3 4] (result-value (source-result "builtins.append [ 1 2 ] [ 3 4 ]"))))
    (is (= [[1 3] [2 4]] (result-value (source-result "builtins.zip [ 1 2 ] [ 3 4 ]"))))
    (is (= [3 2 1] (result-value (source-result "builtins.reverseList [ 1 2 3 ]"))))
    (is (= ["x" "x" "x"] (result-value (source-result "builtins.replicate 3 \"x\""))))
    (is (= 2 (result-value (source-result "builtins.findFirst (x: x > 1) null [ 1 2 3 ]"))))
    (is (= [0 1] (result-value (source-result "builtins.imap0 (i: v: i) [ 1 2 ]"))))
    (is (= [1 2] (result-value (source-result "builtins.imap1 (i: v: i) [ 1 2 ]"))))
    (is (= {"k" [1 2]} (result-value (source-result "builtins.groupBy (x: \"k\") [ 1 2 ]")))))
  (testing "string/version helpers"
    (is (= -1 (result-value (source-result "builtins.compareVersions \"1.2\" \"1.3\""))))
    (is (= ["1" "2" "3"] (result-value (source-result "builtins.splitVersion \"1.2.3\""))))
    (is (= "/a/b" (result-value (source-result "builtins.dirOf \"/a/b/c\""))))
    (is (= "c" (result-value (source-result "builtins.baseNameOf \"/a/b/c\""))))
    (is (= 42 (result-value (source-result "builtins.toInt \"42\""))))
    (is (= true (result-value (source-result "builtins.hasInfix \"b\" \"abc\""))))
    (is (= "ab" (result-value (source-result "builtins.concatStrings [ \"a\" \"b\" ]")))))
  (testing "misc"
    (is (= [] (result-value
               (source-result "builtins.genericClosure { startSet = []; operator = x: []; }"))))
    (is (= :failed (outcome/kind (source-result "builtins.storePath \"/x\""))))))

(defn -main
  [& args]
  (reset! test-root (host/canonical-path (or (first args)
                                             (host/default-root))))
  (let [{:keys [fail error]} (run-tests 'pnix-clr.test-runner)]
    (host/exit! (if (zero? (+ fail error)) 0 1))))
