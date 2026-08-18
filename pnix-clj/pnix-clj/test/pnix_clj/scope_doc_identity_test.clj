(ns pnix-clj.scope-doc-identity-test
  (:require [clojure.string :as str]
            [clojure.test :refer [deftest is testing]]))

(defn- doc [path]
  (slurp path))

(defn- has-all? [text xs]
  (every? #(str/includes? text %) xs))

(deftest scope-documents-preserve-identity
  (testing "README and SCOPE_LOCK keep the core boundary"
    (is (has-all? (doc "README.md")
                  ["Clojure 호스팅 pnix" "NL/MSV/Hangul" "코어 범위 밖"]))
    (is (has-all? (doc "SCOPE_LOCK.md")
                  ["Clojure 호스팅 pnix 런타임"
                   "메타원형 증인 substrate"
                   "Hangul codec"
                   "MSV"
                   "gate-graph"
                   "multi-language emit registry"
                   "tick runner"])))

  (testing "LANE_CLASSIFICATION keeps current identity lock"
    (is (has-all? (doc "LANE_CLASSIFICATION.md")
                  ;; the pnix-meta loader row was retired from the live ns
                  ;; inventory (see docs/LANE_REGISTRY.md history): 79 -> 78
                  ["현재 정체성 잠금 부록"
                   "CORE: 43"
                   "EXPERIMENTAL: 7"
                   "PROOF-ONLY: 28"
                   "TOTAL: 78"
                   "interop"
                   "Clojure 런타임"
                   "pnix 런타임"
                   "nREPL"
                   "메타순환 대화형 제어 표면"
                   "wiki"
                   "자기 문서화 능력"
                   "버릴 수 있는 개발 전용 표면이 아닙니다"
                   "QUARANTINE"
                   "Hangul codec"
                   "MSV / meaning sentence variants"
                   "graph-gate / gate-graph"
                   "multi-language emit registry"
                   "puck-cli bridge"
                   "tick runner"
                   "redb ingest brain"])))

  (testing "generated lane registry exposes current core surfaces"
    (is (has-all? (doc "docs/LANE_REGISTRY.md")
                  ;; the pnix-meta loader row was retired from the live ns
                  ;; inventory: :core 44 -> 43
              ["`:core`: 43"
                   "`:experimental`: 7"
                   "`:proof-only`: 28"
                   "pnix-clj.interop"
                   "meta-circular-runtime-interop-boundary"
                   "pnix-clj.nrepl"
                   "meta-circular-interactive-control-surface"
                   "pnix-clj.wiki"
                   "self-documenting-capability-and-roadmap-substrate"
                   "pnix-clj.lane-registry"]))))
