// pnix-cljs guest eval — structured outcomes, not ambient Node eval.
const path = require("path");
const pnix = require(path.join(__dirname, "..", "..", "dist", "pnix-cljs-module.js"));

const pure = pnix.evalSource("1 + 2 * 3");
console.log("pure:", pure);

const div0 = pnix.evalSource("1 / 0");
console.log("div0:", div0);

// Guest source cannot reach Node require/fs through the evaluator API.
// (No claim of a full effect-system gate like hy/rs — only the admitted seed.)
