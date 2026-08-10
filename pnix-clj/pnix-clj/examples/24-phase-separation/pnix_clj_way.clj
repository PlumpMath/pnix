;;; pnix-clj의 방식 — parse/purity/direct-eval/lowering/compiled-path/capability-gate를
;;; phase별 verdict로 분리한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/24-phase-separation/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.parser :as parser]
            [pnix-clj.safe-eval :as safe]
            [pnix-clj.lowering :as lowering]
            [pnix-clj.clj-meta :as clj-meta]))

(def pure-source
  "let x = 40; in x + 2")

(def impure-source
  "builtins.getEnv \"HOME\"")

(defn pure-phase-report
  [source]
  (let [parsed (parser/parse-source source)
        purity (safe/static-purity-check source)
        direct (pnix/eval-source source)
        lowered (when (= :ok (:status parsed))
                  (lowering/lower-ast (:ast parsed)))
        compiled (when (= :ok (:status lowered))
                   (clj-meta/eval-lowered (:form lowered)))]
    {:source source
     :parse {:status (:status parsed)}
     :purity {:status (:status purity)
              :pure? (:pure? purity)
              :required-capabilities (mapv :builtin (:impure-uses purity))}
     :direct-eval {:status (:status direct)
                   :value (:value direct)}
     :lowering {:status (:status lowered)}
     :compiled-path {:status (:status compiled)
                     :value (:value compiled)
                     :mode (:mode compiled)
                     :determinism (get-in compiled [:compile-receipt :determinism :status])
                     :api-values-agree? (:api-values-agree? compiled)}}))

(defn impure-phase-report
  [source]
  (let [parsed (parser/parse-source source)
        purity (safe/static-purity-check source)
        gated (safe/safe-eval source {:pure-only? true})]
    {:source source
     :parse {:status (:status parsed)}
     :purity {:status (:status purity)
              :pure? (:pure? purity)
              :required-capabilities (mapv :builtin (:impure-uses purity))}
     :capability-gate {:status (:status gated)
                       :limit-exceeded (:limit-exceeded gated)
                       :reason (:reason gated)}
     :direct-eval :not-run-after-capability-deny
     :lowering :not-run-after-capability-deny
     :compiled-path :not-run-after-capability-deny}))

(let [pure-report (pure-phase-report pure-source)
      impure-report (impure-phase-report impure-source)]

  (println "pure source:" pure-source)
  (println "parse:" (get-in pure-report [:parse :status]))
  (println "purity:"
           "status=" (get-in pure-report [:purity :status])
           "pure?=" (get-in pure-report [:purity :pure?])
           "required=" (get-in pure-report [:purity :required-capabilities]))
  (println "direct-eval:"
           (get-in pure-report [:direct-eval :status])
           (get-in pure-report [:direct-eval :value]))
  (println "lowering:" (get-in pure-report [:lowering :status]))
  (println "compiled-path:"
           (get-in pure-report [:compiled-path :status])
           (get-in pure-report [:compiled-path :value]))
  (println "compile determinism:"
           (get-in pure-report [:compiled-path :determinism]))
  (println "api values agree?:"
           (get-in pure-report [:compiled-path :api-values-agree?]))

  (println)
  (println "impure source:" impure-source)
  (println "parse:" (get-in impure-report [:parse :status]))
  (println "purity:"
           "status=" (get-in impure-report [:purity :status])
           "pure?=" (get-in impure-report [:purity :pure?])
           "required=" (get-in impure-report [:purity :required-capabilities]))
  (println "capability-gate:"
           "status=" (get-in impure-report [:capability-gate :status])
           "limit=" (get-in impure-report [:capability-gate :limit-exceeded])
           "reason=" (get-in impure-report [:capability-gate :reason]))
  (println "direct-eval:" (:direct-eval impure-report))
  (println "lowering:" (:lowering impure-report))
  (println "compiled-path:" (:compiled-path impure-report))

  (assert (= :ok (get-in pure-report [:parse :status])))
  (assert (= true (get-in pure-report [:purity :pure?])))
  (assert (= :ok (get-in pure-report [:direct-eval :status])))
  (assert (= 42 (get-in pure-report [:direct-eval :value])))
  (assert (= :ok (get-in pure-report [:lowering :status])))
  (assert (= :ok (get-in pure-report [:compiled-path :status])))
  (assert (= 42 (get-in pure-report [:compiled-path :value])))
  (assert (= :ok (get-in pure-report [:compiled-path :determinism])))
  (assert (= true (get-in pure-report [:compiled-path :api-values-agree?])))

  (assert (= :ok (get-in impure-report [:parse :status])))
  (assert (= false (get-in impure-report [:purity :pure?])))
  (assert (= ["getEnv"] (get-in impure-report [:purity :required-capabilities])))
  (assert (= :held (get-in impure-report [:capability-gate :status])))
  (assert (= :impure (get-in impure-report [:capability-gate :limit-exceeded])))
  (assert (= :static-impure-use (get-in impure-report [:capability-gate :reason])))
  (assert (= :not-run-after-capability-deny (:direct-eval impure-report)))
  (assert (= :not-run-after-capability-deny (:lowering impure-report)))
  (assert (= :not-run-after-capability-deny (:compiled-path impure-report))))

(println)
(println "결론: pnix-clj는 실행을 한 덩어리로 섞지 않고 phase별 verdict로 분리한다.")
