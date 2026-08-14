const path = require("path");
const pnix = require(path.join(__dirname, "..", "..", "dist", "pnix-cljs-module.js"));

for (const src of ["20 + 22", "1 / 0", "missing_var"]) {
  const r = pnix.evalSource(src);
  console.log(JSON.stringify({ source: src, result: r }));
}
