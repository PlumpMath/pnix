const cljsMeta = require("../dist/cljs-meta-module.js");

cljsMeta.evaluate("(let [x 20] (+ x 22))").then(console.log);
