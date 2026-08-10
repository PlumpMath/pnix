[
 [(fn [n] (loop [i n acc 0] (if (< i 1) acc (recur (- i 1) (+ acc i))))) [10]]
 [(fn [x] (mapv (fn [y] (+ x y)) [1 2 3])) [40]]
 [(fn [k] {:square (* k k) :pair [k (+ k 1)]}) [9]]
 [(fn ([] :zero) ([x] [:one x]) ([x y] [:two (+ x y)])) [20 22]]
 [(fn ([x] [:fixed x]) ([x & r] [:rest x r])) [7 8 9]]
 [(fn [] (let [a (java.util.concurrent.atomic.AtomicInteger. 0)]
          [(try (throw (ex-info "boom" {}))
             (catch Exception e :caught)
             (finally (.incrementAndGet a)))
           (.get a)])) []]
 [(fn [] (let [p (java.awt.Point.)]
          (set! (. p x) 12)
          (set! (. p y) 30)
          [(.-x p) (.-y p)])) []]
]
