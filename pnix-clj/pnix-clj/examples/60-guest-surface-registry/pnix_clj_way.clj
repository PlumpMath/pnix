;;; pnix-clj의 방식 - guest builtin surface diff를 resource registry로 고정한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/60-guest-surface-registry/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]))

(let [surface (edn/read-string (slurp (io/resource "pnix_clj/guest_surface.edn")))]
  (println "surface:" (select-keys surface [:kind :schema :lineage]))
  (println "extension count:" (count (:extensions surface)))
  (println "missing-vs-nix count:" (count (:missing-vs-nix-2-34 surface)))
  (println "captured real-nix count:" (count (:captured-real-nix surface)))

  (assert (= :guest-surface-registry (:kind surface)))
  (assert (some #{"koreanFinalConsonantKind"} (:extensions surface)))
  (assert (some #{"fetchGit"} (:missing-vs-nix-2-34 surface)))
  (assert (some #{"attrNames"} (:captured-real-nix surface))))

(println)
(println "결론: pnix-clj는 guest extension surface를 real-Nix captured surface와 함께 고정한다.")
(shutdown-agents)

