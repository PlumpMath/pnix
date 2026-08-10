#!/usr/bin/env node
"use strict";

const fs = require("node:fs");
const path = require("node:path");
const util = require("node:util");

const [compilerPath, root, outputPath] = process.argv.slice(2);
if (!compilerPath || !root || !outputPath) {
  console.error("usage: fixed-point-stage COMPILER ROOT OUTPUT");
  process.exit(2);
}

const compiler = require(path.resolve(compilerPath));

compiler.compileCompiler(path.resolve(root)).then((result) => {
  fs.writeFileSync(outputPath, JSON.stringify(result));
}).catch((error) => {
  let current = error;
  let depth = 0;
  while (current && depth < 12) {
    console.error(`compiler error depth ${depth}:`, current.stack || current);
    if (current.data) {
      console.error("compiler ex-data:", util.inspect(current.data, {depth: 12}));
    }
    current = current.cause;
    depth += 1;
  }
  process.exitCode = 1;
});
