"Kernel feature proof module."

(import __future__ [annotations])
(import re string)
(import typing [List get-type-hints])

(defmacro inc [x]
  `(+ ~x 1))

(defmacro answer []
  '42)

(defmacro add-all [xs]
  `(+ ~@xs))

(setv quasiquote_splice_source '(c d e))
(setv quasiquote_splice_side_effects [])
(setv quasiquote_falsey_splice
      `(a b
          ~@quasiquote_splice_source
          f
          ~@quasiquote_splice_source
          ~@0
          ~@False
          ~@None
          g
          ~@(when False 1)
          h))
(setv quasiquote_single_eval_splice
      `(x ~@(do (.append quasiquote_splice_side_effects "once") [1 2]) y))
(setv quasiquote_nested
      (hy.as-model `(1 `~(+ 1 ~(+ 2 3) ~@None) 4)))
(setv quasiquote_nested_struct
      (hy.as-model
        `(try
           ~@(lfor i [1 2 3]
               `(setv ~(hy.models.Symbol (+ "x" (str i)))
                      (+ "x" (str ~i))))
           (finally
             (print "done")))))
(setv quasiquote_triple_a 1 quasiquote_triple_b 1 quasiquote_triple_c 1)
(setv quasiquote_triple_eval
      ```[~quasiquote_triple_a ~~quasiquote_triple_b ~~~quasiquote_triple_c])
(setv quasiquote_triple_a 2 quasiquote_triple_b 2 quasiquote_triple_c 2)
(setv quasiquote_triple_eval (hy.eval quasiquote_triple_eval))
(setv quasiquote_triple_a 3 quasiquote_triple_b 3 quasiquote_triple_c 3)
(setv quasiquote_triple_eval (hy.as-model (hy.eval quasiquote_triple_eval)))

(setv add10 ((fn [x]
               (fn [y] (+ x y)))
             10))

(defn optional-bonus [[x 5]]
  x)

(defn rest-count [x #* xs]
  (+ x (len xs)))

(defn kw-bonus [x [y 1] #** kw]
  (+ x y (.get kw "z")))

(defn kw-spread [#** kw]
  (.get kw "bonus"))

(defn feature-doc-fn []
  "feature function docstring"
  42)

(defn feature-single-string-fn []
  "feature return string")

(defn pair-total [#(x y)]
  (+ x y))

(defn annotated-pair-total [#^ tuple #(x y)]
  (+ x y))

(defn posonly-total [x / y]
  (+ x y))

(defn kwonly-total [* required [bonus 3]]
  (+ required bonus))

(defn kwonly-pair-default [* [#(x y) [20 22]]]
  (+ x y))

(defn kwonly-pair-required [* #(x y)]
  (+ x y))

(defn mixed-lambda-list [a / [b 2] #* rest required [bonus 5] #** kw]
  (+ a b (len rest) required bonus (.get kw "extra")))

(setv lambda-list-score ((fn [x / * y [z 1]]
                            (+ x y z))
                          20
                          :y 21))
(setv statement-fn (fn [x]
                     (setv y (+ x 1))
                     y))
(setv closure-statement-fn ((fn [base]
                              (fn [x]
                                (setv y (+ base x))
                                y))
                            30))
(setv fstring_p "xyzzy")
(setv fstring_foo "bar")
(setv fstring_value 12.34)
(setv fstring_width 10)
(setv fstring_precision 4)
(setv fstring_events [])
(defclass FstringFormatClass [object]
  (defn __format__ [self format-spec]
    (+ "C[" format-spec "]")))
(setv fstring_pi 3.141593)
(setv fstring_fill "_")
(setv fstring_values
      [f"hello world"
       f"hello {(+ 1 1)} world"
       f"a{1}{2}b"
       f"ab{{cde"
       f"ab{{cde}}}}fg{{{{{{"
       f"ab{{{(+ 1 1)}}}"
       f"a{(.upper (+ "g" "k"))}z"
       f"h{fstring_p}j"
       f"a{(do (setv fstring_loop 4) (* fstring_loop 2))}z"
       f"a{fstring_p !r}"
       f"a{fstring_p :9}"
       f"a{fstring_p !r :9}"
       f"{2 :{(+ 2 2)}}"
       f"result: {fstring_value :{fstring_width}.{fstring_precision}}"
       f"{fstring_foo =}"
       f"xyz{  fstring_foo = }"
       f"{ fstring_foo = !s}"
       #[f[a{fstring_p !r :9}]f]
       #[f-string[result: {fstring_value :{fstring_width}.{fstring_precision}}]f-string]
       #[f[{{escaped braces}} \n {"not escaped"}]f]
       #[f["{0}"]f]
       f"{(FstringFormatClass) :  {(str (+ 1 1)) !r :x<5}}"
       f"{fstring_pi = :{fstring_fill}^8.2f}"
       f"{(do (.append fstring_events "value") 2) :{(do (.append fstring_events "spec") 4)}}"
       fstring_loop
       fstring_events])
(setv quoted_bracket '#[feature[quote body]feature])
(setv quoted_fstring 'f"quote {quoted_world}")
(setv quoted_fstring_missing False)
(try
  (hy.eval quoted_fstring (globals) (globals) "hy")
  (except [NameError]
    (setv quoted_fstring_missing True)))
(setv quoted_world "ready")
(setv quoted_fstring_component (get quoted_fstring 1))
(setv quoted_fstring_value (hy.eval quoted_fstring (globals) (globals) "hy"))
(setv quoted_fstring_repr_roundtrip [])
(for [orig ['f"hello {(+ 1 1)} world"
            'f"a{fstring_p !r:9}"
            'f"{ fstring_foo = !s}"]]
  (setv new (eval (repr orig)))
  (.append quoted_fstring_repr_roundtrip
           [(= (len new) (len orig))
            (list (map (fn [item]
                         (getattr item "conversion" None))
                       new))
            (= new orig)]))
(defn keyword-proof-kwargs [#** kwargs]
  kwargs)
(defclass FeatureKeywordLookup []
  (defn __getitem__ [self key]
    key))
(setv keyword_empty :)
(setv keyword_pickled :feature-keyword)
(setv keyword_lookup :foo)
(setv keyword_values
      [(= :foo :foo)
       (!= :foo :bar)
       (= (get {:foo "bar"} :foo) "bar")
       (= (get {:foo "bar" ":foo" "quux"} :foo) "bar")
       (= (get {:foo "bar" ":foo" "quux"} ":foo") "quux")
       (= keyword_empty ':)
       (= (. keyword_empty name) "")
       (< :a :b)
       (= (sorted [:b :a :c]) [:a :b :c])
       (= (:foo (dict :foo "test")) "test")
       (= (keyword_lookup (dict :foo "test")) "test")
       (= (:foo (dict :a 1) 3) 3)
       (= (:foo (dict :a 1 :foo 5) 3) 5)
       (= (:foo-bar (dict :foo-bar "baz")) "baz")
       (= (:foo-bar (FeatureKeywordLookup)) "foo_bar")
       (= (keyword-proof-kwargs :key-with-dashes "value")
          {"key_with_dashes" "value"})])

(setv hy_eval_argument_x 2)
(setv hy_eval_argument_payload '(+ hy_eval_argument_x 2))
(setv hy_eval_outer "O")
(setv hy_eval_globals_dict {"g1" 1 "g2" 2})
(setv hy_eval_locals_dict {"l1" 1 "l2" 2})
(setv hy_eval_basic (= (hy.eval '(+ 1 1)) 2))
(setv hy_eval_before_set (= (hy.eval '(+ hy_eval_argument_x 2)) 4))
(setv hy_eval_after_set
      (do
        (setv hy_eval_argument_x 4)
        (= (hy.eval hy_eval_argument_payload) 6)))
(hy.eval :globals hy_eval_globals_dict :locals hy_eval_locals_dict
         '(do
            (global g2 g3)
            (setv g2 "newg" g3 3 l2 "newl" l3 4)))
(del (get hy_eval_globals_dict "__builtins__"))
(setv hy_eval_outer_unchanged
      (do
        (hy.eval :globals {"outer" "I"}
                 '(do (global hy_eval_outer)
                      (setv hy_eval_outer "O3")))
        (= hy_eval_outer "O")))
(setv hy_eval_argument_values
      [hy_eval_basic
       hy_eval_before_set
       hy_eval_after_set
       (= ((hy.eval '(fn [x] (+ 3 3 x))) 3) 9)
       (is (hy.eval 're) re)
       (is (hy.eval 'False) False)
       (is (hy.eval 'None) None)
       (= (hy.eval '0) 0)
       (= (hy.eval '"") "")
       (= (hy.eval 'b"") b"")
       (= (hy.eval ':) :)
       (= (hy.eval '[]) [])
       (= (hy.eval '#()) #())
       (= (hy.eval '{}) {})
       (= (hy.eval '#{}) #{})
       (= (hy.eval 'digits :module string) "0123456789")
       (= (hy.eval 'digits :module "string") "0123456789")
       (= (hy.eval 'digits :module string :globals {"digits" "boo"}) "boo")
       hy_eval_outer_unchanged
       (= hy_eval_globals_dict {"g1" 1 "g2" "newg" "g3" 3})
       (= hy_eval_locals_dict {"l1" 1 "l2" "newl" "l3" 4})])

(setv call_argument_order_events [] call_argument_order_x 1)
(defn call_argument_order_capture [a b #* rest #** kw]
  [a b rest kw call_argument_order_events])
(setv call_argument_order_values
      (call_argument_order_capture
        :k (do (.append call_argument_order_events "kw") call_argument_order_x)
        (do
          (.append call_argument_order_events "pos")
          (setv call_argument_order_x 2)
          call_argument_order_x)
        #*(do (.append call_argument_order_events "star") [3])
        :j (do (.append call_argument_order_events "kw2") 4)
        #**(do (.append call_argument_order_events "kwpack") {"m" 5})))
(setv mangling_special_form_alias_values
      (let [a-b 1
            -a-_b- 2
            -_- 3
            foo? "nachos"
            $ "dosh"
            not-in 5
            is-not 6
            + 7
            left []
            right []]
        [(= [a-b a_b] [1 1])
         (= [-a-_b- -a--b- -a__b-] [2 2 2])
         (= [-_- -__] [3 3])
         (= [foo? hyx_fooXquestion_markX] ["nachos" "nachos"])
         (= [$ hyx_Xdollar_signX] ["dosh" "dosh"])
         (= [+ hyx_Xplus_signX] [7 7])
         (is (not-in 2 [1 2 3]) False)
         (is (not_in 2 [1 2 3]) False)
         (is (not-in 4 [1 2 3]) True)
         (is (not_in 4 [1 2 3]) True)
         (is (is-not left right) True)
         (is (is_not left right) True)]))

(import math)
(import pickle)
(setv root (.sqrt math 81))
(setv local-sum (let [a 7 b 8] (+ a b)))
(setv destructured-let (let [[a b] [6 7]] (+ a b)))
(setv annotated-let-score (let [(annotate value int) 42] value))
(setv annotated-destructured-let-score
      (let [[(annotate left int) right] [20 22]]
        (+ left right)))
(setv let-statement-score
      (let [value 40]
        (setv bumped (+ value 2))
        bumped))
(setv let-match-score
      (let [x 3 y 4]
        (match x
               y (= x y))))
(setv let-match-sequence-score
      (let [x 1 y 2]
        (match [5 6]
               [x y] [x y])))
(setv let-rebind-score
      (let [x "foo"
            y "bar"
            x (+ x y)
            y (+ y x)
            x (+ x x)]
        [x y]))
(setv let-early-score
      (let [a "a"
            b (+ a "b")
            c (+ b "c")]
        c))
(setv let-unpacking-score
      (let [[a b] [1 2]
            [lhead #* ltail] (range 3)
            #(thead #* ttail) (range 3)
            [nhead #* #(c #* nrest)] [0 1 2]]
        [a b lhead ltail thead ttail nhead c nrest]))
(setv let-unpacking-rebind-score
      (let [[a b] [:foo :bar]
            [a #* c] (range 3)
            [head #* tail] [a b c]]
        [a b c head tail]))
(let [hidden-let-binding 1]
  (setv hidden-let-binding 2))
(setv hidden-let-binding-missing False)
(try
  hidden-let-binding
  (except [NameError]
    (setv hidden-let-binding-missing True)))
(let [let-leak-base 1]
  (setv let-leaked-setv 2))
(let [let-leak-base 1]
  (defn let-leaked-function [] 42))
(let [let-leak-base 1]
  (defclass LetLeakedClass []))
(let [types 6]
  (import types))
(let [sqrt 6]
  (import math [sqrt]))
(let [let-leak-base 1]
  (for [let-leaked-for [42]] None))
(let [let-leak-base 1]
  (match [20 22]
         [let-leaked-left let-leaked-right] None))
(assert hidden-let-binding-missing)
(assert (= let-leaked-setv 2))
(assert (= (let-leaked-function) 42))
(assert (= (. LetLeakedClass __name__) "LetLeakedClass"))
(assert (= (. (type types) __name__) "module"))
(assert (= (sqrt 1764) 42.0))
(assert (= let-leaked-for 42))
(assert (= (+ let-leaked-left let-leaked-right) 42))
(setv packed {"bonus" 5})
(setv [pair-a pair-b] [2 3])
(setv unpacked-list [1 #* [2 3] 4])
(setv unpacked-tuple #("a" #* ["b" "c"]))
(setv unpacked-set #{1 #* [1 2] 3})
(setv unpacked-dict {"a" 1 #** {"b" 2} "c" 3})
(setv collection-pending-x 1)
(setv collection-pending-list
      [collection-pending-x
       (do (setv collection-pending-x 2) collection-pending-x)])
(setv collection-pending-x 1)
(setv collection-pending-tuple
      #(collection-pending-x
        (do (setv collection-pending-x 2) collection-pending-x)))
(setv collection-pending-x 1)
(setv collection-pending-set
      #{collection-pending-x
        (do (setv collection-pending-x 2) collection-pending-x)})
(setv collection-pending-x 1)
(setv collection-pending-dict-key
      {(do (setv collection-pending-x 2) collection-pending-x)
       collection-pending-x})
(setv collection-pending-x 1)
(setv collection-pending-dict-value
      {collection-pending-x
       (do (setv collection-pending-x 2) collection-pending-x)})
(setv collection-pending-x 1)
(setv collection-pending-list-unpack
      [collection-pending-x
       #*(do (setv collection-pending-x 2) [collection-pending-x 3])
       collection-pending-x])
(setv collection-pending-x 1)
(setv collection-pending-dict-unpack
      {collection-pending-x 10
       #**(do (setv collection-pending-x 2) {collection-pending-x 20})
       collection-pending-x 30})
(setv operator-unpack-score
      (+ (+ #* [1 2 3] #* [4 5])
         (* #* [2 3 7])
         (* #* [])
         (and #* [1 2 3])
         (or #* [False 0 42])
         (if (< #* [1 2 3]) 5 0)
         (if (= #* [1 1 1]) 6 0)
         (len (+ #* [[1] [2]]))))
(setv boolop-del-list ["a" "b"])
(setv boolop-and-del-skip (and 0 (del (get boolop-del-list 1))))
(setv boolop-and-del-run (and 15 (del (get boolop-del-list 1))))
(assert (= boolop-and-del-skip 0))
(assert (is boolop-and-del-run None))
(assert (= boolop-del-list ["a"]))
(setv boolop-for-list [])
(setv boolop-or-for-skip
      (or 15 (for [n [1 2]]
               (.append boolop-for-list n))))
(setv boolop-or-for-run
      (or 0 (for [n [1 2]]
              (.append boolop-for-list n))))
(assert (= boolop-or-for-skip 15))
(assert (is boolop-or-for-run None))
(assert (= boolop-for-list [1 2]))
(setv [unpack-head #* unpack-tail] [10 20 12])
(setv dfor-unpacked
      (dfor pair [["a" 10] ["b" 32]]
            #** {(get pair 0) (get pair 1)}))
(setv control-score (cond False 1 (> root 8) 17 True 0))
(setv when-score (when (> control-score 10) 4))
(setv statement-score 0)
(when True
  (setv statement-score 8))
(cond False
      (setv statement-score 1)
      True
      (setv statement-score (+ statement-score 9)))
(setv conditional-expression-if-x 0)
(setv conditional-expression-if
      [(if False (setv conditional-expression-if-x 1) 2)
       conditional-expression-if-x])
(setv conditional-expression-when-x 0)
(setv conditional-expression-when
      [(when False (setv conditional-expression-when-x 1))
       conditional-expression-when-x])
(setv conditional-expression-cond-y 0)
(setv conditional-expression-cond
      [(cond False 2 True (setv conditional-expression-cond-y 1))
       conditional-expression-cond-y])
(setv for-score 0)
(for [item [1 2 3 4]]
  (setv for-score (+ for-score item)))
(setv for-pending-iter-source "")
(setv for-pending-iter-values [])
(for [outer-item "ab"
      inner-item (do
                   (+= for-pending-iter-source "y")
                   "de")]
  (.append for-pending-iter-values
           (+ outer-item inner-item for-pending-iter-source)))
(setv loop-else-score 0)
(for [item [1 2]]
  (setv loop-else-score (+ loop-else-score item))
  (else
    (setv loop-else-score (+ loop-else-score 4))))
(while (< loop-else-score 10)
  (setv loop-else-score (+ loop-else-score 1))
  (else
    (setv loop-else-score (+ loop-else-score 2))))
(setv while-do-score "")
(setv while-do-x 2)
(while (do
         (+= while-do-score "a")
         while-do-x)
  (+= while-do-score "b")
  (-= while-do-x 1)
  (else
    (+= while-do-score "z")))
(setv while-do-continue-score "")
(setv while-do-continue-x 2)
(setv while-do-continued False)
(while (do
         (+= while-do-continue-score "a")
         while-do-continue-x)
  (+= while-do-continue-score "b")
  (when (and (= while-do-continue-x 1) (not while-do-continued))
    (+= while-do-continue-score "c")
    (setv while-do-continued True)
    (continue))
  (-= while-do-continue-x 1)
  (else
    (+= while-do-continue-score "z")))
(setv while-condition-break-score "")
(for [outer-x "123"]
  (+= while-condition-break-score outer-x)
  (setv inner-y 0)
  (while (do
           (when (and (= outer-x "2") (= inner-y 1))
             (break))
           (< inner-y 3))
    (+= while-condition-break-score "y")
    (+= inner-y 1)))
(defn break-loop-value []
  (for [break-loop-x (range 10)]
    (when (= break-loop-x 5)
      (break)))
  break-loop-x)
(setv continue-loop-values [])
(for [continue-loop-x (range 10)]
  (when (!= continue-loop-x 5)
    (continue))
  (.append continue-loop-values continue-loop-x))
(setv match-score 0)
(match [4 5]
  [a b]
  (setv match-score (+ a b 3))
  _
  (setv match-score 0))
(match 5
  4
  (setv match-score 0)
  5
  (setv match-score (+ match-score 30))
  _
  (setv match-score 0))
(setv match-map-score 0)
(match {"left" 20 "right" 22}
  {"left" left "right" right}
  (setv match-map-score (+ left right))
  _
  (setv match-map-score 0))
(setv match-guard-score 0)
(match [10 32]
  [a b] :if (= (+ a b) 0)
  (setv match-guard-score 0)
  [a b] :if (= (+ a b) 42)
  (setv match-guard-score (+ a b))
  _
  (setv match-guard-score 0))
(defclass MatchPoint []
  (setv __match_args__ (tuple ["x" "y"]))
  (defn __init__ [self x y]
    (setv (. self x) x)
    (setv (. self y) y)))
(defclass BareFeatureClass)
(defclass FeatureMeta [type])
(defclass FeatureWithMeta [:metaclass FeatureMeta])
(defclass FeaturePreparedDict [dict]
  (defn __setitem__ [self key value]
    (dict.__setitem__ self (+ "prepared_" key) value)))
(defclass FeaturePreparedMeta [type]
  (defn [classmethod] __prepare__ [metacls name bases]
    (FeaturePreparedDict)))
(defclass FeaturePrepared [:metaclass FeaturePreparedMeta]
  (defn [classmethod] method [cls] 7))
(defclass FeatureBase []
  (defn __init-subclass__ [cls swallow #** kwargs]
    (setv (. cls swallow) swallow)))
(defclass FeatureChild [FeatureBase :swallow "african"])
(defclass FeatureDocClass []
  "feature class docstring"
  (setv value 42))
(defclass FeatureDynamicBase [((fn [] (if True list dict)))]
  (setv value 42))
(defclass FeatureNoLeak []
  (setv class_body_fn (fn [] 1)))
(setv feature_class_no_leak False)
(try
  (class_body_fn)
  (except [NameError]
    (setv feature_class_no_leak True)))
(setv feature_class_side_effect False)
(defn set-feature-class-side-effect []
  (global feature_class_side_effect)
  (setv feature_class_side_effect True))
(defclass FeatureClassSideEffect []
  (set-feature-class-side-effect))
(assert (= (. BareFeatureClass __name__) "BareFeatureClass"))
(assert (is (type FeatureWithMeta) FeatureMeta))
(assert (= (FeaturePrepared.prepared-method) 7))
(assert (= (. (FeatureChild) swallow) "african"))
(assert (isinstance (FeatureDynamicBase) list))
(assert feature_class_no_leak)
(assert feature_class_side_effect)
(setv match-class-score 0)
(match (MatchPoint 11 31)
  (MatchPoint x y)
  (setv match-class-score (+ x y))
  _
  (setv match-class-score 0))
(setv match-class-kw-score 0)
(match (MatchPoint 11 31)
  (MatchPoint :y y :x x)
  (setv match-class-kw-score (+ x y))
  _
  (setv match-class-kw-score 0))
(setv match-or-score 0)
(match 7
  (| 6 7)
  (setv match-or-score 42)
  _
  (setv match-or-score 0))
(setv match-as-score 0)
(match [10 32]
  (as [x y] pair)
  (setv match-as-score (+ (get pair 0) (get pair 1)))
  _
  (setv match-as-score 0))
(defclass NativeMatchBox []
  (setv VALUE 42))
(setv native-match-statement-events [])
(match 1
       1 :if (do (.append native-match-statement-events 1) False)
       (.append native-match-statement-events 2)
       1 :if False
       (.append native-match-statement-events 3)
       _ :if (do (.append native-match-statement-events 4) True)
       (.append native-match-statement-events 5))
(setv native-match-expr-results
      [(match 0
              1 "no"
              0 "zero")
       (match 4
              (| 0 1 2 3) True)
       (match 1
              1 :if True ':as)
       (match :hello
              :hello "keyword"
              _ "missing")
       (match [0 1 2]
              [0 #* xs] :as whole
              :if (do
                    (setv guard-len (len whole))
                    (= guard-len 3))
              (sum xs))
       (match 42
              NativeMatchBox.VALUE "dotted"
              _ "missing")
       (match 99)])
(setv match-star-score 0)
(match [1 2 3 4]
  [1 #* middle 4]
  (setv match-star-score (sum middle))
  _
  (setv match-star-score 0))
(setv match-rest-score 0)
(match {"keep" 7 "a" 10 "b" 25}
  {"keep" keep #** rest}
  (setv match-rest-score (+ keep (get rest "a") (get rest "b")))
  _
  (setv match-rest-score 0))
(setv try-score 0)
(try
  (raise (ValueError "kernel"))
  (except [ValueError err]
    (setv try-score 11))
  (finally
    (setv try-score (+ try-score 2))))
(setv try-expr-score
      (+ (try (+ 1 2))
         (try 0 (else (+ 3 4)))
         (try
           (raise (ValueError "value"))
           (except [ValueError err]
             (+ 5 6)))
         (try 13
           (finally
             (pass)))))
(setv native-except-values
      [(try
         (get "foo" 5)
         (except [[IndexError NameError]]
           "type-list")
         (except []
           "fallback"))
       (try
         (abs "hi")
         (except [e TypeError]
           (type e)))
       (try
         (get {1 2} 3)
         (except [e [KeyError AttributeError]]
           [(type e) "name-list"]))
       (try
         (raise ValueError)
         (except [[]]
           "empty-list-caught")
         (except []
           "fallback"))
       (try
         (raise (ValueError "type-first"))
         (except [ValueError err]
           (get (getattr err "args") 0)))])
(setv try-outer-effect-x 1)
(setv try-outer-effect-y 0)
(setv try-outer-effect-x
      (try
        (+ "G" "H")
        (except [NameError]
          (+ "I" "J"))
        (else
          (setv try-outer-effect-y 1)
          (assert (= try-outer-effect-x 1))
          (+ "K" "L"))))
(setv try-except-outer-effect-y 0
      try-except-outer-effect-out
      (try
        (raise (ValueError "bad"))
        (except [ValueError err]
          (setv try-except-outer-effect-y 42)
          "ok")))
(setv try-star-score 0)
(try
  (raise (ExceptionGroup "kernel" [(ValueError "star")]))
  (except* [ValueError err]
    (setv try-star-score 14)))
(setv try-star-expr-events [])
(setv try-star-expr-score
      (try
        (raise (ExceptionGroup "kernel" [(KeyError "key") (ValueError "value")]))
        (except* [KeyError err]
          (.append try-star-expr-events "key")
          21)
        (except* [ValueError err]
          (.append try-star-expr-events "value")
          42)
        (finally
          (.append try-star-expr-events "finally"))))
(setv raise-cause-score 0)
(try
  (try
    (raise (ValueError "inner"))
    (except [ValueError err]
      (raise (RuntimeError "outer") :from err)))
  (except [RuntimeError err]
    (when (isinstance (getattr err "__cause__") ValueError)
      (setv raise-cause-score 15))))
(defclass FeatureBox []
  (setv score 19))
(defn add-decorated [f]
  (fn [] (+ (f) 23)))
(defn [add-decorated] decorated-score []
  20)
(defn class-score [cls]
  (setattr cls "bonus" 7)
  cls)
(defclass [class-score] DecoratedBox [])
(defclass AssignedBox [])
(setv (. AssignedBox points) 3)
(import math [floor :as down])
(import math *)
(import asyncio)
(import contextlib [nullcontext])
(setv import-score (down 5.9))
(setv import-star-score (ceil 41.2))
(setv subs [1 2 3])
(setv (get subs 1) 6)
(setv subscript-score (+ (get subs 0) (get subs 1) (get subs 2)))
(setv multi-subscript-items [[1 2] [3 4]])
(setv (get multi-subscript-items 1 0) 9)
(setv multi-subscript-values
      [(get [[1 2 3] [4 5 6] [7 8 9]] 1 2)
       (get {"x" {"y" {"z" 12}}} "x" "y" "z")
       multi-subscript-items])
(setv dot-chain-items [[10 20 12]])
(setv (. dot-chain-items [0] [1]) 40)
(setv dot-chain-score
      (+ (. dot-chain-items [0] [1])
         (len (. "ab hello" (strip "ab ") (upper)))
         (if (= (. "abc" __class__ __name__ [0]) "s") -3 0)))
(defclass FeatureMethodShortcut [object]
  (defn method [self #* args #** kwargs]
    (.join " " (+ #("method") args
      (tuple (map (fn [k] (get kwargs k))
                  (sorted (.keys kwargs))))))))
(setv dot-method-shortcut-box (FeatureMethodShortcut))
(setv dot-method-shortcut-values
      [(.method dot-method-shortcut-box)
       (.method dot-method-shortcut-box "foo" "bar")
       (.method :b "1" :a "2" dot-method-shortcut-box "foo" "bar")
       (.method dot-method-shortcut-box #* ["foo" "bar"])
       (is (. dot-chain-items) dot-chain-items)
       ((. "" join) ["aa" "bb"])])
((. defn) dotted_root_function [] "ok")
(setv aug-score 1)
(+= aug-score 2 3)
(setv operator-score
      (+ (% 20 6)
         (// 20 3)
         (** 2 5)
         (& 7 3)
         (invert -1)
         (- (| 8 1) 10)))
(defclass FeaturePosable [object]
  (defn __pos__ [self]
    "called __pos__"))
(setv operator-edge-values
      [(/ 2)
       (/ 8 2 2 2)
       (** 5 4 3)
       (** #* [5 4 3])
       (|)
       (| #* [])
       (| 5)
       (& 5)
       (@ 5)
       (bnot 0b00101111)
       (+ (FeaturePosable))
       (+ #* [])
       (* #* [])
       (/ #* [2])])
(setv aug-operator-score 45)
(%= aug-operator-score 43)
(<<= aug-operator-score 4)
(|= aug-operator-score 10)
(setv aug-b 2)
(setv aug-c 3)
(setv aug-d 4)
(defclass FeatureMatmulBox [object]
  (defn __init__ [self content]
    (setv (. self content) content))
  (defn __matmul__ [self other]
    (FeatureMatmulBox (+ (. self content) (. other content)))))
(setv aug-edge-a 4)
(+= aug-edge-a aug-b aug-c aug-d)
(setv aug-edge-add aug-edge-a)
(setv aug-edge-a 4)
(-= aug-edge-a aug-b aug-c aug-d)
(setv aug-edge-sub aug-edge-a)
(setv aug-edge-a 4)
(*= aug-edge-a aug-b aug-c aug-d)
(setv aug-edge-mul aug-edge-a)
(setv aug-edge-a 4)
(**= aug-edge-a aug-b aug-c)
(setv aug-edge-pow aug-edge-a)
(setv aug-edge-a 4)
(/= aug-edge-a aug-b aug-c aug-d)
(setv aug-edge-div aug-edge-a)
(setv aug-edge-a 4)
(//= aug-edge-a aug-b aug-c aug-d)
(setv aug-edge-floor aug-edge-a)
(setv aug-edge-a 4)
(<<= aug-edge-a aug-b aug-c aug-d)
(setv aug-edge-left aug-edge-a)
(setv aug-edge-a 4)
(>>= aug-edge-a aug-b aug-c aug-d)
(setv aug-edge-right aug-edge-a)
(setv aug-edge-a 4)
(&= aug-edge-a aug-b aug-c aug-d)
(setv aug-edge-and aug-edge-a)
(setv aug-edge-a 4)
(|= aug-edge-a aug-b aug-c aug-d)
(setv aug-edge-or aug-edge-a)
(setv aug-edge-a 15)
(%= aug-edge-a 9)
(setv aug-edge-mod aug-edge-a)
(setv aug-edge-a 0b1100)
(^= aug-edge-a 0b1010)
(setv aug-edge-xor aug-edge-a)
(setv aug-edge-box (FeatureMatmulBox "a"))
(setv aug-edge-b-box (FeatureMatmulBox "b"))
(setv aug-edge-c-box (FeatureMatmulBox "c"))
(setv aug-edge-d-box (FeatureMatmulBox "d"))
(@= aug-edge-box aug-edge-b-box aug-edge-c-box aug-edge-d-box)
(setv aug-edge-values
      [aug-edge-add
       aug-edge-sub
       aug-edge-mul
       aug-edge-pow
       aug-edge-div
       aug-edge-floor
       aug-edge-left
       aug-edge-right
       aug-edge-and
       aug-edge-or
       aug-edge-mod
       aug-edge-xor
       (. aug-edge-box content)])
(setv (annotate annotated_score int) 42)
(annotate bare_annotated str)
(defclass FeatureAnnotationContainer []
  (setv #^ int x 1 y 2)
  (#^ bool z))
(setv feature_annotation_hints (get-type-hints FeatureAnnotationContainer))
(defn #^ int annotated_fn [#^ int value]
  value)
(defn #^ int annotated_complex_fn [#^ (get List int) values
                                   #^ str #* rest
                                   #^ bool #** kwargs]
  (len values))
(setv annotated_complex_hints (get-type-hints annotated_complex_fn))
(setv annotated_lambda (fn #^ int [#^ int value] value))
(setv annotated_pos_kw (fn #^ int [#^ int x / * #^ int y] (+ x y)))
(setv del-items [4 5 6])
(del (get del-items 1))
(setv del-score (len del-items))
(setv bool-score (if (and True (not False)) (or False 12) 0))
(setv do-effect-x "a")
(setv do-effect-y (do
                    (setv do-effect-x "b")
                    "c"))
(setv do-effect-when 0)
(when (do
        (setv do-effect-when 1)
        True)
  (setv do-effect-when (+ do-effect-when 41)))
(setv ellipsis-original Ellipsis)
(setv Ellipsis 14)
(setv ellipsis-score
      (if (and (= Ellipsis 14) (!= ... 14) (is ... ellipsis-original))
          42
          0))
(setv setx-score (if (setx setx_value 42) setx_value 0))
(setx setx_chain_y (+ (setx setx_chain_x (+ "a" "b")) "c"))
(setv setx_filter_items ["apple" None "banana"])
(setv setx_filter_values
      (lfor index (range (len setx_filter_items))
            :if (is-not (setx setx_filter_kept
                              (get setx_filter_items index))
                         None)
            setx_filter_kept))
(defn setx-helper-existing []
  (setv outer 20)
  (lfor n (range 10)
        :do outer
        (setx outer n))
  outer)
(defn setx-helper-new []
  (setv outer 20)
  (lfor n (range 10)
        :do outer
        (setx created n))
  created)
(defn setx-helper-empty []
  (setv outer 2)
  (lfor n (range 0)
        :do outer
        (setx never n))
  never)
(setv setx-empty-error "")
(try
  (setx-helper-empty)
  (except [err UnboundLocalError]
    (setv setx-empty-error (. err __class__ __name__))))
(setv pending-setv-order [])
(setv pending-setv-a 1
      pending-setv-b (try
                       (.append pending-setv-order pending-setv-a)
                       (setv pending-setv-a 2)
                       3))
(setv :chain [chain-a chain-b chain-c] 3)
(setv chain-v1 1
      :chain [chain-v2 chain-v3] 2
      chain-v4 4
      :chain [chain-v5 chain-v6 chain-v7] 5)
(setv :chain [[chain-y #* chain-z chain-w]
              chain-x
              [chain-aa chain-bb chain-cc chain-dd]]
      "abcd")
(setv chain-order-list (* [0] 5))
(setv chain-order-calls [])
(defn chain-order-index [index]
  (.append chain-order-calls [index (list chain-order-list)])
  index)
(setv :chain [(get chain-order-list (chain-order-index 1))
              (get chain-order-list (chain-order-index 2))
              (get chain-order-list (chain-order-index 3))]
      (chain-order-index 9))
(defn none-value? [value]
  (is value None))
(setv setv-expression-x 1)
(setv setv-expression-arg-result (none-value? (setv setv-expression-x 2)))
(setv setv-expression-p (setv setv-expression-q 12))
(setv setv-expression-empty-result (none-value? (setv)))
(setv setv-expression-chain-result
      (none-value? (setv :chain [setv-expression-chain-a
                                 setv-expression-chain-b]
                         3)))
(setv statement-expression-defn-result
      (none-value? (setv statement-expression-defn-value
                         (defn statement-expression-function [] 7))))
(setv statement-expression-defclass-result
      (none-value? (setv statement-expression-defclass-value
                         (defclass StatementExpressionClass))))
(setv statement-expression-for-seen [])
(setv statement-expression-for-result
      (none-value? (setv statement-expression-for-value
                         (for [i (range 3)]
                           (.append statement-expression-for-seen i)))))
(setv statement-expression-assert-result
      (none-value? (setv statement-expression-assert-value
                         (assert True))))
(setv statement-expression-pass-result
      (none-value? (pass)))
(setv statement-expression-del-result
      (none-value? (del)))
(setv slice-score (len (cut [1 2 3 4] 1 3)))
(setv slice-native-values
      [(cut "abcdef")
       (cut "abcdef" 3)
       (cut "abcdef" -2)
       (cut "abcdef" 3 None)
       (cut "abcdef" 3 5)
       (cut "abcdef" 0 None 2)])
(setv slice-target-score [0 1 2 3])
(setv (cut slice-target-score 1 3) [20 22])
(setv slice-delete-score [0 1 2 3])
(del (cut slice-delete-score 1 3))
(setv slice-whole-target [1 2 3])
(setv (cut slice-whole-target) [4 5])
(setv slice-prefix-target [1 2 3])
(setv (cut slice-prefix-target 2) [9])
(setv slice-whole-delete [1 2 3])
(del (cut slice-whole-delete))
(setv contains-score
      (+ (if (in 3 [1 2 3]) 4 0)
         (if (not-in 4 [1 2 3]) 5 0)))
(setv comparison-pending "a")
(setv comparison-edge-values
      [(= 1)
       (< 1)
       (<= 1)
       (> 1)
       (>= 1)
       (is None)
       (= (do (setv comparison-pending "b") "hello"))
       comparison-pending
       (= #* [1])
       (< #* [1])])
(setv chainc-false-seen [])
(setv chainc-true-seen [])
(setv chainc-values
      [(chainc 2 = (+ 1 1) = (- 3 1))
       (chainc 2 = (+ 1 1) = (+ 3 1))
       (chainc 2 = 2 > 1)
       (chainc 1 in [1] in [[1] [2 3]] not-in [5])
       (chainc 1 in [1] not-in [[1] [2 3]] not-in [5])
       (chainc (do (.append chainc-false-seen "a") 1)
               <
               (do (.append chainc-false-seen "b") 0)
               <
               (do (.append chainc-false-seen "c") 3))
       chainc-false-seen
       (chainc (do (.append chainc-true-seen "a") 1)
               <
               (do (.append chainc-true-seen "b") 2)
               <
               (do (.append chainc-true-seen "c") 3))
       chainc-true-seen])
(setv lfor-score (sum (lfor x [1 2 3] (* x 2))))
(setv comp-score
      (+ (len (sfor x [1 1 2] x))
         (sum (.values (dfor x [1 2] x (* x x))))
         (sum (gfor x [1 2 3] x))))
(setv comp-side-effects [])
(setv comp-do-setv-score
      (+ (sum (lfor x [1 2 3]
                    :do (.append comp-side-effects x)
                    :setv y (* x 2)
                    y))
         (len (sfor x [1 1 2]
                    :setv y (+ x 10)
                    y))
         (sum (.values (dfor x [1 2]
                             :setv y (* x x)
                             x y)))
         (sum (gfor x [1 2 3 4]
                    :do (when (= x 4) (break))
                    x))))
(setv comp-unpacked-list
      (lfor xs [[1 2] [3 4] [5]]
            #* xs))
(setv comp-unpacked-set
      (sfor xs [[1 2] [2 3]]
            #* xs))
(setv comp-unpacked-generator
      (list (gfor xs [[1 2] [3 4] [5]]
                   #* xs)))
(setv comp-unpacked-side-effects [])
(setv comp-unpacked-side-effect-generator
      (list (gfor xs [[1 2] [3 4] [5]]
                   :do (.append comp-unpacked-side-effects (len xs))
                   #* xs)))
(defn comp-unpacked-subgenerator []
  (setv received (yield "first"))
  (yield (+ "received: " (str received)))
  (yield "last"))
(setv comp-unpacked-send-generator
      (gfor factory [comp-unpacked-subgenerator]
            #* (factory)))
(setv comp-unpacked-send-values
      [(next comp-unpacked-send-generator)
       (.send comp-unpacked-send-generator "hello")
       (next comp-unpacked-send-generator)])
(setv comp-pending-iter-source "")
(setv comp-pending-iter-values
      (lfor x (do
                (setv comp-pending-iter-source "x")
                "ab")
            y (do
                (+= comp-pending-iter-source "y")
                "def")
            (+ x y comp-pending-iter-source)))
(setv comp-pending-if-source [])
(setv comp-pending-if-values
      (lfor x (range 3)
            :if (do
                  (.append comp-pending-if-source x)
                  (% x 2))
            x))
(setv with-score 0)
(with [value (nullcontext 14)]
  (setv with-score value))
(setv with-expr-score
      (with [value (nullcontext 21)]
        (+ value 21)))
(setv with-outer-effect-y 0)
(setv with-outer-effect-out
      (with [value (nullcontext 40)]
        (setv with-outer-effect-y 2)
        (+ value with-outer-effect-y)))
(defclass FeatureSuppressZDE [object]
  (defn __enter__ [self]
    self)
  (defn __exit__ [self exc-type exc-value traceback]
    (and (is-not exc-type None)
         (issubclass exc-type ZeroDivisionError))))
(setv with-suppress-normal (with [(FeatureSuppressZDE)] 5))
(setv with-suppress-error (with [(FeatureSuppressZDE)] (/ 1 0)))
(setv with-suppress-error-final (with [(FeatureSuppressZDE)] (/ 1 0) 5))
(defn early-score []
  (return 16)
  0)
(defn yield-score []
  (yield 2)
  (yield 3))
(defn yield-from-score []
  (yield 10)
  (yield :from [20 12]))
(defn yield-from-error-score []
  (try
    (yield :from (yield-from-broken))
    (except [ZeroDivisionError]
      (yield 39))))
(defn yield-from-broken []
  (yield 1)
  (yield 2)
  (/ 1 0))
(defn yield-from-keyword-values []
  (yield :from)
  (yield :from))
(defn yield-expression-values []
  (setv received (yield "first"))
  (yield (+ "received: " received))
  (yield "last"))
(defn yield-from-return-values []
  (setv delegated (yield :from (yield-from-return-subgenerator)))
  (yield delegated))
(defn yield-from-return-subgenerator []
  (yield 10)
  (return 32))
(defn :async async-score []
  (+ (await (asyncio.sleep 0 :result 8)) 9))
(setv async-lambda-score
      (asyncio.run ((fn :async [value]
                      (await (asyncio.sleep 0))
                      (+ value 2))
                    40)))
(defn :async async-generator-values []
  (yield 20)
  (yield 22))
(defn :async async-generator-total []
  (setv total 0)
  (for [:async value (async-generator-values)]
    (setv total (+ total value)))
  total)
(defn :async async-anonymous-generator-total []
  (setv total 0)
  (setv values (fn :async []
                 (yield 19)
                 (yield 23)))
  (for [:async value (values)]
    (setv total (+ total value)))
  total)
(setv async-generator-total-value
      (asyncio.run (async-generator-total)))
(setv async-anonymous-generator-total-value
      (asyncio.run (async-anonymous-generator-total)))
(defn :async async-identity [value]
  value)
(defn :async async-try-expr-score []
  (+ (try
       (await (async-identity 10))
       (except [ValueError err]
         0))
     (try
       (raise (ValueError "async"))
       (except [ValueError err]
         (await (async-identity 11))))
     (try
       (raise (ExceptionGroup "async" [(ValueError "star")]))
       (except* [ValueError err]
         (await (async-identity 21))))))
(defn :async async-try-outer-effect-score []
  (setv else-y 0)
  (setv else-out
        (try
          (await (async-identity 40))
          (else
            (setv else-y 2)
            (+ 40 else-y))))
  (setv except-y 0)
  (setv except-out
        (try
          (raise (ValueError "async"))
          (except [ValueError err]
            (setv except-y 42)
            (await (async-identity "ok")))))
  (setv star-y 0)
  (setv star-out
        (try
          (raise (ExceptionGroup "async" [(ValueError "star")]))
          (except* [ValueError err]
            (setv star-y 2)
            (await (async-identity 40)))))
  [else-out else-y except-out except-y star-out star-y])
(defclass AsyncBox []
  (defn __init__ [self value]
    (setv (. self value) value))
  (defn :async __aenter__ [self]
    (. self value))
  (defn :async __aexit__ [self exc-type exc-value traceback]
    False))
(defclass AsyncValues []
  (defn __init__ [self values]
    (setv (. self values) (iter values)))
  (defn __aiter__ [self]
    self)
  (defn :async __anext__ [self]
    (try
      (return (next (. self values)))
      (except [StopIteration]
        (raise StopAsyncIteration)))))
(defn :async async-with-score []
  (setv result 0)
  (with [:async value (AsyncBox 18)]
    (setv result value))
  result)
(defn :async async-with-expr-score []
  (return (with [:async value (AsyncBox 40)]
            (+ value 2))))
(defn :async async-with-outer-effect-score []
  (setv y 0)
  (setv out (with [:async value (AsyncBox 40)]
              (setv y 2)
              (+ value y)))
  [out y])
(defn :async async-for-score []
  (setv total 0)
  (for [:async value (AsyncValues [5 6])]
    (setv total (+ total value)))
  total)
(defn :async async-comp-score []
  (sum (lfor :async value (AsyncValues [5 6]) value)))
(defn :async async-comp-side-effect-score []
  (setv seen [])
  (setv xs (lfor :async value (AsyncValues [1 2 3])
                  :do (.append seen value)
                  :setv doubled (* value 2)
                  doubled))
  (setv ss (sfor :async value (AsyncValues [1 1 2])
                  :setv bumped (+ value 10)
                  bumped))
  (setv dd (dfor :async value (AsyncValues [1 2])
                  :setv squared (* value value)
                  value squared))
  (setv gs [])
  (for [:async value (gfor :async value (AsyncValues [1 2 3 4])
                            :do (when (= value 4) (break))
                            value)]
    (.append gs value))
  (+ (sum xs) (len seen) (len ss) (sum (.values dd)) (sum gs)))
(setv async-with-total (asyncio.run (async-with-score)))
(setv async-with-expr-total (asyncio.run (async-with-expr-score)))
(setv async-with-outer-effect-total
      (asyncio.run (async-with-outer-effect-score)))
(setv async-try-expr-total (asyncio.run (async-try-expr-score)))
(setv async-try-outer-effect-total
      (asyncio.run (async-try-outer-effect-score)))
(setv async-for-total (asyncio.run (async-for-score)))
(setv async-comp-total (asyncio.run (async-comp-score)))
(setv async-comp-side-effect-total (asyncio.run (async-comp-side-effect-score)))
(setv global-score 0)
(defn set-global-score []
  (global global-score)
  (setv global-score 9))
(set-global-score)
(defn nonlocal-score []
  (setv local-score 4)
  (defn inner []
    (nonlocal local-score)
    (setv local-score 10))
  (inner)
  local-score)
(setv module-nonlocal-home "earth")
(defn set-module-nonlocal-home []
  (nonlocal module-nonlocal-home)
  (setv module-nonlocal-home "saturn"))
(set-module-nonlocal-home)
(defn module-nonlocal-health [days intensity]
  (setv health 20
        ration-log
        (list (map (fn [_]
                     (nonlocal module-nonlocal-rations health)
                     (-= module-nonlocal-rations intensity)
                     (+= health (* 0.5 intensity))
                     module-nonlocal-rations)
                   (range days))))
  health)
(setv module-nonlocal-rations 100)
(setv module-nonlocal-health-score (module-nonlocal-health 4 1.5))
(assert (= with-score 14) "sync with score changed")
(assert (= with-expr-score 42) "sync with expression score changed")
(assert (= [with-outer-effect-out with-outer-effect-y] [42 2])
        "sync with expression outer effects changed")
(assert (= [with-suppress-normal with-suppress-error with-suppress-error-final]
           [5 None None])
        "sync with exception suppression changed")
(assert (= annotated-let-score 42) "annotated let score changed")
(assert (= annotated-destructured-let-score 42)
        "annotated destructured let score changed")
(assert (= let-statement-score 42) "statement-body let score changed")
(assert let-match-score "let match capture score changed")
(assert (= let-match-sequence-score [5 6])
        "let sequence match capture score changed")
(assert (= let-rebind-score ["foobarfoobar" "barfoobar"])
        "let sequential rebind score changed")
(assert (= let-early-score "abc") "let early binding score changed")
(assert (= let-unpacking-score [1 2 0 [1 2] 0 [1 2] 0 1 [2]])
        "let starred unpacking score changed")
(assert (= let-unpacking-rebind-score [0 :bar [1 2] 0 [:bar [1 2]]])
        "let starred unpacking rebind score changed")
(assert (= async-with-total 18) "async with score changed")
(assert (= async-with-expr-total 42) "async with expression score changed")
(assert (= async-with-outer-effect-total [42 2])
        "async with expression outer effects changed")
(assert (= async-try-expr-total 42) "async try expression score changed")
(assert (= async-try-outer-effect-total [42 2 "ok" 42 40 2])
        "async try expression outer effects changed")
(assert (= async-for-total 11) "async for score changed")
(assert (= async-comp-total 11) "async comprehension score changed")
(assert (= async-comp-side-effect-total 28)
        "async comprehension :do/:setv score changed")
(assert (= comp-side-effects [1 2 3]) "comprehension :do side effects changed")
(assert (= comp-do-setv-score 25) "comprehension :do/:setv score changed")
(assert (= (+ global-score (nonlocal-score)) 19) "scope declaration score changed")
(assert (= [module-nonlocal-home
            module-nonlocal-health-score
            module-nonlocal-rations]
           ["saturn" 23.0 94.0])
        "module nonlocal promotion changed")
(assert (= loop-else-score 12) "loop else score changed")
(assert (= match-score 42) "match score changed")
(assert (= match-map-score 42) "match mapping score changed")
(assert (= match-guard-score 42) "match guard score changed")
(assert (= match-class-score 42) "match class score changed")
(assert (= match-class-kw-score 42) "match class keyword score changed")
(assert (= match-or-score 42) "match or score changed")
(assert (= match-as-score 42) "match as score changed")
(assert (= native-match-statement-events [1 4 5])
        "native flat match statement guard order changed")
(assert (= native-match-expr-results
           ["zero" None ':as "keyword" 3 "dotted" None])
        "native flat match expression results changed")
(assert (= match-star-score 5) "match star score changed")
(assert (= match-rest-score 42) "match mapping rest score changed")
(assert (= try-star-score 14) "try star score changed")
(assert (= try-expr-score 34) "try expression score changed")
(assert (= (get native-except-values 0) "type-list")
        "native except type-list changed")
(assert (is (get native-except-values 1) TypeError)
        "native except name-first changed")
(assert (= (get native-except-values 2) [KeyError "name-list"])
        "native except name-first type-list changed")
(assert (= (get native-except-values 3) "fallback")
        "native except empty type-list changed")
(assert (= (get native-except-values 4) "type-first")
        "legacy type-first except binding changed")
(assert (= [try-outer-effect-x try-outer-effect-y] ["KL" 1])
        "try expression outer else effects changed")
(assert (= [try-except-outer-effect-out try-except-outer-effect-y] ["ok" 42])
        "try expression outer except effects changed")
(assert (= try-star-expr-score 42) "try star expression score changed")
(assert (= try-star-expr-events ["key" "value" "finally"])
        "try star expression events changed")
(assert (= raise-cause-score 15) "raise cause score changed")
(assert (= operator-score 42) "operator score changed")
(assert (= operator-edge-values
           [0.5
            1.0
            542101086242752217003726400434970855712890625
            542101086242752217003726400434970855712890625
            0
            0
            5
            5
            5
            -48
            "called __pos__"
            0
            1
            0.5])
        "operator edge semantics changed")
(assert (= aug-operator-score 42) "augmented operator score changed")
(assert (= aug-edge-values
           [13
            -5
            96
            65536
            (/ 1 6)
            0
            2048
            0
            0
            7
            6
            6
            "abcd"])
        "augmented assignment edge semantics changed")
(assert (= annotated_score 42) "annotated assignment changed")
(assert (in (get __annotations__ "annotated_score") [int "int"]))
(assert (in (get __annotations__ "bare_annotated") [str "str"]))
(assert (is (get feature_annotation_hints "x") int))
(assert (is (get feature_annotation_hints "z") bool))
(assert (in (get (getattr annotated_fn "__annotations__") "value") [int "int"]))
(assert (in (get (getattr annotated_fn "__annotations__") "return") [int "int"]))
(assert (= (get annotated_complex_hints "values") (get List int)))
(assert (is (get annotated_complex_hints "rest") str))
(assert (is (get annotated_complex_hints "kwargs") bool))
(assert (is (get annotated_complex_hints "return") int))
(assert (= (annotated_lambda 42) 42))
(assert (in (get (getattr annotated_lambda "__annotations__") "value") [int "int"]))
(assert (in (get (getattr annotated_lambda "__annotations__") "return") [int "int"]))
(assert (= (annotated_pos_kw 20 :y 22) 42))
(assert (in (get (getattr annotated_pos_kw "__annotations__") "x") [int "int"]))
(assert (in (get (getattr annotated_pos_kw "__annotations__") "y") [int "int"]))
(assert (in (get (getattr annotated_pos_kw "__annotations__") "return") [int "int"]))
(assert (= (posonly-total 10 :y 32) 42))
(try
  (posonly-total :x 10 :y 32)
  (raise (AssertionError "positional-only argument accepted keyword call"))
  (except [TypeError]))
(assert (= (kwonly-total :required 39) 42))
(try
  (kwonly-total 39)
  (raise (AssertionError "keyword-only argument accepted positional call"))
  (except [TypeError]))
(assert (= (mixed-lambda-list 10 20 30 31 :required 1 :extra 4) 42))
(assert (= (annotated-pair-total [20 22]) 42))
(assert (= (len (getattr annotated-pair-total "__annotations__")) 1))
(assert (= (kwonly-pair-default) 42))
(assert (= (kwonly-pair-required :__hy_meta_arg_0 [20 22]) 42))
(assert (= lambda-list-score 42))
(assert (= (statement-fn 41) 42))
(assert (= (closure-statement-fn 12) 42))
(assert (= __doc__ "Kernel feature proof module.")
        "module docstring changed")
(assert (= [(. feature-doc-fn __doc__)
            (feature-doc-fn)
            (. feature-single-string-fn __doc__)
            (feature-single-string-fn)
            (. FeatureDocClass __doc__)
            (. FeatureDocClass value)]
           ["feature function docstring"
            42
            None
            "feature return string"
            "feature class docstring"
            42])
        "function/class docstring handling changed")
(assert (and (= keyword_values (* [True] 16))
             (= keyword_pickled
                (pickle.loads
                  (pickle.dumps keyword_pickled :protocol pickle.HIGHEST-PROTOCOL))))
        "keyword lowering changed")
(assert (= hy_eval_argument_values (* [True] 21))
        "hy.eval argument behavior changed")
(assert (= call_argument_order_values
           [2 3 #() {"k" 2 "j" 4 "m" 5}
            ["kw" "pos" "star" "kw2" "kwpack"]])
        "call argument order changed")
(assert (= mangling_special_form_alias_values (* [True] 12))
        "mangling special-form alias behavior changed")
(assert (= fstring_values
           ["hello world"
            "hello 2 world"
            "a12b"
            "ab{cde"
            "ab{cde}}fg{{{"
            "ab{2}"
            "aGKz"
            "hxyzzyj"
            "a8z"
            "a'xyzzy'"
            "axyzzy    "
            "a'xyzzy'  "
            "   2"
            "result:      12.34"
            "fstring_foo ='bar'"
            "xyz  fstring_foo = 'bar'"
            " fstring_foo = bar"
            "a'xyzzy'  "
            "result:      12.34"
            "{escaped braces} \\n not escaped"
            "\"0\""
            "C[  '2'xx]"
            "fstring_pi = __3.14__"
            "   2"
            4
            ["value" "spec"]])
        "f-string lowering changed")
(assert (= [(. quoted_bracket brackets)
            quoted_fstring_missing
            (isinstance quoted_fstring hy.models.FString)
            (getattr quoted_fstring_component "expression")
            quoted_fstring_value
            quoted_fstring_repr_roundtrip]
           ["feature"
            True
            True
            "quoted_world"
            "quote ready"
            [[True [None None None] True]
             [True [None "r"] True]
             [True [None "s"] True]]])
        "quoted string model metadata changed")
(assert (= [quasiquote_falsey_splice
            (len quasiquote_single_eval_splice)
            (str (get quasiquote_single_eval_splice 0))
            (get quasiquote_single_eval_splice 1)
            (get quasiquote_single_eval_splice 2)
            (str (get quasiquote_single_eval_splice 3))
            quasiquote_splice_side_effects
            quasiquote_nested
            (get quasiquote_nested 1)
            quasiquote_nested_struct
            quasiquote_triple_eval]
           ['(a b c d e f c d e g h)
            4
            "x"
            1
            2
            "y"
            ["once"]
            '(1 `~(+ 1 5) 4)
            '`~(+ 1 5)
            '(try
               (setv x1 (+ "x" (str 1)))
               (setv x2 (+ "x" (str 2)))
               (setv x3 (+ "x" (str 3)))
               (finally
                 (print "done")))
            '[3 2 1]])
        "quasiquote splice lowering changed")
