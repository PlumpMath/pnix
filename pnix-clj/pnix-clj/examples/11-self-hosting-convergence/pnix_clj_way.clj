;;; pnix-clj의 방식 — run-tower = 4 substrate 수렴(자기호스팅 타워).
;;;
;;; 한 소스를 read -> emit-roundtrip -> 직접평가 -> specialize-residual -> lowering ->
;;; clj-meta(bytecode) -> px-runtime -> pnix-mirror 로 등반시키고, 모든 층이 한 값에
;;; collapse 하는지 판정한다. frontier 소스는 어느 층이 막았는지 정직하게 held.
;;;
;;; 실행:  cd pnix-clj && clojure -M examples/11-self-hosting-convergence/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.tower :as tower]))

;; 1) 완전 지원 소스는 4 substrate에서 한 값으로 collapse.
(let [t (tower/run-tower "let x = 40; in x + 2")]
  (println "collapse:" (get-in t [:collapse :status])
           "| value:" (get-in t [:collapse :value]))
  (println "  동의한 층:" (mapv name (get-in t [:collapse :agreeing-layers])))
  (assert (and (= :collapsed (get-in t [:collapse :status]))
               (= 42 (get-in t [:collapse :value])))))

;; 2) 더 무거운 소스(패턴 람다 + functionArgs)도 collapse — 최근 slice로 lift됨.
(let [t (tower/run-tower "let f = { a ? 1 }: a; in builtins.functionArgs f")]
  (println "패턴/functionArgs collapse:" (get-in t [:collapse :status])
           "| value:" (get-in t [:collapse :value]))
  (assert (= :collapsed (get-in t [:collapse :status]))))

;; 3) import + 모듈맵도 4-lane collapse (타워가 모듈을 climb 전체에 스레딩).
(let [t (tower/run-tower {:source "(import ./five.px) + 10"
                          :import-modules {"./five.px" "5"}})]
  (println "import collapse:" (get-in t [:collapse :status])
           "| value:" (get-in t [:collapse :value]))
  (assert (and (= :collapsed (get-in t [:collapse :status]))
               (= 15 (get-in t [:collapse :value])))))

;; 4) purity-gated host effect는 정직하게 held — 어느 층이 막았는지 명시(silent-wrong 금지).
(let [t (tower/run-tower "builtins.getEnv \"HOME\"")]
  (println "frontier held:" (get-in t [:collapse :status])
           "| blocking layer:" (get-in t [:collapse :blocking :layer]))
  (assert (= :held (get-in t [:collapse :status]))))

(println "\n결론: 한 소스가 네 substrate에서 같은 값에 수렴 — 자기호스팅 메타서큘러 증거.")
