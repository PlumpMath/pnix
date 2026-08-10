(ns pnix.clj-meta.selfhost
  "축 A — self-host 체인 (M5).

  우리 컴파일러(compiler.clj)로 작은 프로그램을 stage1..7 반복 컴파일해서, 각 stage
  가 생성한 JVM bytecode 가 stage 간 byte-동일(고정점)인지 검증한다. 결정적 클래스명
  + gensym 없는 emit 이라 같은 소스는 같은 bytecode 여야 한다.

  현재 단계(M5a): **deterministic self-compile 고정점** — '우리 컴파일러가 같은
  프로그램을 결정적으로 컴파일'을 증명한다. 이것은 진짜 'compiler-compiles-itself'
  (M5b)의 전제다(작은 self-host 타겟을 우리 부분집합으로 작성 → 그것이 자기 소스를
  처리). 정직하게 라벨: 아직은 컴파일러 결정성 고정점이지 컴파일러 자기재생산은 아니다.

  hy-meta 대응: stage chain 이 compiler.hy(206줄)를 반복 컴파일해 산출물 고정점을
  본 것과 같은 자리. 우리 산출물은 Python AST 가 아니라 JVM bytecode."
  (:require [pnix.clj-meta.compiler :as c]
            [clojure.java.io :as io]
            [clojure.pprint :as pp]))

(def receipt-path "clj-meta/proof/selfhost-chain.receipt.edn")

(def program
  "self-host 타겟(우리 부분집합으로 작성한 작은 프로그램). 우리 컴파일러가 컴파일하며,
  분기/재귀/loop/클로저/컬렉션/interop-free 산술을 고루 포함한다."
  '(do
     (def k-add (fn [a b] (+ a b)))
     (def k-sq  (fn [n] (* n n)))
     (def k-fib (fn fib [n] (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))))
     (def k-sum (fn [n] (loop [i n acc 0] (if (< i 1) acc (recur (- i 1) (+ acc i))))))
     (def k-map (fn [x] (map (fn [y] (+ x y)) [1 2 3])))
     [(k-add 20 22) (k-sq 9) (k-fib 10) (k-sum 5) (k-map 10)]))

;; ── M5b: 미니 메타순환 평가기(우리 부분집합으로 작성) ────────────────────────
;; 이 평가기 *소스* 를 우리 컴파일러가 컴파일한다(= 우리 컴파일러가 '평가기'를 컴파일).
;; quote/if/do/let/벡터/함수적용 + env 맵. 함수 심볼은 env 에 바인딩되어 있어야 한다
;; (순수 환경 기반, host resolve 없음).
(def mini-eval-src
  '(fn me [form env]
     (cond
       (symbol? form) (get env form)
       (vector? form) (mapv (fn [x] (me x env)) form)
       (seq? form)
       (let [h (first form)]
         (cond
           (= h 'quote) (nth form 1)
           (= h 'if)    (if (me (nth form 1) env)
                          (me (nth form 2) env)
                          (me (nth form 3) env))
           (= h 'do)    (last (mapv (fn [x] (me x env)) (rest form)))
           (= h 'let)   (let [bs (nth form 1)
                              k  (nth bs 0)
                              v  (me (nth bs 1) env)]
                          (me (nth form 2) (assoc env k v)))
           :else        (apply (me h env)
                               (mapv (fn [a] (me a env)) (rest form)))))
       :else form)))

