const fs = require("fs");
const path = require("path");
const pnix = require(path.join(__dirname, "..", "..", "dist", "pnix-cljs-module.js"));

const px = path.join(__dirname, "snippet.px");
const source = fs.readFileSync(px, "utf8");
console.log("host-main 평가:", source.trim());
console.log(pnix.evalSource(source));
