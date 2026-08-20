# pnix-clj capabilities (generated — do not edit)

Regenerate with `clojure -M:capabilities`; the gate runs
`clojure -M:capabilities-check` and fails on drift. Use this index to
check whether something already exists before implementing it.

## Lanes (receipt/lane-order)

- pnix-clj-evaluator
- pnix-clj-lowering-clj-meta
- clojure-stage15-mirror
- px-runtime-pnix-mirror

## Report artifacts (report-artifact/supported-kinds)

- arith-proof
- bool-proof
- cached-eval
- cas
- cegis
- classfile-receipt
- clojure-form
- clojure-projection
- coverage
- determinism
- emit-form-roundtrip
- form-analysis
- forward-reference
- futamura
- generate
- grammar-fuzzer
- live-oracle
- machine
- mirror-chain
- mirror-error
- mirror-pair
- persist
- property-fuzzer
- purity
- reflect
- replay
- rust-batch
- safe-eval
- search
- self-improve
- self-mod-gate
- smoke
- snapshot
- specialize
- stage15-exec
- stage7-core
- store
- strict-audit
- synthesize
- tower
- translation-validation
- trust
- value-roundtrip
- weval
- witness
- witnessed-run

## Deps aliases

  arith-proof benchmark bool-proof cached-eval capabilities capabilities-check
  cas cegis classfile-receipt clojure-form clojure-projection coverage
  determinism emit-form-roundtrip form-analysis forward-reference futamura generate
  grammar-fuzzer io-probe lane-registry lane-registry-check live-oracle machine
  machine-outcome-check mirror-chain mirror-error mirror-pair nrepl nrepl-pnix
  persist property-fuzzer purity reflect repl-pnix repl-pnix-server
  replay report-arith-proof report-batch report-bool-proof report-cached-eval report-cas
  report-cegis report-classfile-receipt report-clojure-form report-clojure-projection report-coverage report-determinism
  report-emit-form-roundtrip report-form-analysis report-forward-reference report-futamura report-generate report-grammar-fuzzer
  report-live-oracle report-machine report-mirror-chain report-mirror-error report-mirror-pair report-persist
  report-property-fuzzer report-purity report-reflect report-replay report-rust-batch report-safe-eval
  report-search report-self-improve report-self-mod-gate report-smoke report-snapshot report-specialize
  report-stage15-exec report-stage7-core report-store report-strict-audit report-synthesize report-tower
  report-translation-validation report-trust report-value-roundtrip report-weval report-witness report-witnessed-run
  runtime-plan rust-batch safe-eval search self-improve self-mod-gate
  smoke snapshot specialize stage15-exec stage15-plan stage7-core
  store strict-audit strict-gate synthesize test tower
  translation-validation trust value-roundtrip weval wiki wiki-check
  witness witnessed-run

## Builtins (196)

  abort abs add addErrorContext all and
  any append appendContext assertMsg atan2 attrByPath
  attrNames attrValues baseNameOf bitAnd bitOr bitXor
  boolToString break builtins catAttrs ceil compareVersions
  concatLists concatMap concatMapStrings concatMapStringsSep concatStrings concatStringsSep
  cons const cos count currentSystem deepSeq
  derivation derivationStrict dirOf div drop elem
  elemAt eq exp false fetchGit fetchTarball
  fetchurl filter filterAttrs filterAttrsRecursive find findFirst
  fix flatten flip floor foldl foldl'
  foldlAttrs foldr fromJSON functionArgs ge genAttrs
  genList genericClosure get getAttr getAttrFromPath getAttrFromPathOr
  getContext getEnv getName getVersion groupBy gt
  hasAttr hasAttrByPath hasContext hasInfix hasPrefix hasSuffix
  hashString head id imap0 imap1 implies
  init intersectAttrs intersectLists isAttrs isBool isFloat
  isFunction isInt isList isNull isPath isString
  keys langVersion last le length lessThan
  listToAttrs ln log lt map mapAttrs
  mapAttrs' mapAttrsRecursive mapAttrsToList match max merge
  min mod mul nameValuePair neg nixVersion
  not null optional optionalAttrs optionalString optionals
  or parseDrvName partition pathExists pipe placeholder
  pnixMounts pow product range readDir readFile
  recursiveUpdate removeAttrs removePrefix removeSuffix replaceStrings replicate
  reverseList seq set sin sort split
  splitString splitVersion sqrt storeDir storePath stringLength
  stringToCharacters sub substring subtractLists sum tail
  take tan throw toFile toInt toJSON
  toLower toPath toString toUpper toXML trace
  true tryEval typeOf unique unsafeDiscardOutputDependency unsafeDiscardStringContext
  unsafeGetAttrPos updateManyAttrs values warn when zip
  zipAttrs zipAttrsWith zipLists zipListsWith

## Unprefixed default scope

  abort baseNameOf dirOf import isNull lib map removeAttrs
  scopedImport throw toString

## Public API

### pnix-clj.arith-proof

  -main equivalent? lane-classification poly-of poly-of-source
  poly-substitute proof-cases prove-specialize-meaning report

### pnix-clj.bool-proof

  -main free-vars lane-classification proof-cases prove-equivalent
  refute-cases report

### pnix-clj.cached-eval

  -main cache-epoch cache-key cached-eval clear-eval-cache!
  eval-cache-stats lane-classification report

### pnix-clj.cas

  -main alpha-equivalent? canonical-form clear-store! empty-store
  get-term has-term? lane-classification pure-term? put-term!
  report store-cases structurally-equivalent? term-count term-hash

### pnix-clj.cegis

  -main cegis-and-propose cegis-synthesize counterexample default-probes
  lane-classification report wide-probes

