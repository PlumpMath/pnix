(ns pnix-clj.math)

(def lane-classification
  {:lane :core
   :scope :numeric-runtime-helper
   :role :nix-compatible-basic-math-helper
   :product-runtime :allowed
   :semantic-authority :helper-only
   :mutation :forbidden
   :admission :forbidden
   :determinism :required
   :allowed-output :numeric-result})

(defn div
  [left right]
  (if (and (integer? left) (integer? right))
    (quot left right)
    (/ left right)))
