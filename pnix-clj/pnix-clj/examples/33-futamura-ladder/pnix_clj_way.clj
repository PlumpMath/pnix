;;; pnix-clj의 방식 — futamura/report를 이용해
;;; direct interpreter, 1st projection, 2nd projection, 3rd projection route status를 전시한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/33-futamura-ladder/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.futamura :as fut]))

(defn short-hash
  [x]
  (subs (str x) 0 12))

(let [r (fut/report)
      rows (:rows r)
      first-row (first rows)
      jones (:jones-optimality r)
      third (:third-projection r)]

  (println "futamura report status:" (:status r))
  (println "total:" (:total r)
           "accepted:" (:accepted r)
           "rejected:" (:rejected r))
  (println)

  (println "2nd projection generating extension:")
  (println " route:" (:generating-extension-route r))
  (println " compiler-id:" (short-hash (:generating-extension-compiler-id r)))
  (println " compiler fixed across programs?:"
           (:compiler-fixed-across-programs? r))
  (println)

  (println "1st projection:")
  (println " specialization varies per program?:"
           (:first-projection-specialization-varies? r))
  (println " distinct residuals:"
           (:distinct-first-projection-residuals r))
  (println)

  (println "sample row:")
  (println " id:" (:id first-row))
  (println " source:" (:source first-row))
  (println " direct-value:" (:direct-value first-row))
  (println " first-projection-value:" (:first-projection-value first-row))
  (println " second-projection-value:" (:second-projection-value first-row))
  (println " residual-hash:"
           (short-hash (:first-projection-residual-hash first-row)))
  (println " compiler-id:"
           (short-hash (:generating-extension-compiler-id first-row)))
  (println " bytecode determinism:"
           (:second-projection-bytecode-determinism first-row))
  (println " jones row witness:"
           (:jones-witness first-row))
  (println)

  (println "Jones optimality witness:")
  (println " kind:" (:kind jones))
  (println " verdict:" (:verdict jones))
  (println " bounded?:" (:bounded? jones))
  (println " ratio-min:" (:ratio-min jones))
  (println " ratio-max:" (:ratio-max jones))
  (println " note:" (:note jones))
  (println)

  (println "3rd projection:")
  (println " projection:" (:projection third))
  (println " product:" (:product third))
  (println " status:" (:status third))
  (println " reason:" (:reason third))
  (println " proof-anchor:" (:proof-anchor third))

  ;; report-level checks
  (assert (= :ok (:status r)))
  (assert (pos? (:total r)))
  (assert (= (:total r) (:accepted r)))
  (assert (= 0 (:rejected r)))
  (assert (= :cogen-free (:generating-extension-route r)))
  (assert (= true (:compiler-fixed-across-programs? r)))
  (assert (= true (:first-projection-specialization-varies? r)))
  (assert (= 1 (:distinct-compiler-ids r)))
  (assert (> (:distinct-first-projection-residuals r) 1))

  ;; row-level equation: interp == 1st == 2nd
  (doseq [row rows]
    (assert (= :accepted (:status row)))
    (assert (= (:direct-value row)
               (:first-projection-value row)
               (:second-projection-value row)))
    (assert (string? (:first-projection-residual-hash row)))
    (assert (= (:generating-extension-compiler-id r)
               (:generating-extension-compiler-id row)))
    (assert (= :ok (:second-projection-bytecode-determinism row)))
    (assert (= false (get-in row [:jones-witness :interpreter-dispatch?]))))

  ;; Jones witness is measured/structural, not a mechanized theorem.
  (assert (= :jones-optimality-witness (:kind jones)))
  (assert (= true (:bounded? jones)))
  (assert (= :jones-optimal-no-interpreter-floor (:verdict jones)))
  (assert (= :structural-measurement-not-mechanized-proof (:note jones)))

  ;; 3rd projection route is now present as a cogen-free curried construction,
  ;; still not a self-application cogen claim.
  (assert (= :third (:projection third)))
  (assert (= :compiler-generator (:product third)))
  (assert (= :built-curried-route (:status third)))
  (assert (= :cogen-free-currying-not-self-application (:reason third)))
  (assert (= :genuine-proof-not-heuristic
             (get-in third [:proof-anchor :kind]))))

(println)
(println "결론: pnix-clj는 Futamura 1차/2차와 cogen-free 3차 route를 report로 검증하고, self-application cogen과는 구분한다.")
