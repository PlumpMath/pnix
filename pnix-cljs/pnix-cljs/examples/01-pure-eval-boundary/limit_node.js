// plain Node — no guest language boundary
// Node can always reach process, fs, and the ambient global.
// Untrusted strings must not be handed to eval/Function.

console.log("plain Node has no built-in pure guest sandbox for arbitrary code");
console.log("eval / new Function can touch process, require, and globals");
// Not executed:
//   eval("require('fs').readFileSync('/etc/passwd','utf8')")
//   new Function("return process.env")()
console.log("conclusion: use an explicit guest evaluator, not host eval");
