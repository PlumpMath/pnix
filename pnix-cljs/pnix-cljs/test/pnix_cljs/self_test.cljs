(ns pnix-cljs.self-test
  (:require [pnix-cljs.core :as core]))

(def cases
  [["20 + 22" "done" (js/BigInt "42")]
   ["let double = x: x * 2; in double 21" "done" (js/BigInt "42")]
   ["rec { answer = base + 2; base = 40; }.answer" "done" (js/BigInt "42")]
   ["1 / 0" "failed" nil]
   ["missing" "failed" nil]
   ;; Extended builtins (maturity pass 2026-08-11)
   ["builtins.pow 2 10" "done" (js/BigInt "1024")]
   ["builtins.bitAnd 14 6" "done" (js/BigInt "6")]
   ["builtins.and true false" "done" false]
   ["builtins.eq 1 1" "done" true]
   ["builtins.lt 1 2" "done" true]
   ["builtins.merge { a = 1; } { b = 2; }" "done" {"a" (js/BigInt "1") "b" (js/BigInt "2")}]
   ["builtins.drop 2 [ 1 2 3 4 ]" "done" [(js/BigInt "3") (js/BigInt "4")]]
   ["builtins.take 2 [ 1 2 3 4 ]" "done" [(js/BigInt "1") (js/BigInt "2")]]
   ["builtins.zip [ 1 2 ] [ 3 4 ]"
    "done" [[(js/BigInt "1") (js/BigInt "3")] [(js/BigInt "2") (js/BigInt "4")]]]
   ["builtins.compareVersions \"1.2\" \"1.3\"" "done" (js/BigInt "-1")]
   ["builtins.dirOf \"/a/b/c\"" "done" "/a/b"]
   ["builtins.toInt \"42\"" "done" (js/BigInt "42")]
   ["builtins.groupBy (x: \"k\") [ 1 2 ]"
    "done" {"k" [(js/BigInt "1") (js/BigInt "2")]}]
   ["builtins.functionArgs ({ a, b ? 1 }: a)" "done" {"a" false "b" true}]
   ["builtins.concatStrings [ \"a\" \"b\" ]" "done" "ab"]
   ["builtins.seq 1 2" "done" (js/BigInt "2")]
   ["builtins.storePath \"/x\"" "failed" nil]
   ["builtins.filterAttrsRecursive (name: value: name == \"license\") { meta = { license = \"MIT\"; }; }"
    "done" {}]
   ["builtins.filterAttrsRecursive (name: value: name == \"keep\" || name == \"meta\") { meta = { license = \"MIT\"; }; keep = 1; }"
    "done" {"keep" (js/BigInt "1") "meta" {}}]])

(defn -main [& _]
  (doseq [[source expected-kind expected-value] cases]
    (let [result (core/projection source)]
      (when-not (= expected-kind (get result "outcome_kind"))
        (throw (js/Error. (str "unexpected outcome for " source))))
      (when (and (= "done" expected-kind)
                 (not= expected-value (get result "value")))
        (throw (js/Error. (str "unexpected value for " source))))))
  (println "pnix-cljs self-test: PASS"))

(set! *main-cli-fn* -main)
