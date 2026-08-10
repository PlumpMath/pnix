;;; pnix-clj의 방식 - run-source가 direct evaluator, clj-meta bytecode,
;;; px-runtime, pnix-mirror receipt를 묶고 cross-lane verdict를 남긴다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/02-four-lane-receipt/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]))

(def source
  "let x = 40; in x + 2")

(let [row (pnix/run-source {:source-id :example/four-lane-receipt
                            :source source
                            :oracle-result {:status :ok
                                            :authority :example-inline
                                            :value 42}})
      values [(get-in row [:eval-result :value])
              (get-in row [:clj-meta-result :value])
              (get-in row [:px-runtime :value])
              (get-in row [:pnix-mirror :value])]
      cross (:cross-mirror-verdict row)]
  (println "source:" source)
  (println "receipt status:" (:status row) "| reason:" (:reason row))
  (println "lane values:" values)
  (println "cross mirror:" (select-keys cross [:status :equivalence :reason]))
  (println "bytecode hash:" (:bytecode-hash row))
  (println "receipt count:" (count (:receipts row)))

  (assert (= :accepted (:status row)))
  (assert (= [42 42 42 42] values))
  (assert (= :ok (:status cross)))
  (assert (= :agree (:equivalence cross)))
  (assert (string? (:bytecode-hash row)))
  (assert (<= 6 (count (:receipts row)))))

(println)
(println "결론: pnix-clj는 값과 함께 lane별 receipt, bytecode hash, cross-lane agreement verdict를 남긴다.")
