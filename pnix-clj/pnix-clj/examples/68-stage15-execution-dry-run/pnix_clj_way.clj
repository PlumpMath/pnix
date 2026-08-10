;;; pnix-clj의 방식 - stage15 execute-plan을 fake runner로 dry-run한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/68-stage15-execution-dry-run/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.stage15 :as stage15]))

(defn fake-runner
  [{:keys [id command purpose]} _timeout-ms]
  {:id id
   :command command
   :purpose purpose
   :status :ok
   :reason :fake-command-ok
   :exit 0
   :duration-ms 0
   :stdout-hash "stdout"
   :stderr-hash "stderr"})

(let [report (stage15/execute-plan {:command-ids [:compiler-smoke]
                                    :timeout-ms 1
                                    :runner fake-runner})
      row (first (:rows report))]
  (println "report:" (select-keys report [:kind :status :reason :selected-command-ids
                                          :selected-command-count :held-count :receipt-hash]))
  (println "row:" row)

  (assert (= :stage15-execution-report (:kind report)))
  (assert (= :ok (:status report)))
  (assert (= [:compiler-smoke] (:selected-command-ids report)))
  (assert (= 1 (:selected-command-count report)))
  (assert (zero? (:held-count report)))
  (assert (= :fake-command-ok (:reason row)))
  (assert (string? (:receipt-hash report))))

(println)
(println "결론: pnix-clj stage15 execution report는 runner를 주입해 외부 실행 없이도 shape를 검증할 수 있다.")
(shutdown-agents)

