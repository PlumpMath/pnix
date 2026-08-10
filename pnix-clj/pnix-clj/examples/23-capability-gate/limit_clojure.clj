;;; plain Clojure의 한계 — host capability 호출은 가능하지만,
;;; pnix식 capability verdict/receipt가 자동으로 남지 않는다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/23-capability-gate/limit_clojure.clj

(ns capability-gate-limit)

(defn host-env-read []
  ;; 이 호출은 host 환경에 접근한다.
  ;; plain Clojure는 이를 막지 않으며, capability 요구/허용/거부 receipt도 만들지 않는다.
  (System/getenv "HOME"))

(let [v (host-env-read)]
  (println "plain host env read value exists?:" (boolean v))
  (println "capability verdict:" nil)
  (println "capability receipt:" nil)
  (assert (or (nil? v) (string? v))))

(println)
(println "결론: plain Clojure host interop은 실행값은 줄 수 있지만 capability gate verdict를 기본으로 주지 않는다.")
