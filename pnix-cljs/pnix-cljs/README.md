# pnix-cljs runtime

This package is the active ClojureScript implementation of the PNIX seed
runtime. It parses PNIX/Nix-superset source directly and returns nominal
`Done` or `Failed` values.

```clojure
(require '[pnix-cljs.core :as pnix]
         '[pnix-cljs.outcome :as outcome])

(outcome/project (pnix/eval-source "20 + 22"))
```

JavaScript callers use `dist/pnix-cljs-module.js`:

```js
const pnix = require("./dist/pnix-cljs-module.js");
pnix.evalSource("let x = 20; in x + 22");
```

The semantic payload remains a native ClojureScript value. The JSON-facing
projection is observation evidence, not the language value and not a type
authority.
