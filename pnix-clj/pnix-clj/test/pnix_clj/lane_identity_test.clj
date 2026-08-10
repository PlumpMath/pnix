(ns pnix-clj.lane-identity-test
  (:require [clojure.test :refer [deftest is testing]]
            [pnix-clj.core :as core]
            [pnix-clj.evaluator :as evaluator]
            [pnix-clj.interop :as interop]
            [pnix-clj.nrepl :as nrepl]
            [pnix-clj.wiki :as wiki]
            [pnix-clj.generate :as generate]
            [pnix-clj.self-improve :as self-improve]
            [pnix-clj.self-mod-gate :as self-mod-gate]
            [pnix-clj.synthesize :as synthesize]
            [pnix-clj.tower :as tower]
            [pnix-clj.futamura :as futamura]
            [pnix-clj.mirror-pair :as mirror-pair]))

(deftest metacircular-identity-surfaces-are-core
  (testing "nREPL, wiki, interop, core, and evaluator are pnix-clj identity surfaces"
    (is (= :core (:lane core/lane-classification)))
    (is (= :public-runtime-orchestration (:scope core/lane-classification)))

    (is (= :core (:lane evaluator/lane-classification)))
    (is (= :semantic-evaluator (:scope evaluator/lane-classification)))
    (is (= :core-evaluator (:semantic-authority evaluator/lane-classification)))

    (is (= :core (:lane interop/lane-classification)))
    (is (= :meta-circular-runtime-interop-boundary
           (:scope interop/lane-classification)))
    (is (= :stage15-to-N-compiler-evaluator-interpreter
           (:clojure-runtime interop/lane-classification)))
    (is (= :runtime-compiler-evaluator-interpreter
           (:pnix-runtime interop/lane-classification)))

    (is (= :core (:lane nrepl/lane-classification)))
    (is (= :meta-circular-interactive-control-surface
           (:scope nrepl/lane-classification)))
    (is (= :eval-routes-through-core-only
           (:semantic-authority nrepl/lane-classification)))

    (is (= :core (:lane wiki/lane-classification)))
    (is (= :self-documenting-capability-and-roadmap-substrate
           (:scope wiki/lane-classification)))
    (is (= :documentation-index-only
           (:semantic-authority wiki/lane-classification)))))

(deftest experimental-self-change-lanes-remain-non-admitting
  (testing "self-improve/generate/synthesize/self-mod stay experimental and cannot admit product behavior"
    (doseq [[label lc] {:generate generate/lane-classification
                       :self-improve self-improve/lane-classification
                       :self-mod-gate self-mod-gate/lane-classification
                       :synthesize synthesize/lane-classification}]
      (is (= :experimental (:lane lc)) label)
      (is (not= :allowed (:product-runtime lc)) label)
      (is (not= :admitted (:admission lc)) label)
      (is (= :forbidden (:auto-promotion lc)) label))))

(deftest proof-evidence-lanes-remain-proof-only
  (testing "tower/futamura/mirror-pair are evidence lanes, not product runtimes"
    (doseq [[label lc] {:tower tower/lane-classification
                       :futamura futamura/lane-classification
                       :mirror-pair mirror-pair/lane-classification}]
      (is (= :proof-only (:lane lc)) label)
      (is (= :forbidden (:product-runtime lc)) label)
      (is (not= :core-evaluator (:semantic-authority lc)) label))))
