;;; pnix-clj의 방식 — foreign host object를 opaque-host-ref로 격리하고,
;;; release 이후 deref를 :held verdict로 만든다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/31-compartment-isolation/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.interop :as interop]))

(def host-object
  (atom {:secret 42 :public 7}))

(def pure-data
  {:public 7
   :items [1 2 3]})

(def nested-host
  {:public 7
   :host host-object})

(defn describe-value
  [label v]
  {:label label
   :value v
   :host-object? (interop/host-object? v)
   :opaque-ref? (interop/opaque-host-ref? v)
   :class (some-> v class .getName)})

(let [pure-marshaled (interop/from-host pure-data)
      opaque-ref (interop/from-host host-object)
      nested-marshaled (interop/from-host nested-host)
      deref-before (interop/opaque-ref-deref opaque-ref)
      _ (interop/release-opaque-ref! opaque-ref)
      deref-after (interop/opaque-ref-deref opaque-ref)]

  (println "pure data:")
  (println (describe-value :pure-data pure-data))
  (println "marshaled pure data:" pure-marshaled)
  (println)

  (println "host object:")
  (println (describe-value :host-object host-object))
  (println "opaque ref:" opaque-ref)
  (println "opaque ref?:" (interop/opaque-host-ref? opaque-ref))
  (println)

  (println "nested host object:")
  (println "raw host-object?:" (interop/host-object? nested-host))
  (println "marshaled nested:" nested-marshaled)
  (println "nested host slot opaque?:" (interop/opaque-host-ref? (:host nested-marshaled)))
  (println)

  (println "deref before release:")
  (println {:status (:status deref-before)
            :same-object? (identical? host-object (:value deref-before))
            :value @(:value deref-before)})
  (println)

  (println "deref after release:")
  (println deref-after)

  ;; pure values remain pure values
  (assert (= pure-data pure-marshaled))
  (assert (= false (interop/host-object? pure-data)))

  ;; foreign host object is isolated as opaque ref
  (assert (= true (interop/host-object? host-object)))
  (assert (= true (interop/opaque-host-ref? opaque-ref)))
  (assert (= :opaque-host-ref (:kind opaque-ref)))
  (assert (integer? (:id opaque-ref)))
  (assert (= "clojure.lang.Atom" (:class opaque-ref)))

  ;; nested host object is wrapped only at the host slot
  (assert (= true (interop/host-object? nested-host)))
  (assert (= 7 (:public nested-marshaled)))
  (assert (= true (interop/opaque-host-ref? (:host nested-marshaled))))

  ;; before release, deref recovers the object
  (assert (= :ok (:status deref-before)))
  (assert (identical? host-object (:value deref-before)))
  (assert (= {:secret 42 :public 7} @(:value deref-before)))

  ;; after release, deref does not throw or leak object; it becomes held
  (assert (= :held (:status deref-after)))
  (assert (= :opaque-ref-released (:reason deref-after)))
  (assert (= opaque-ref (:ref deref-after))))

(println)
(println "결론: pnix-clj는 host object를 canonical value로 새지 않게 opaque ref로 격리하고 release 이후 접근을 held verdict로 만든다.")