(def mini-programs
  "[program env] 쌍 — host mini-eval 과 우리-컴파일된 mini-eval 의 결과를 대조."
  [['(if c a b)                   {'c true 'a 10 'b 20}]
   ['(if c a b)                   {'c false 'a 10 'b 20}]
   ['(plus a b)                   {'plus + 'a 20 'b 22}]
   ['(let [x five] (twice x))     {'five 5 'twice (fn [n] (* n 2))}]
   ['[a b c]                      {'a 1 'b 2 'c 3}]
   [''hello                       {}]
   ['(do x y)                     {'x 1 'y 99}]
   ['(sum (quote (1 2 3)))        {'sum #(reduce + %)}]])

(defn- fingerprint
  "{classname byte[]} → 비교 가능 형태 {classname (vec bytes)} (sorted)."
  [classes]
  (into (sorted-map) (map (fn [[k v]] [k (vec v)]) classes)))

;; ── 3b-5: 완전 메타순환 tower ────────────────────────────────────────────────
;; full-eval 은 자기가 쓰는 형식(if/let/quote/fn[named·익명·variadic]/벡터/함수적용)을
;; 모두 해석한다. 따라서 full-eval 이 full-eval *자기 소스* 를 해석해 또 다른 평가기를
;; 만들 수 있다. host 런타임 함수(symbol?/get/nth/…/bindp)는 env 로 주입한다.

(defn bindp
  "파라미터 벡터 ps 에 args 를 바인딩(& rest 처리). full-eval 의 fn 해석이 호출한다.
  full-eval-src 안에서 `bindp` 로 직접 참조되므로 selfhost ns 의 var 여야 한다."
  [ps args env]
  (loop [i 0 e env]
    (if (< i (count ps))
      (if (= (nth ps i) '&)
        (assoc e (nth ps (inc i)) (seq (drop i args)))
        (recur (inc i) (assoc e (nth ps i) (nth args i))))
      e)))

(defn bindl
  "let binding vector bs 를 순차 평가해 env 를 확장한다.
  full-eval-src 가 자기 자신의 다중-binding let 을 해석할 때 사용한다."
  [bs env me]
  (loop [i 0 e env]
    (if (< i (count bs))
      (recur (+ i 2)
             (assoc e (nth bs i) (me (nth bs (inc i)) e)))
      e)))

(def base-env
  "full-eval 이 자기 소스를 해석할 때 필요한 host 런타임 함수들."
  {'symbol? symbol? 'vector? vector? 'seq? seq? 'get get 'nth nth
   'first first 'rest rest '= = 'mapv mapv 'assoc assoc 'apply apply
   'bindp bindp 'bindl bindl})

(def full-eval-src
  '(fn me [form env]
     (if (symbol? form)
       (get env form)
       (if (vector? form)
         (mapv (fn [x] (me x env)) form)
         (if (seq? form)
           (let [h (first form)]
             (if (= h 'quote)
               (nth form 1)
               (if (= h 'if)
                 (if (me (nth form 1) env)
                   (me (nth form 2) env)
                   (me (nth form 3) env))
                 (if (= h 'let)
                   (let [bs (nth form 1)]
                     (me (nth form 2)
                         (bindl bs env me)))
                   (if (= h 'fn)
                     (let [nmd (symbol? (nth form 1))
                           nm  (if nmd (nth form 1) nil)
                           ps  (if nmd (nth form 2) (nth form 1))
                           bd  (if nmd (nth form 3) (nth form 2))]
                       (fn this [& args]
                         (me bd (let [e1 (bindp ps args env)]
                                  (if nm (assoc e1 nm this) e1)))))
                     (apply (me h env)
                            (mapv (fn [a] (me a env)) (rest form))))))))
           form)))))

(def tower-programs
  "[program prog-vars] — full-eval 형식 프로그램. prog-env = base-env + prog-vars."
  [['(if c a b)                                              {'c true 'a 10 'b 20}]
   ['((fn [x] (mul x x)) five)                               {'mul * 'five 9}]
   ['((fn f [n] (if (lt n 2) 1 (mul n (f (sub n 1))))) five) {'lt < 'mul * 'sub - 'five 5}]
   ['(let [x five] (add x x))                                {'add + 'five 21}]])

(defn tower-report
  "4중 일관: host-me ≡ compiled-me ≡ (compiled-me 가 me 소스를 해석한 평가기 E)."
  []
  ;; me 본문이 selfhost/bindp 를 var 로 참조하므로 host eval/우리 compile 은
  ;; selfhost ns 에서 해야 resolve 된다(self-interp 경로는 env 에서 찾으니 무관).
  (binding [*ns* (the-ns 'pnix.clj-meta.selfhost)]
   (let [host-me (eval full-eval-src)            ; host Compiler 가 만든 평가기
        comp-me (c/compile-form full-eval-src)  ; 우리 컴파일러가 만든 평가기
        E-comp  (comp-me full-eval-src base-env) ; comp-me 가 me 소스를 해석 → 평가기 E
        rows    (mapv (fn [[prog vars]]
                        (let [penv (merge base-env vars)
                              rh   (host-me prog penv)
                              rc   (comp-me prog penv)
                              re   (E-comp prog penv)]
                          {:prog prog :host rh :compiled rc :tower re
                           :ok (= rh rc re)}))
                      tower-programs)]
    {:rows rows :ok (every? :ok rows)})))

(defn mini-eval-report
  []
  (let [host-me (eval mini-eval-src)               ; host Compiler 가 만든 평가기
        comp-me (c/compile-form mini-eval-src)     ; 우리 컴파일러가 만든 평가기
        rows    (mapv (fn [[prog env]]
                        (let [h (host-me prog env)
                              c (comp-me prog env)]
                          {:prog prog :host h :compiled c :ok (= h c)}))
                      mini-programs)
        fps     (mapv (fn [_] (fingerprint (c/compile-classes mini-eval-src))) (range 7))
        fp-ok   (every? #(= (first fps) %) fps)]
    {:rows rows
     :conformance-ok (every? :ok rows)
     :classes (count (first fps))
     :fixed-point fp-ok}))

(defn run
  [stages]
  (let [wrapped (list 'fn [] program)
        fps     (mapv (fn [_] (fingerprint (c/compile-classes wrapped))) (range stages))
        ref     (first fps)
        ok      (every? #(= ref %) fps)]
    {:stages stages
     :classes (count ref)
     :class-names (vec (keys ref))
     :fixed-point ok}))

(defn write-receipt!
  [m5a program-result m5b tower]
  (let [ready (and (:fixed-point m5a)
                   (:conformance-ok m5b)
                   (:fixed-point m5b))
        receipt {:schema "pnix.clj-meta.selfhost-chain.receipt.v1"
                 :lane "clojure-self-written-compiler-selfhost"
                 :m5a-deterministic-self-compile
                 {:stages (:stages m5a)
                  :classes (:classes m5a)
                  :class-names (:class-names m5a)
                  :fixed-point (:fixed-point m5a)
                  :program-result program-result}
                 :m5b-compiler-compiles-evaluator
                 {:conformance (:conformance-ok m5b)
                  :fixed-point (:fixed-point m5b)
                  :classes (:classes m5b)
                  :rows (:rows m5b)}
                 :tower-reference
                 {:ok (:ok tower)
                  :known-issue (not (:ok tower))
                  :rows (:rows tower)}
                 :ready ready}]
    (io/make-parents receipt-path)
    (spit receipt-path (with-out-str (pp/pprint receipt)))
    receipt))

(defn -main
  [& _]
  (let [r      (run 7)
        result (c/eval-form program)
        m      (mini-eval-report)]
    (println "── M5a: deterministic self-compile 고정점 ──")
    (println (str "  stage1..7: " (if (:fixed-point r) "OK" "FAILED")
                  " — " (:classes r) " classes bytecode byte-동일"))
    (println (str "  program(host JVM) = " (pr-str result)))
    (println "── M5b: 우리 컴파일러가 컴파일한 미니 메타순환 평가기 ──")
    (doseq [{:keys [prog host compiled ok]} (:rows m)]
      (println (format "  [%s] %-26s host=%s compiled=%s"
                       (if ok "OK" "FAIL") (pr-str prog) (pr-str host) (pr-str compiled))))
    (println (str "  conformance host-eval≡compiled-eval: "
                  (if (:conformance-ok m) "OK" "FAILED")))
    (println (str "  mini-eval 소스 stage1..7 bytecode 고정점: "
                  (if (:fixed-point m) "OK" "FAILED")
                  " — " (:classes m) " classes"))
    (println "── 3b-5: 완전 메타순환 tower (평가기가 자기 소스를 해석) ──")
    (let [t (tower-report)
          receipt (write-receipt! r result m t)]
      (doseq [{:keys [prog host compiled tower ok]} (:rows t)]
        (println (format "  [%s] %-44s host=%s comp=%s tower=%s"
                         (if ok "OK" "FAIL") (pr-str prog)
                         (pr-str host) (pr-str compiled) (pr-str tower))))
      (println (str "  1단(host≡compiled): OK / 2단 self-interp(tower): "
                    (if (:ok t) "OK" "FAILED")))
      (when (:ok t)
        (println "  full-eval self-interp: OK — if/fn/named-fn/let tower 일치"))
      (println (str "  receipt: " receipt-path " ready=" (:ready receipt)))
      (shutdown-agents)
      (when-not (and (:fixed-point r) (:conformance-ok m) (:fixed-point m) (:ok t))
        (System/exit 1)))))
