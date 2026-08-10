;;; plain Clojure의 한계 - atom/vector 로그는 append-only나 tamper evidence를 강제하지 않는다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/14-append-only-event-log/limit_clojure.clj

(ns append-only-event-log-limit)

(def log (atom []))

(swap! log conj {:kind :eval/run :payload {:result 1}})
(swap! log assoc-in [0 :payload :result] 999)

(println "mutated log:" @log)
(println "missing: hash chain, contaminated payload rejection, pointer move event")

(assert (= 999 (get-in @log [0 :payload :result])))

(println)
(println "결론: 평범한 in-memory 로그는 과거 event 수정과 host object 오염을 막지 못한다.")