### pnix-clj.core

  compile-source eval-file eval-source eval-source-strict eval-source-strict-audit
  eval-source-with-imports lane-classification lower-source parse-source report
  run-source verify-source

### pnix-clj.evaluator

  *coverage* *fuel* *import-context* *import-modules* *import-origin*
  *import-resolver* *pure-eval* *strict* *strict-audit* ->PxBytes
  apply-callable assert-condition-violation attr-key-value-result attrset-value? binary-value-result
  contextual-import-target ctx-string ctx-string? default-env default-env-names
  eval-ast eval-ast* eval-ast-whnf eval-ast-with-fuel force-normal
  force-whnf if-condition-violation import-context-key import-resolver-context impure-builtins
  interpolation-value-result lane-classification lazy-host-fn logical-operand-result make-value-thunk
  merge-attr-path neg-value-result nix-float-str nix-regex-pattern not-value-result
  nullary-builtin-result path-value path-value? source-position strict-type
  string-content string-ctx value-thunk?

### pnix-clj.form-analysis

  -main analysis-cases analyze-form lane-classification report

### pnix-clj.futamura

  -main cogen cogen-note fourth-projection-collapse generating-extension
  jones-optimality-witness lane-classification projection-cases report run-projection-case

### pnix-clj.generate

  -main default-literals default-ops lane-classification report
  synthesize synthesize-and-propose value-vector

### pnix-clj.interop

  *capabilities* ->EffectExecuted ->EffectFailed apply-effect-request attach-witness
  check-capability crossing-witness default-capabilities effect-class? effect-classes
  fresh-host-ns from-host host-compile-capabilities host-eval-capabilities host-eval-form
  host-object? interop-meta lane-classification make-opaque-host-ref make-witness
  opaque-host-ref? opaque-ref-deref read-only-effect-names release-opaque-ref! run-crossing
  to-host witness?

### pnix-clj.lowering

  *force-on-read-vars* *import-context* *import-modules* *lexical-renames* *lexical-vars*
  *with-depth* *with-scope-syms* abort-value append-context assert-function
  attr-by-path attr-key-string attrset-pairs attrset-value? base-name-of
  builtins-attrset clear-lower-cache! coerce-to-string dir-of discard-string-context
  eval-value->lowered find-value force-normal force-slot function-args
  generic-closure get-context has-context has-infix? has-prefix?
  has-suffix? host-builtin inclusive-range interpolate-to-string lane-classification
  lazy-slot list-head list-init list-last list-length
  list-tail list-to-attrs lookup-with-scopes lower-ast lower-cache-key
  lower-cache-stats lower-case lowered-value->eval lowering-policy nix-binary
  nix-equal nix-neg nix-order path-value? pattern-actual
  pattern-guard plus recursive-update regex-match regex-split
  remove-prefix remove-suffix replace-strings require-bool split-string
  string-to-characters substring template-join throw-value to-int
  type-of unique-list upper-case

### pnix-clj.mirror

  clojure-mirror-row cross-mirror-verdict-row default-facets lane-classification pnix-mirror-row
  px-runtime-row run-mirror

### pnix-clj.mirror-chain

  -main converge? lane-classification mirror-chain! report
  run-result-hash

### pnix-clj.parser

  *allow-call* clear-parse-cache! lane-classification parse-cache-key parse-cache-stats
  parse-source

### pnix-clj.persist

  -main lane-classification load-events load-source load-term
  load-witness open-persistent-store persist-events! persist-source! persist-term!
  persist-witness! report

### pnix-clj.property-fuzzer

  -main cache-preserves-meaning? cache-property cross-lane-property gen-typed-expr
  lane-classification lanes-collapse? machine-agrees? machine-property report
  specialize-preserves-meaning? specializer-property specializer-proven-property

### pnix-clj.purity

  -main lane-classification mutation-isolation! purity-check! report
  threaded-stress

### pnix-clj.receipt

  lane-classification lane-order lane-summary summarize verdict

### pnix-clj.reflect

  -main all-ns-snapshot classpath-snapshot host-lane-id jvm-version-id
  lane-classification namespace-diff ns-publics-snapshot reflection-snapshot report
  var-snapshot

### pnix-clj.replay

  -main lane-classification replay-witness report

### pnix-clj.safe-eval

  -main cases default-fuel lane-classification report
  safe-eval static-purity-check

### pnix-clj.search

  -main free-vars lane-classification open-term-summary report
  search-events similar-terms skeleton structural-distance

### pnix-clj.self-improve

  -main evaluate-round lane-classification report

### pnix-clj.self-mod-gate

  -main decide lane-classification policies propose!
  propose-and-gate report

### pnix-clj.snapshot

  -main assert-snapshot-runtime-match! evaluator-version lane-classification make-snapshot
  report resolve-under-snapshot runtime-matches? symbol-version

### pnix-clj.specialize

  *fold-fuel* -main cases clear-specialize-cache! default-fold-fuel
  futamura-cases host-artifact-cases invoke-host-artifact lane-classification report
  specialize specialize-cache-epoch specialize-cache-stats specialize-cached specialize-to-host
  specialize-to-host-artifact

### pnix-clj.store

  -main append! by-field by-hash events
  events-of get-pointer head-hash lane-classification open-store
  report set-pointer! verify-chain

### pnix-clj.synthesize

  -main cases form->pnix held-cases lane-classification
  report

### pnix-clj.tower

  -main lane-classification report run-tower

### pnix-clj.unparse

  lane-classification strip-positions unparse

### pnix-clj.wiki

  -main capability-registry check generate! integrity
  lane-classification render roadmap

### pnix-clj.witness

  -main admit lane-classification make-witness report
  status-transition statuses valid-transition? witness-eval witness-fields

### pnix-clj.witnessed-run

  -main lane-classification report residual-key run-witnessed
  run-witnessed-durable
