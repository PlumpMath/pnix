#!/usr/bin/env node
"use strict";

const fs = require("node:fs");
const meta = require("./cljs-meta-fixed.js");

const args = process.argv.slice(2);
let source = null;

if (args.length === 2 && (args[0] === "-e" || args[0] === "--eval")) {
  source = args[1];
} else if (args.length === 1) {
  source = fs.readFileSync(args[0], "utf8");
}

if (source === null) {
  console.error("usage: cljs-meta-fixed -e EXPR | cljs-meta-fixed FILE");
  process.exitCode = 2;
} else {
  meta.evaluate(source).then((result) => {
    console.log(JSON.stringify(result));
    if (result.outcome_kind !== "done") process.exitCode = 1;
  }).catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}
