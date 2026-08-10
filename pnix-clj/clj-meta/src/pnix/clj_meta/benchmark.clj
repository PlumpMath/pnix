(ns pnix.clj-meta.benchmark
  "성능 정책(todo §13/§14): 같은 동작을 우리 직접 emit vs host 로 컴파일해 측정하고
  빠른 쪽을 채택한다. ratio = ours/host (≈1.0 이면 host 급, <1 이면 우리가 더 빠름,
  >1 이면 host 가 빠름 → host 유지가 옳음).

  ours-class 가 pnix.clj_meta.gen.Fn__* 이면 우리 직접 emit, 그 외(host gensym)면
  host fallback 으로 컴파일된 것."
  (:require [pnix.clj-meta.compiler :as c]))

(defn- ms
  [thunk n]
  (dotimes [_ 3000] (thunk))                 ; warmup (JIT)
  (let [t (System/nanoTime)]
    (dotimes [_ n] (thunk))
    (/ (- (System/nanoTime) t) 1e6)))

(def ^:private cases
  [['(fn [n] (* n n))                                    [9]          2000000]
   ['(fn [^long n] (let [a (+ n 1)
                         b (* a 2)]
                     (+ b 3)))                            [19]         2000000]
   ['(fn fact [n] (if (< n 2) 1 (* n (fact (- n 1)))))   [12]         1000000]
   ['(fn [n] (loop [i n acc 0] (if (< i 1) acc (recur (- i 1) (+ acc i))))) [100] 500000]
   ['(fn [^long n] (loop [i n acc 0] (if (< i 1) acc (recur (- i 1) (+ acc i))))) [100] 500000]
   ['(fn ([] 0) ([x] x) ([x y] (+ x y)))                 [20 22]      2000000]
   ['(fn [a & r] (count r))                              [1 2 3 4 5]  1000000]
   ['(fn ([x] x) ([x & r] (count r)))                    [1 2 3 4 5]  1000000]
   ['(fn [x] (map (fn [y] (+ x y)) [1 2 3]))             [10]         1000000]])

(defn run
  []
  (mapv (fn [[form args n]]
          (let [ours (c/compile-form form)
                host (eval form)
                o    (ms #(apply ours args) n)
                h    (ms #(apply host args) n)]
            {:form form
             :ours-class (.getSimpleName (class ours))
             :ours-ms (Math/round ^double o)
             :host-ms (Math/round ^double h)
             :ratio (/ o h)}))
        cases))

(defn -main
  [& _]
  (println "성능: 우리 직접 emit vs host (ratio=ours/host, ≈1=host급, >1=host가 빠름)")
  (doseq [r (run)]
    (println (format "  %-50s ours=%4dms host=%4dms ratio=%.2f  [%s]"
                     (pr-str (:form r)) (:ours-ms r) (:host-ms r) (double (:ratio r))
                     (:ours-class r))))
  (shutdown-agents))
