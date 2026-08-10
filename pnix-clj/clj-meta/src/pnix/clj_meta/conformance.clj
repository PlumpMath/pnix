(ns pnix.clj-meta.conformance
  "등가성 게이트: 같은 form 을 세 백엔드로 평가해 결과가 일치하는지 검증한다.

  - host      : (eval form)            — 기준(host clojure.lang.Compiler)
  - compiler  : 우리 자작 bytecode 컴파일러 (host Compiler 0회)
  - kernel    : 메타순환 인터프리터 (적용 가능한 경우의 참고 축)

  게이트 기준은 **host ≡ compiler** (필수). kernel 은 reference semantics 보조축으로
  현재 conformance corpus 전체에서 함께 비교한다.

  목적: 앞으로 컴파일러 op 를 확장할 때마다 'host 와 의미가 같다'를 자동 확인해
  회귀를 막는 안전망."
  (:require [pnix.clj-meta.compiler :as c]
            [pnix.clj-meta.kernel :as k]
            [clojure.java.io :as io]))

(def ^:private inline-cases
  "[form args]. form 은 (fn …). 세 백엔드로 같은 결과여야 한다."
  [['(fn [] 42)                                           []]
   ['(fn [n] n)                                           [7]]
   ['(fn [a b] a)                                         [11 22]]
   ['(fn [a b] b)                                         [11 22]]
   ['(fn [n] (* n n))                                     [9]]
   ['(fn [a b] (+ a b))                                   [20 22]]
   ['(fn [a b] (+ a b))                                   [1.5 2.25]]
   ['(fn [] (+ 1.5 2.25))                                 []]
   ['(fn [n] (+ n 1))                                     [41]]
   ['(fn [n] (* (+ n 1) 2))                               [20]]
   ['(fn [n] (if (< n 5) 1 0))                            [3]]
   ['(fn [n] (if (< n 5) 1 0))                            [9]]
   ['(fn [n] (let [a (* n 2)] (+ a 1)))                   [10]]
   ['(fn [n] (let [a (+ n 1) b (* a 2)] b))               [20]]
   ['(fn [^long n] (let [a (+ n 1)
                         b (* a 2)]
                     (+ b 3)))                             [19]]
   ['(fn [^double x] (let [y (+ x 0.5)]
                       (* y 2.0)))                         [20.5]]
   ['(fn [^long a ^long b] (unchecked-add a b))             [Long/MAX_VALUE 1]]
   ['(fn [^long a ^long b] (unchecked-subtract a b))        [Long/MIN_VALUE 1]]
   ['(fn [^long a ^long b] (unchecked-multiply a b))        [Long/MAX_VALUE 2]]
   ['(fn [] (+ 20 22))                                      []]
   ['(fn [] (- 50 8))                                       []]
   ['(fn [] (* 6 7))                                        []]
   ['(fn [] (let [a 20 b 22] (+ a b)))                      []]
   ['(fn [] (let [a 20 b (+ a 1)] (+ b 21)))                []]
   ['(fn [] (let [a 6 b 7] (* a b)))                        []]
   ['(fn [] (loop [a 20 b 22] (+ a b)))                     []]
   ['(fn [] (loop [a 20 b 22] (if true (+ a b) (recur a b)))) []]
   ['(fn [] (loop [a 6 b 7] (* a b)))                       []]
   ['(fn [] (loop [i 0]
             (if (< i 10)
               (recur (+ i 1))
               [(+ i 20) (- 30 i) (* i 2)])))               []]
   ['(fn [] (loop [i 0]
             (if (< i 10)
               (recur (+ i 3))
               [(+ i 20) (- 40 i) (* i 2)])))               []]
   ['(fn [] (loop [i 10]
             (if (> i 0)
               (recur (- i 3))
               [(+ i 20) (- 40 i) (* i 2)])))               []]
   ['(fn [] (loop [i 0 acc 0]
             (if (< i 10)
               (recur (+ i 1) (+ acc 2))
               [(+ acc 20) (- 50 acc) (* acc 2)])))          []]
   ['(fn [^long x]
       (if (> x -10)
         (if (< x 10)
           [(+ x 1) (- 10 x) (* x 2)]
           0)
         0))                                                  [7]]
   ['(fn [] (loop [i 0 acc 0]
             (if (< i 10)
               (recur (+ i 1) (+ acc i))
               [(+ acc 1) (- 50 acc) (* acc 2)])))             []]
   ['(fn [] (loop [i 0 acc 1]
             (if (< i 5)
               (recur (+ i 1) (* acc 2))
               [(+ acc 1) (- 100 acc) (* acc 2)])))             []]
   ['(fn fact [n] (if (< n 2) 1 (* n (fact (- n 1)))))    [5]]
   ['(fn [] [1 2 3])                                      []]
   ['(fn [k] [k (* k 2)])                                 [21]]
   ['(fn [] :hello)                                       []]
   ['(fn [] \.)                                           []]
   ['(fn [] '(1 2 sym))                                   []]
   ['(fn [k] {:a k :b 2})                                 [9]]
   ['(fn [m] (:a m))                                      [{:a 42}]]
   ['(fn [] #{1 2 3})                                     []]
   ['(fn [] {:x [1 2] :y :z})                             []]
   ['(fn [] 'sym)                                         []]
   ['(fn [n] (loop [i n acc 1] (if (< i 2) acc (recur (- i 1) (* acc i))))) [5]]
   ['(fn [^long n]
       (loop [i n acc 0]
         (if (< i 1)
           acc
           (recur (- i 1) (+ acc i)))))                   [10]]
   ['(fn [^double x]
       (loop [i 0 acc x]
         (if (< i 3)
           (recur (+ i 1) (+ acc 0.5))
           acc)))                                         [40.5]]
   ['(fn [n acc] (if (< n 1) acc (recur (- n 1) (+ acc n))))               [5 0]]
   ['(fn [x] (try (quot 10 x) (catch ArithmeticException e :divzero)))     [2]]
   ['(fn [x] (try (quot 10 x) (catch ArithmeticException e :divzero)))     [0]]
   ['(fn [] (try (throw (ex-info "b" {})) (catch Exception e :caught)))    []]
   ['(fn [] (let [a (java.util.concurrent.atomic.AtomicInteger. 0)]
              (try 10 (finally (.incrementAndGet a)))
              (.get a)))                                                   []]
   ['(fn [] (let [a (java.util.concurrent.atomic.AtomicInteger. 0)]
              [(try (quot 10 0)
                 (catch ArithmeticException e :divzero)
                 (finally (.incrementAndGet a)))
               (.get a)]))                                                 []]
   ['(fn [] (let [a (java.util.concurrent.atomic.AtomicInteger. 0)]
              (try
                (try (throw (ex-info "b" {}))
                  (finally (.incrementAndGet a)))
                (catch Exception e (.get a)))))                             []]
   ['(fn [^String s] (.length s))                                         ["hello"]]
   ['(fn [^String s] (.toUpperCase s))                                    ["hi"]]
   ['(fn [] (Math/sqrt 16.0))                                             []]
   ['(fn [] Long/MAX_VALUE)                                               []]
   ['(fn [] (.size (java.util.ArrayList.)))                               []]
   ['(fn [x] (.length x))                                                 ["hello"]]
   ['(fn [p] (.-x p))                                                     [(java.awt.Point. 7 9)]]
   ['(fn [p] (set! (.-x p) 8) (.-x p))                                    [(java.awt.Point.)]]
   ['(fn [] (let [seen (atom []) p (java.awt.Point.)]
              (set! (. (do (swap! seen conj :target) p) x)
                    (do (swap! seen conj :val) 8))
              @seen))                                                     []]
   ['(fn [xs] (clojure.core.protocols/coll-reduce xs +))                  [[1 2 3]]]
   ['(fn [xs] (clojure.core.protocols/coll-reduce xs + 10))               [[1 2 3]]]
   ['(fn [] (String/valueOf (.toCharArray "ab")))                         []]
   ['(fn [] (let [sb (StringBuilder.)]
              (.append sb (.toCharArray "ab"))
              (.toString sb)))                                             []]
   ['(fn [] (Integer/toString 42))                                         []]
   ['(fn [] (.size (java.util.ArrayList. 4)))                              []]
   ['(fn [x] (instance? String x))                                         ["x"]]
   ['(fn [x] (instance? String x))                                         [42]]
   ['(fn [] (let [p (java.awt.Point.)] (set! (. p x) 7) (. p x)))         []]
   ['(fn [] (binding [*print-length* 1]
              (set! *print-length* 7)
              *print-length*))                                             []]
   ['(fn [] (let [sb (StringBuilder.)]
              (locking sb (.append sb "x"))
              (.toString sb)))                                             []]
   ['(fn [] (var +))                                                      []]
   ['(fn [x] ((fn [y] (+ x y)) 5))                                        [10]]
   ['(fn [n] (let [m (* n 2)] ((fn [] m))))                               [21]]
   ['(fn [a b] ((fn [] (+ a b))))                                         [20 22]]
   ['(fn [x] (map (fn [y] (+ x y)) [1 2 3]))                              [10]]
   ['(fn [] (do (def k-sq2 (fn [n] (* n n))) (k-sq2 9)))                  []]
   ['(fn [] (do (def k-v2 (+ 40 2)) k-v2))                               []]
   ['(fn [a & r] r)                                                       [1 2 3]]
   ['(fn [& r] (count r))                                                 [1 2 3]]
   ['(fn [a b & r] [a b r])                                               [1 2 3 4]]
   ['(fn ([x] [:fixed x]) ([x & r] [:rest x r]))                          [7]]
   ['(fn ([x] [:fixed x]) ([x & r] [:rest x r]))                          [7 8 9]]
   ['(fn [a] ((fn ([x] (+ a x)) ([x & r] [a x r])) 1 2 3))                [10]]
   ['(fn [n] (case n 1 :one 2 :two :other))                               [2]]
   ['(fn [] (case (long 9999999999) 1 :one 2 :two :other))                 []]
   ['(fn [k] (case k :a 1 :b 2 :c 3 :d 4 :e 5 :f 6 :g 7 :h 8 0))           [:g]]
   ['(fn [] (case "Aa" "Aa" 1 "BB" 2 :other))                              []]
   ['(fn [] (case "BB" "Aa" 1 "BB" 2 :other))                              []]
   ['(fn [] [(class 5N) 5N])                                                []]
   ['(fn [] [(class 10000000000000000000N) 10000000000000000000N])          []]
   ['(fn [] [(class 1.5M) 1.5M])                                            []]
   ['(fn [] [(class 1/3) 1/3])                                              []]
   ['(fn [] [(class #"a+") (boolean (re-find #"a+" "aa"))])                 []]
   ['(fn [] (do (def ^:dynamic *conf-dyn-smoke* 1)
                [(.isDynamic (var *conf-dyn-smoke*))
                 (binding [*conf-dyn-smoke* 2] *conf-dyn-smoke*)]))         []]
   ['(fn [] (do (deftype ConfVol [^:volatile-mutable v]
                  Object
                  (toString [_] ""))
                (java.lang.reflect.Modifier/isVolatile
                 (.getModifiers (.getDeclaredField (class (->ConfVol 1)) "v"))))) []]
   ['(fn [] (do (defrecord ConfRecMap [a b])
                [(:a (map->ConfRecMap {:a 5 :b 6}))
                 (:c (map->ConfRecMap {:a 1 :b 2 :c 9}))
                 (= (map->ConfRecMap {:a 5 :b 6}) (->ConfRecMap 5 6))]))    []]
   ['(fn [] (do (defrecord ConfRecSem [a b])
                (let [r (->ConfRecSem 1 2)
                      r2 (->ConfRecSem 1 2)
                      d (dissoc r :a)
                      t (.getLookupThunk r :a)]
                  [(.get t r)
                   (record? d)
                   (:b d)
                   (contains? #{r} r2)
                   (= (hash r) (hash r2))
                   (= (clojure.lang.Util/hasheq r)
                      (clojure.lang.Util/hasheq r2))])))                   []]
   ['(fn [] [(filterv (fn [x] (> x 5)) [1 2 3])
             (vec (remove (fn [x] (> x 1)) [1 2 3]))
             (every? (fn [x] (> x 10)) [1 2 3])
             (vec (take-while (fn [x] (< x 0)) [1 2 3]))])                []]
   ['(fn [] (do (def conf-canonical-false (< 5 1))
                (if conf-canonical-false :truthy :falsey)))                []]
   ['(fn [] [(identical? (< 1 5) true)
             (identical? (< 5 1) false)
             (identical? (instance? String 42) false)])                   []]
   ['(fn [] [(= 1 1.0)
             (= 1.0 1)
             (if (= 5 5.0) :eq :neq)])                                    []]
   ;; host fallback (우리 미구현 op → host eval 위임; Clojure 전체 기능 손실 0)
   ['(fn ([] 0) ([x] x) ([x y] (+ x y)))                                  [20 22]]
   ['(fn [n] (letfn [(ev? [x] (if (zero? x) true (od? (dec x))))
                     (od? [x] (if (zero? x) false (ev? (dec x))))] (ev? n))) [4]]])

(defn- example-cases
  []
  (let [dir (io/file "examples")]
    (if (.isDirectory dir)
      (->> (file-seq dir)
           (filter #(and (.isFile ^java.io.File %)
                         (.endsWith (.getName ^java.io.File %) ".clj")))
           (sort-by #(.getPath ^java.io.File %))
           (mapcat #(read-string (slurp %)))
           vec)
      [])))

(defn- all-cases
  []
  (into inline-cases (example-cases)))

(def ^:private negative-cases
  "[form args]. host 와 compiler 가 모두 실패해야 하는 케이스."
  [['(fn [] (throw (ex-info "boom" {}))) []]
   ['(fn [] (+ Long/MAX_VALUE 1)) []]
   ['(fn [] (+ 9223372036854775807 1)) []]
   ['(fn [] (- Long/MIN_VALUE 1)) []]
   ['(fn [] (* 9223372036854775807 2)) []]
   ['(fn [] (* Long/MIN_VALUE -1)) []]
   ['(fn [] (let [a 9223372036854775807] (+ a 1))) []]
   ['(fn [] (loop [a 9223372036854775807] (+ a 1))) []]
   ['(fn [] (loop [i 9223372036854775806]
             (if (< i 9223372036854775807)
               (recur (+ i 1))
               (+ i 1)))) []]
   ['(fn [] (loop [i 9223372036854775806]
             (if (< i 9223372036854775807)
               (recur (+ i 2))
               (+ i 1)))) []]
   ['(fn [] (loop [i -9223372036854775807]
             (if (> i -9223372036854775808)
               (recur (- i 2))
               (+ i -1)))) []]
   ['(fn [] (loop [i 0 acc 9223372036854775806]
             (if (< i 2)
               (recur (+ i 1) (+ acc 1))
               (+ acc 1)))) []]
   ['(fn [^long x]
       (if (< x 10)
         (- x 1)
         0)) [Long/MIN_VALUE]]
   ['(fn [] (loop [i 0 acc 9223372036854775800]
             (if (< i 10)
               (recur (+ i 1) (+ acc i))
               (+ acc 1)))) []]
   ['(fn [] (loop [i 0 acc 4611686018427387904]
             (if (< i 2)
               (recur (+ i 1) (* acc 2))
               (+ acc 1)))) []]
   ['(fn [^long n]
       (loop [i 0 acc 2]
         (if (< i n)
           (recur (+ i 1) (* acc acc))
           acc))) [6]]
   ;; 이질적(heterogeneous) index stride 누적합 miscompile 회귀: 두 then-recur 가
   ;; index 를 +3/+2 로 서로 다르게 증가. 예전엔 min-stride(=2) 닫힌식이 acc range 를
   ;; [0,20] 으로 *너무 좁게* 잡아(실제 i=0,3,5,7,9 → acc=24) 이후 (+ acc K) 를 unsound
   ;; raw LADD 로 승격 → host 는 throw 하는데 우리는 silent wraparound 였다. 이제
   ;; homogeneous-index-stride? gate 로 sound unroll/interval 경로로 fall through →
   ;; host/compiler 모두 long overflow throw.
   ['(fn [] (loop [i 0 acc 0]
             (if (< i 10)
               (if (< i 1)
                 (recur (+ i 3) (+ acc i))
                 (recur (+ i 2) (+ acc i)))
               (+ acc 9223372036854775787)))) []]
   ['(fn [] (reify clojure.lang.IObj
              (meta [_] {:k 1})
              (withMeta [this _] this))) []]
   ['(fn [] (try 1 (catch Object o 2))) []]
   ['(fn [^String s] (.length s)) [42]]
   ['(fn [] (Math/max 1 2.0)) []]
   ['(fn [x] x) []]])

(defn- try-val
  [f]
  (try {:val (f)} (catch Throwable t {:err (.getMessage t)})))

(defn- via-host     [form args] (apply (eval form) args))
(defn- via-compiler [form args] (apply (c/compile-form form) args))
(defn- via-kernel   [form args] (apply (k/k-eval form (k/fresh-env)) args))

(defn run
  []
  (mapv (fn [[form args]]
          (let [host (try-val #(via-host form args))
                comp (try-val #(via-compiler form args))
                kern (try-val #(via-kernel form args))
                hv   (:val host)
                hc-ok (and (contains? host :val)
                           (contains? comp :val)
                           (= hv (:val comp)))
                k-ok (if (contains? kern :val)
                       (= hv (:val kern))
                       :unsupported)]
            {:form form :args args :host host :compiler comp :kernel kern
             :hc-ok hc-ok :k-ok k-ok}))
        (all-cases)))

(defn run-negative
  []
  (mapv (fn [[form args]]
          (let [host (try-val #(via-host form args))
                comp (try-val #(via-compiler form args))
                ok   (and (contains? host :err)
                          (contains? comp :err))]
            {:form form :args args :host host :compiler comp :ok ok}))
        negative-cases))

(defn- show
  [m]
  (if (contains? m :val) (pr-str (:val m)) (str "ERR:" (:err m))))

(defn -main
  [& _]
  (let [results (run)
        negatives (run-negative)
        gate-ok (every? :hc-ok results)
        negative-ok (every? :ok negatives)
        k-checked (filter #(boolean? (:k-ok %)) results)
        k-pass    (filter #(true? (:k-ok %)) results)]
    (doseq [{:keys [form args host compiler kernel hc-ok k-ok]} results]
      (println (format "  [%s] %-46s args=%s → host=%s compiler=%s kernel=%s"
                       (if hc-ok "OK" "FAIL")
                       (pr-str form) (pr-str args)
                       (show host) (show compiler)
                       (cond (= :unsupported k-ok) "n/a"
                             (true? k-ok)          (str (show kernel) "✓")
                             :else                 (str (show kernel) "✗")))))
    (doseq [{:keys [form args host compiler ok]} negatives]
      (println (format "  [%s] negative %-38s args=%s → host=%s compiler=%s"
                       (if ok "OK" "FAIL")
                       (pr-str form) (pr-str args)
                       (show host) (show compiler))))
    (println (str "conformance host≡compiler: " (if gate-ok "ALL OK" "FAILED")
                  " (" (count (filter :hc-ok results)) "/" (count results) ")"))
    (println (str "  negative tests: " (if negative-ok "ALL OK" "FAILED")
                  " (" (count (filter :ok negatives)) "/" (count negatives) ")"))
    (println (str "  kernel 3-way 참고: " (count k-pass) "/" (count k-checked)
                  " 일치, " (count (filter #(= :unsupported (:k-ok %)) results)) " unsupported"))
    (shutdown-agents)
    (when-not (and gate-ok negative-ok)
      (System/exit 1))))
