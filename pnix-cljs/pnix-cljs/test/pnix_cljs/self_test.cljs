(ns pnix-cljs.self-test
  (:require [pnix-cljs.core :as core]))

(def cases
  [["20 + 22" "done" (js/BigInt "42")]
   ["let double = x: x * 2; in double 21" "done" (js/BigInt "42")]
   ["rec { answer = base + 2; base = 40; }.answer" "done" (js/BigInt "42")]
   ["1 / 0" "failed" nil]
   ["missing" "failed" nil]])

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