(assert (= async-lambda-score 42) "async anonymous function changed")
(assert (= async-generator-total-value 42) "async generator function changed")
(assert (= async-anonymous-generator-total-value 42)
        "async anonymous generator function changed")
(assert (= import-star-score 42) "bare star import changed")
(assert (= dot-chain-score 42) "dot chain lowering changed")
(assert (= dot-method-shortcut-values
           ["method"
            "method foo bar"
            "method foo bar 2 1"
            "method foo bar"
            True
            "aabb"])
        "method shortcut dot lowering changed")
(assert (= (dotted_root_function) "ok")
        "dotted statement root lowering changed")
(assert (= multi-subscript-values [6 12 [[1 2] [9 4]]])
        "multi-index get lowering changed")
(assert (= [do-effect-x do-effect-y do-effect-when] ["b" "c" 42])
        "do statement expression lowering changed")
(assert (= ellipsis-score 42) "ellipsis constant lowering changed")
(assert (= unpacked-list [1 2 3 4]))
(assert (= unpacked-tuple #("a" "b" "c")))
(assert (= unpacked-set #{1 2 3}))
(assert (= unpacked-dict {"a" 1 "b" 2 "c" 3}))
(assert (= [collection-pending-list
            collection-pending-tuple
            (= collection-pending-set #{2})
            collection-pending-dict-key
            collection-pending-dict-value
            collection-pending-list-unpack
            collection-pending-dict-unpack]
           [[2 2]
            #(2 2)
            True
            {2 2}
            {2 2}
            [2 2 3 2]
            {2 30}])
        "collection pending evaluation order changed")
(assert (= operator-unpack-score 116)
        "operator iterable unpacking changed")
(assert (= [unpack-head unpack-tail] [10 [20 12]]))
(assert (= dfor-unpacked {"a" 10 "b" 32}))
(assert (= setx-score 42) "setx score changed")
(assert (= [setx_chain_x
            setx_chain_y
            setx_filter_values
            setx_filter_kept
            (setx-helper-existing)
            (setx-helper-new)
            setx-empty-error]
           ["ab"
            "abc"
            ["apple" "banana"]
            "banana"
            9
            9
            "UnboundLocalError"])
        "setx scope semantics changed")
(assert (= slice-target-score [0 20 22 3]) "slice assignment changed")
(assert (= slice-delete-score [0 3]) "slice deletion changed")
(assert (= slice-native-values
           ["abcdef" "abc" "abcd" "def" "de" "ace"])
        "native cut expression semantics changed")
(assert (= [slice-whole-target slice-prefix-target slice-whole-delete]
           [[4 5] [9 3] []])
        "native cut target semantics changed")
(assert (= comparison-edge-values
           [True True True True True True True "b" True True])
        "comparison edge semantics changed")
(assert (= chainc-values
           [True
            False
            True
            True
            False
            False
            ["a" "b"]
            True
            ["a" "b" "c"]])
        "chainc semantics changed")
(assert (= comp-unpacked-list [1 2 3 4 5])
        "lfor unpacked final value changed")
(assert (= comp-unpacked-set #{1 2 3})
        "sfor unpacked final value changed")
(assert (= comp-unpacked-generator [1 2 3 4 5])
        "gfor unpacked final value changed")
(assert (= comp-unpacked-side-effect-generator [1 2 3 4 5])
        "gfor unpacked side-effect final value changed")
(assert (= comp-unpacked-side-effects [2 2 1])
        "gfor unpacked side-effect ordering changed")
(assert (= comp-unpacked-send-values ["first" "received: None" "last"])
        "gfor unpacked generator protocol changed")
(assert (= [comp-pending-iter-values comp-pending-iter-source]
           [["adxy" "aexy" "afxy" "bdxyy" "bexyy" "bfxyy"] "xyy"])
        "comprehension iterable pending placement changed")
(assert (= [comp-pending-if-values comp-pending-if-source]
           [[1] [0 1 2]])
        "comprehension if pending placement changed")
(assert (= [for-pending-iter-values for-pending-iter-source]
           [["ady" "aey" "bdyy" "beyy"] "yy"])
        "for iterable pending placement changed")
(assert (= (list (yield-from-score)) [10 20 12])
        "yield from delegation changed")
(assert (= (sum (yield-from-error-score)) 42)
        "yield from exception propagation changed")
(assert (= (list (yield-from-keyword-values)) [:from :from])
        "single-argument yield keyword changed")
(setv yield-expression-generator (yield-expression-values))
(assert (= [(next yield-expression-generator)
            (.send yield-expression-generator "hello")
            (next yield-expression-generator)]
           ["first" "received: hello" "last"])
        "yield expression send protocol changed")
(assert (= (list (yield-from-return-values)) [10 32])
        "yield from return value propagation changed")
(assert (= while-do-score "ababaz")
        "while do-condition else semantics changed")
(assert (= while-do-continue-score "ababcabaz")
        "while do-condition continue semantics changed")
(assert (= while-condition-break-score "1yyy2y3yyy")
        "while do-condition break semantics changed")
(assert (= [(break-loop-value) continue-loop-values] [5 [5]])
        "break/continue loop semantics changed")
(assert (= [pending-setv-a pending-setv-b pending-setv-order] [2 3 [1]])
        "pending setv ordering changed")
(assert (= [chain-a chain-b chain-c] [3 3 3])
        "setv chain simple assignment changed")
(assert (= [chain-v1 chain-v2 chain-v3 chain-v4 chain-v5 chain-v6 chain-v7]
           [1 2 2 4 5 5 5])
        "setv chain mixed assignment changed")
(assert (= [chain-y chain-z chain-w chain-x chain-aa chain-bb chain-cc chain-dd]
           ["a" ["b" "c"] "d" "abcd" "a" "b" "c" "d"])
        "setv chain destructuring changed")
(assert (= [chain-order-calls chain-order-list]
           [[[9 [0 0 0 0 0]]
             [1 [0 0 0 0 0]]
             [2 [0 9 0 0 0]]
             [3 [0 9 9 0 0]]]
            [0 9 9 9 0]])
        "setv chain evaluation order changed")
(assert (= [setv-expression-arg-result setv-expression-x]
           [True 2])
        "setv expression argument result changed")
(assert (= [setv-expression-p setv-expression-q]
           [None 12])
        "nested setv expression result changed")
(assert setv-expression-empty-result
        "empty setv expression result changed")
(assert (= [setv-expression-chain-result
            setv-expression-chain-a
            setv-expression-chain-b]
           [True 3 3])
        "setv chain expression result changed")
(assert (= [statement-expression-defn-result
            statement-expression-defn-value
            (statement-expression-function)]
           [True None 7])
        "defn expression none result changed")
(assert (= [statement-expression-defclass-result
            statement-expression-defclass-value
            (. StatementExpressionClass __name__)]
           [True None "StatementExpressionClass"])
        "defclass expression none result changed")
(assert (= [statement-expression-for-result
            statement-expression-for-value
            statement-expression-for-seen]
           [True None [0 1 2]])
        "for expression none result changed")
(assert (= [statement-expression-assert-result
            statement-expression-assert-value
            statement-expression-pass-result
            statement-expression-del-result]
           [True None True True])
        "statement none expression result changed")
(assert (= conditional-expression-if [2 0])
        "if expression statement branch changed")
(assert (= conditional-expression-when [None 0])
        "when expression statement branch changed")
(assert (= conditional-expression-cond [None 1])
        "cond expression statement branch changed")
(pass)

(+ (add10 (inc 31))
   root
   (answer)
   (add-all [1 2 3])
   local-sum
   destructured-let
   (optional-bonus)
   (rest-count 3 4 5)
   (kw-bonus :x 4 :y 5 :z 6)
   (kw-spread #** packed)
   (+ pair-a pair-b)
   (pair-total [6 7])
   control-score
   when-score
   statement-score
   for-score
   try-score
   (. FeatureBox score)
   (decorated-score)
   (. DecoratedBox bonus)
   (. AssignedBox points)
   import-score
   subscript-score
   aug-score
   del-score
   bool-score
   slice-score
   contains-score
   lfor-score
   comp-score
   with-score
   (early-score)
   (sum (yield-score))
   (asyncio.run (async-score))
   async-with-total)
