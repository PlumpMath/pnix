#!/usr/bin/env node
// 최소 host-main import: pnix-cljs 라이브러리 require + evalFileValueJson.
// HM `node` 래퍼의 NODE_PATH, 또는:
//   export NODE_PATH="$(pnix-cljs-library | sed -n 's/^PNIX_CLJS_SHARE=//p'):$NODE_PATH"
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import path from "node:path";

const require = createRequire(import.meta.url);
const here = path.dirname(fileURLToPath(import.meta.url));
const px = path.join(here, "..", "hello.px");

function loadPnix() {
  try {
    return require("@plumpmath/pnix-cljs");
  } catch {
    return require("pnix-cljs-module.js");
  }
}

const pnix = loadPnix();
const out = pnix.evalFileValueJson(px);
process.stdout.write(String(out) + "\n");
