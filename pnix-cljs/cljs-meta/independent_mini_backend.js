"use strict";
/**
 * Tiny independent ClojureScript-subset-to-JS emitter.
 *
 * Trusting-Trust (Diverse Double-Compiling) witness: a hand-written
 * tokenizer/reader plus direct JS source-text emission, sharing zero code
 * with the official cljs.js/cljs.compiler/cljs.analyzer payload this repo's
 * self-hosted `dist/cljs-meta-module.js` is built from. `new Function(...)`
 * and the JS engine itself remain trusted host substrate here, the same
 * honest role the JVM classfile format plays for clj-meta's analogous
 * `frontend_selfhost.clj` and Python's `ast`/`compile()` play for hy-meta's
 * `independent_mini_backend.py`.
 *
 * It is a frontier witness, not a replacement for the production frontend:
 * it covers a bounded fixture set (arithmetic, comparisons, `if`, `let`,
 * booleans/keywords), not ClojureScript.
 */

const TOKEN_RE = /\s*(\(|\)|\[|\]|:[^\s()\[\]]+|-?\d+|[^\s()\[\]]+)/y;

function tokenize(source) {
  const tokens = [];
  TOKEN_RE.lastIndex = 0;
  let pos = 0;
  while (pos < source.length) {
    TOKEN_RE.lastIndex = pos;
    const m = TOKEN_RE.exec(source);
    if (!m || m.index !== pos) {
      if (source.slice(pos).trim() === "") break;
      throw new SyntaxError(`tiny reader: unexpected input at ${pos}`);
    }
    pos = TOKEN_RE.lastIndex;
    if (m[1] !== undefined) tokens.push(m[1]);
  }
  return tokens;
}

function parseOne(tokens, i) {
  const tok = tokens[i];
  if (tok === "(") {
    const items = [];
    i += 1;
    while (tokens[i] !== ")") {
      if (i >= tokens.length) throw new SyntaxError("tiny reader: missing )");
      const [item, next] = parseOne(tokens, i);
      items.push(item);
      i = next;
    }
    return [{ kind: "list", items }, i + 1];
  }
  if (tok === "[") {
    const items = [];
    i += 1;
    while (tokens[i] !== "]") {
      if (i >= tokens.length) throw new SyntaxError("tiny reader: missing ]");
      const [item, next] = parseOne(tokens, i);
      items.push(item);
      i = next;
    }
    return [{ kind: "vector", items }, i + 1];
  }
  if (tok === ")" || tok === "]") {
    throw new SyntaxError(`tiny reader: unexpected closing delimiter ${tok}`);
  }
  if (tok === "true") return [{ kind: "bool", value: true }, i + 1];
  if (tok === "false") return [{ kind: "bool", value: false }, i + 1];
  if (tok === "nil") return [{ kind: "nil" }, i + 1];
  if (tok.startsWith(":") && tok.length > 1) {
    return [{ kind: "keyword", name: tok.slice(1) }, i + 1];
  }
  if (/^-?\d+$/.test(tok)) return [{ kind: "num", value: tok }, i + 1];
  return [{ kind: "sym", name: tok }, i + 1];
}

function tinyRead(source) {
  const tokens = tokenize(source);
  const [form, next] = parseOne(tokens, 0);
  if (next !== tokens.length) {
    throw new SyntaxError("tiny reader: trailing tokens");
  }
  return form;
}

const BINOPS = { "+": "+", "-": "-", "*": "*" };
const CMPOPS = { "<": "<", ">": ">", "<=": "<=", ">=": ">=", "=": "===" };

let jsIdCounter = 0;
function freshName(prefix) {
  jsIdCounter += 1;
  return `${prefix}_${jsIdCounter}`;
}

function emitExpr(form, env) {
  switch (form.kind) {
    case "num":
      return form.value;
    case "bool":
      return form.value ? "true" : "false";
    case "nil":
      return "null";
    case "keyword":
      return JSON.stringify(form.name);
    case "sym": {
      if (!env.has(form.name)) {
        throw new SyntaxError(`tiny analyzer: unknown local ${form.name}`);
      }
      return env.get(form.name);
    }
    case "list": {
      const [head, ...args] = form.items;
      if (head.kind !== "sym") {
        throw new SyntaxError("tiny analyzer: call head must be a symbol");
      }
      if (head.name === "if") {
        if (args.length !== 3) throw new SyntaxError("tiny analyzer: if arity");
        const [test, then, els] = args;
        return `(${emitExpr(test, env)} !== false && ${emitExpr(test, env)} !== null ? ${emitExpr(then, env)} : ${emitExpr(els, env)})`;
      }
      if (head.name === "let") {
        if (args.length < 2) throw new SyntaxError("tiny analyzer: let arity");
        const [bindingsForm, ...body] = args;
        if (bindingsForm.kind !== "vector" || bindingsForm.items.length % 2 !== 0) {
          throw new SyntaxError("tiny analyzer: malformed let bindings");
        }
        const innerEnv = new Map(env);
        const decls = [];
        for (let i = 0; i < bindingsForm.items.length; i += 2) {
          const nameForm = bindingsForm.items[i];
          const initForm = bindingsForm.items[i + 1];
          if (nameForm.kind !== "sym") {
            throw new SyntaxError("tiny analyzer: let binding name");
          }
          const jsName = freshName(nameForm.name);
          const initCode = emitExpr(initForm, innerEnv);
          decls.push(`let ${jsName} = ${initCode};`);
          innerEnv.set(nameForm.name, jsName);
        }
        const bodyCode = body.map((f) => emitExpr(f, innerEnv));
        const returnCode = bodyCode[bodyCode.length - 1];
        return `(function(){ ${decls.join(" ")} return (${returnCode}); })()`;
      }
      if (Object.prototype.hasOwnProperty.call(BINOPS, head.name)) {
        if (args.length !== 2) throw new SyntaxError("tiny analyzer: binary op arity");
        return `(${emitExpr(args[0], env)} ${BINOPS[head.name]} ${emitExpr(args[1], env)})`;
      }
      if (Object.prototype.hasOwnProperty.call(CMPOPS, head.name)) {
        if (args.length !== 2) throw new SyntaxError("tiny analyzer: compare op arity");
        return `(${emitExpr(args[0], env)} ${CMPOPS[head.name]} ${emitExpr(args[1], env)})`;
      }
      throw new SyntaxError(`tiny analyzer: unsupported call head ${head.name}`);
    }
    default:
      throw new SyntaxError(`tiny analyzer: unsupported form kind ${form.kind}`);
  }
}

function compileAndEval(source) {
  const form = tinyRead(source);
  const code = emitExpr(form, new Map());
  // eslint-disable-next-line no-new-func
  const fn = new Function(`"use strict"; return (${code});`);
  return fn();
}

module.exports = { tinyRead, compileAndEval };
