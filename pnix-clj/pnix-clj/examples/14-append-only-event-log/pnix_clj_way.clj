;;; pnix-clj의 방식 - store/append!는 pure EDN event만 받고 hash chain으로
;;; tamper-evident append-only log를 만든다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/14-append-only-event-log/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.store :as store]))

(let [log (store/open-store)
      a (store/append! log :eval/run {:source-hash "s1" :result-hash "r1"})
      b (store/append! log :eval/run {:source-hash "s2" :result-hash "r2"})
      p (store/set-pointer! log :active "s2")
      bad (store/append! log :eval/run {:when (java.util.Date.)})
      chain (store/verify-chain log)]
  (println "append results:" a b p)
  (println "bad payload:" bad)
  (println "active pointer:" (store/get-pointer log :active))
  (println "chain:" chain)
  (println "events:" (mapv #(select-keys % [:seq :kind :prev-hash :event-hash])
                            (store/events log)))

  (assert (= :ok (:status a)))
  (assert (= :ok (:status b)))
  (assert (= :ok (:status p)))
  (assert (= :rejected (:status bad)))
  (assert (= :contaminated-payload (:reason bad)))
  (assert (= "s2" (store/get-pointer log :active)))
  (assert (= :intact (:status chain)))
  (assert (= 3 (count (store/events log)))))

(println)
(println "결론: pnix-clj store는 event를 append-only hash chain과 hermetic payload gate로 다룬다.")
