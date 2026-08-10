(ns pnix.clr-meta.main-test
  (:require [clojure.test :refer [deftest is testing]]
            [pnix.clr-meta.main :as main]))

(defn- failure
  [source]
  (:error (main/evaluate-source source)))

(deftest tool-evaluation-uses-the-admitted-generation
  (is (= {:schema :pnix.clr-meta.tool-eval.v1
          :outcome-kind :done
          :execution :evaluator-generation-2
          :value 42}
         (main/evaluate-source "(+ 20 22)"))))

(deftest tool-reader-fails-closed-before-evaluation
  (testing "reader evaluation and multiple forms are rejected"
    (is (= {:phase :tool-read :class :reader-form-rejected}
           (select-keys (failure "#=(str \"HOST-EVAL\")")
                        [:phase :class])))
    (is (= {:phase :tool-read :class :trailing-tool-form}
           (select-keys (failure "42 43") [:phase :class])))
    (is (= {:phase :tool-read :class :empty-tool-source}
           (select-keys (failure "  ; empty\n") [:phase :class]))))
  (testing "tagged and conditional reader objects are rejected"
    (doseq [source ["#inst \"2026-07-14\""
                    "#uuid \"00000000-0000-0000-0000-000000000000\""
                    "#example/tag [1]"
                    "#?(:cljr 42 :default 0)"]]
      (is (= :tool-read (:phase (failure source))))))
  (testing "host-created values outside the documented portable domain fail"
    (doseq [source ["{:x 1}" "#{1 2}" "#\"a+\"" "\\a"]]
      (is (= {:phase :tool-read :class :non-portable-reader-value}
             (select-keys (failure source) [:phase :class]))))))
