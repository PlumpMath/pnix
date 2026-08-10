;;; pnix-clj의 방식 - analyzer AST로 pure core와 host interop 표면을 구분한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/38-form-analysis-host-surface/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.form-analysis :as fa]))

(let [pure (fa/analyze-form '(+ (* 2 3) 4))
      host (fa/analyze-form '(System/getProperty "user.home"))
      report (fa/report)]
  (println "pure status/core?:" (:status pure) (:pure-core? pure))
  (println "host status/core?:" (:status host) (:pure-core? host))
  (println "host interop:" (:host-interop host))
  (println "report accepted/rejected:" (:accepted report) (:rejected report))

  (assert (= :ok (:status pure)))
  (assert (= true (:pure-core? pure)))
  (assert (= :ok (:status host)))
  (assert (= false (:pure-core? host)))
  (assert (seq (:host-interop host)))
  (assert (= :ok (:status report))))

(println)
(println "결론: pnix-clj는 form 실행 전 host surface를 구조화해 gate 입력으로 쓸 수 있다.")
(shutdown-agents)
