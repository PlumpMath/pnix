// 관측용 JSON 투영 — 타입 권위가 아님.
const path = require("path");
const pnix = require(path.join(__dirname, "..", "..", "dist", "pnix-cljs-module.js"));

const src = "{ a = 1; b = [ true false ]; }";
console.log("evalSourceJson:", pnix.evalSourceJson(src));
console.log("evalValueJson:", pnix.evalValueJson(src));
