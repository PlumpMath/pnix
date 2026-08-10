;;; plain Clojure의 한계 - builtin map 확장은 guest surface registry가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/60-guest-surface-registry/limit_clojure.clj

(ns guest-surface-registry-limit)

(def builtins
  {"length" count
   "myExtra" identity})

(println "manual builtins:" (keys builtins))
(println "captured real Nix diff?:" false)
(println "extension vs host leak classified?:" false)

(assert (contains? builtins "myExtra"))

(println)
(println "결론: 손으로 builtin을 추가하는 것은 real-Nix surface diff와 extension registry를 남기지 않는다.")

