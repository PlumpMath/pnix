;;; plain Clojure의 한계 — 한 프로그램을 여러 substrate에서 돌려 '수렴'을 증명할 수 없다.
;;;
;;; plain Clojure에는:
;;;   1) 한 소스를 독립적인 여러 실행경로(해석/컴파일/미러)로 돌릴 lane이 없다,
;;;   2) 그 경로들이 '같은 값'인지 대조하는 교차검증 개념이 없다,
;;;   3) 자기호스팅 언어가 자기 자신에 수렴한다는 증거를 남길 방법이 없다.
;;;
;;; 실행:  cd pnix-clj && clojure -M examples/11-self-hosting-convergence/limit_clojure.clj

(ns limit-clojure)

;; eval 한 번이 전부다. 결과는 나오지만 "이 값이 여러 경로에서 같은가?"는 알 수 없다.
(def src '(let [x 40] (+ x 2)))
(println "eval 결과:" (eval src))    ; => 42

;; 컴파일 결과·자기-런타임·미러가 같은 값인지 대조할 lane 자체가 없다.
(println "이 값이 (a)직접평가 (b)bytecode (c)자기런타임 (d)미러 에서 모두 같은지")
(println "plain Clojure로는 대조할 수단이 없다 — eval 하나뿐이다.")

;; N-version 교차검증이 없으니, 한 경로가 조용히 틀려도(silent-wrong) 잡을 수 없다.
(println "\n결론: plain Clojure는 '자기호스팅 수렴/다중경로 의미일치'를 증명하지 못한다.")
