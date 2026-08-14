// 파일 경로로 게스트 평가 (evalFile 계열).
const path = require("path");
const pnix = require(path.join(__dirname, "..", "..", "dist", "pnix-cljs-module.js"));

const px = path.join(__dirname, "sample.px");
console.log("파일:", px);
console.log("evalFile:", pnix.evalFile(px));
console.log("evalFileValueJson:", pnix.evalFileValueJson(px));
