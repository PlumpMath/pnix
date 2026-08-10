;;; plain Clojure의 한계 - machine report처럼 생긴 파일을 손으로 쓸 수는 있지만
;;; report-artifact registry, gate alias, shared corpus 실행 여부는 보장하지 못한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/91-machine-report-artifact-gate/limit_clojure.clj

(ns machine-report-artifact-gate-limit)

(def hand-report
  {:kind :machine-report
   :status :ok
   :row-count 155
   :divergent []
   :constant-stack-witness {:ok? true}
   :derivation {:honest-labels []}})

(def file
  (java.io.File/createTempFile "plain-machine-report-" ".edn"))

(spit file (pr-str hand-report))

(def loaded
  (read-string (slurp file)))

(println "plain file:" (.getPath file))
(println "loaded:" (select-keys loaded [:kind :status :row-count]))
(println "registered report kind?:" false)
(println "gate alias wired?:" false)
(println "shared corpus really executed?:" false)
(println "honest labels:" (get-in loaded [:derivation :honest-labels]))

(assert (= :ok (:status loaded)))
(assert (= :machine-report (:kind loaded)))

(.delete file)

(println)
(println "결론: plain Clojure는 machine report 모양의 파일을 쓸 수 있지만, 실제 gate capability라는 증거는 없다.")
