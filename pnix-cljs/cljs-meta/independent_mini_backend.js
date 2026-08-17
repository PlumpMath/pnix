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
 * it covers a bounded fixture set (arithmetic, comparisons, `if`, `let`
 * (including nested vector destructuring), `do`, `loop`/`recur`, named `fn`
 * literals/recursion, genuine closures (a `let`-bound `fn` called more than
 * once and/or from a non-tail position -- this falls out for free from
 * ordinary JS function-expression closure semantics, no separate closure
 * value representation needed), strings, vector/map/set literals,
 * booleans/keywords, the seq ops `get`/`nth`/`count`/`conj`/`nil?`,
 * `assoc`/`update` on maps, and the `when`/`cond`/`->` macros), not
 * ClojureScript. The host side of this DDC pair
 * (`cljs.js` via `core/evaluate`) runs `eval-str` with `:context :expr`,
 * which only accepts a single top-level expression, so this backend does
 * not implement `defn` or other multi-form/top-level-definition source —
 * recursion is expressed the same way both backends can agree on it: a
 * self-referencing named `fn` literal invoked in place, e.g.
 * `((fn fact [n] (if (<= n 1) 1 (* n (fact (- n 1))))) 6)`.
 */

const TOKEN_RE = /\s*("(?:[^"\\]|\\.)*"|#\{|\(|\)|\[|\]|\{|\}|:[^\s()\[\]{}]+|-?\d+|[^\s()\[\]{}]+)/y;

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
  if (tok === "{") {
    const items = [];
    i += 1;
    while (tokens[i] !== "}") {
      if (i >= tokens.length) throw new SyntaxError("tiny reader: missing }");
      const [item, next] = parseOne(tokens, i);
      items.push(item);
      i = next;
    }
    if (items.length % 2 !== 0) {
      throw new SyntaxError("tiny reader: malformed map literal");
    }
    return [{ kind: "map", items }, i + 1];
  }
  if (tok === "#{") {
    const items = [];
    i += 1;
    while (tokens[i] !== "}") {
      if (i >= tokens.length) throw new SyntaxError("tiny reader: missing }");
      const [item, next] = parseOne(tokens, i);
      items.push(item);
      i = next;
    }
    return [{ kind: "set", items }, i + 1];
  }
  if (tok === ")" || tok === "]" || tok === "}") {
    throw new SyntaxError(`tiny reader: unexpected closing delimiter ${tok}`);
  }
  if (tok === "true") return [{ kind: "bool", value: true }, i + 1];
  if (tok === "false") return [{ kind: "bool", value: false }, i + 1];
  if (tok === "nil") return [{ kind: "nil" }, i + 1];
  if (tok.startsWith('"') && tok.endsWith('"')) {
    return [{ kind: "str", value: JSON.parse(tok) }, i + 1];
  }
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

function bindPattern(nameForm, jsExprCode, decls, innerEnv) {
  if (nameForm.kind === "sym") {
    const jsName = freshName(nameForm.name);
    decls.push(`let ${jsName} = ${jsExprCode};`);
    innerEnv.set(nameForm.name, jsName);
    return;
  }
  if (nameForm.kind === "vector") {
    // Vector destructuring `[a b c]`, recursively so nested patterns like
    // `[[a b] c]` work too. Positions past the source's length (or missing
    // nested elements) bind to nil (JS `null`, matching this backend's own
    // `nil` -> `null` mapping), not JS `undefined`, so `(nil? c)` on a
    // missing position works the same way it does on the real host.
    const jsTemp = freshName("destructure");
    decls.push(`let ${jsTemp} = ${jsExprCode};`);
    for (let k = 0; k < nameForm.items.length; k += 1) {
      const subCode = `(${jsTemp}[${k}] === undefined ? null : ${jsTemp}[${k}])`;
      bindPattern(nameForm.items[k], subCode, decls, innerEnv);
    }
    return;
  }
  throw new SyntaxError("tiny analyzer: let binding name");
}

function emitFn(args, env) {
  let rest = args;
  let jsFnName = "";
  let fnEnv = env;
  if (rest.length > 0 && rest[0].kind === "sym") {
    const nameForm = rest[0];
    jsFnName = freshName(nameForm.name);
    fnEnv = new Map(env);
    fnEnv.set(nameForm.name, jsFnName);
    rest = rest.slice(1);
  }
  const [paramsForm, ...body] = rest;
  if (!paramsForm || paramsForm.kind !== "vector" || body.length === 0) {
    throw new SyntaxError("tiny analyzer: malformed fn");
  }
  const paramsEnv = new Map(fnEnv);
  const jsParams = [];
  for (const p of paramsForm.items) {
    if (p.kind !== "sym") throw new SyntaxError("tiny analyzer: fn param must be a symbol");
    const jsParam = freshName(p.name);
    jsParams.push(jsParam);
    paramsEnv.set(p.name, jsParam);
  }
  const bodyCode = body.map((f) => emitExpr(f, paramsEnv));
  const returnCode = bodyCode[bodyCode.length - 1];
  return `(function ${jsFnName}(${jsParams.join(", ")}) { return (${returnCode}); })`;
}

// Emits `form` as the TAIL position of a `loop` body into `stmts` (an
// array of JS statement-text lines, appended in place -- matching/nesting
// braces textually is enough since everything is joined back into one JS
// source string). Returns a JS expression-text string when the tail form
// is an ordinary value (caller should wrap it in `return (...)`), or
// `null` when the tail form was a `recur` (or an `if`/`do` whose own tail
// resolved to one) -- those cases already pushed their own `continue;`/
// `return` statements into `stmts`, so the caller must NOT also emit a
// return for them.
//
// `recur` is only recognized here, in `loop`'s own tail position (and
// recursively inside `if`/`do` nested in that tail position) -- unlike
// real ClojureScript, a bare `recur` inside a plain `fn` (self-recursion
// without an explicit `loop`) is NOT supported here; ordinary recursive
// self-calls (already covered by this backend's named-`fn` fixtures) are
// the way to do that instead. Narrower than real ClojureScript, but no
// fixture needs the bare-fn case, and it keeps `emitFn` untouched.
function emitTailForm(form, env, recurCtx, stmts) {
  if (form.kind === "list" && form.items[0] && form.items[0].kind === "sym") {
    const headName = form.items[0].name;
    if (headName === "recur") {
      const recurArgs = form.items.slice(1);
      if (recurArgs.length !== recurCtx.jsNames.length) {
        throw new SyntaxError("tiny analyzer: recur arity");
      }
      // Compute every new value from the OLD bindings into fresh temps
      // FIRST, then reassign all of them -- matching Clojure's own
      // "recur rebinds simultaneously, not sequentially" semantics (a
      // `(recur b a)`-style swap must see the pre-recur values of both).
      const argCodes = recurArgs.map((a) => emitExpr(a, env));
      const temps = argCodes.map((code) => {
        const t = freshName("recur_tmp");
        stmts.push(`let ${t} = ${code};`);
        return t;
      });
      recurCtx.jsNames.forEach((jsName, idx) => stmts.push(`${jsName} = ${temps[idx]};`));
      stmts.push("continue;");
      return null;
    }
    if (headName === "if") {
      const ifArgs = form.items.slice(1);
      if (ifArgs.length !== 3) throw new SyntaxError("tiny analyzer: if arity");
      const [test, then, els] = ifArgs;
      const testCode = emitExpr(test, env);
      stmts.push(`if (${testCode} !== false && ${testCode} !== null) {`);
      const thenVal = emitTailForm(then, env, recurCtx, stmts);
      if (thenVal !== null) stmts.push(`return (${thenVal});`);
      stmts.push(`} else {`);
      const elseVal = emitTailForm(els, env, recurCtx, stmts);
      if (elseVal !== null) stmts.push(`return (${elseVal});`);
      stmts.push(`}`);
      return null;
    }
    if (headName === "do") {
      const doArgs = form.items.slice(1);
      if (doArgs.length === 0) throw new SyntaxError("tiny analyzer: do arity");
      for (const f of doArgs.slice(0, -1)) stmts.push(`${emitExpr(f, env)};`);
      return emitTailForm(doArgs[doArgs.length - 1], env, recurCtx, stmts);
    }
  }
  return emitExpr(form, env);
}

function emitLoop(args, env) {
  if (args.length < 2) throw new SyntaxError("tiny analyzer: loop arity");
  const [bindingsForm, ...body] = args;
  if (bindingsForm.kind !== "vector" || bindingsForm.items.length % 2 !== 0) {
    throw new SyntaxError("tiny analyzer: malformed loop bindings");
  }
  const innerEnv = new Map(env);
  const initDecls = [];
  const jsNames = [];
  for (let i = 0; i < bindingsForm.items.length; i += 2) {
    const nameForm = bindingsForm.items[i];
    if (nameForm.kind !== "sym") throw new SyntaxError("tiny analyzer: loop binding name must be a symbol");
    const initForm = bindingsForm.items[i + 1];
    const initCode = emitExpr(initForm, innerEnv);
    const jsName = freshName(nameForm.name);
    initDecls.push(`let ${jsName} = ${initCode};`);
    innerEnv.set(nameForm.name, jsName);
    jsNames.push(jsName);
  }
  const recurCtx = { jsNames };
  const stmts = [];
  for (const f of body.slice(0, -1)) {
    stmts.push(`${emitExpr(f, innerEnv)};`);
  }
  const lastForm = body[body.length - 1];
  const tailVal = emitTailForm(lastForm, innerEnv, recurCtx, stmts);
  if (tailVal !== null) stmts.push(`return (${tailVal});`);
  return `(function(){ ${initDecls.join(" ")} while (true) { ${stmts.join(" ")} } })()`;
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
    case "str":
      return JSON.stringify(form.value);
    case "vector":
      return `[${form.items.map((f) => emitExpr(f, env)).join(", ")}]`;
    case "map": {
      // Keys are keyword/string literals only (the DDC fixtures this
      // backend targets), emitted as a plain JS object -- matching what
      // cljs.js's own clj->js conversion produces for a returned map
      // (keyword keys become string keys), so results compare equal via
      // assert.deepEqual against the real host.
      const pairs = [];
      for (let i = 0; i < form.items.length; i += 2) {
        const keyForm = form.items[i];
        if (keyForm.kind !== "keyword" && keyForm.kind !== "str") {
          throw new SyntaxError("tiny analyzer: map literal key must be a keyword or string");
        }
        const key = keyForm.kind === "keyword" ? keyForm.name : keyForm.value;
        const valueCode = emitExpr(form.items[i + 1], env);
        pairs.push(`${JSON.stringify(key)}: ${valueCode}`);
      }
      return `{${pairs.join(", ")}}`;
    }
    case "set":
      // Represented as a JS array (matching clj->js's own set -> array
      // conversion, confirmed live against the real host: `#{1 2 3}`
      // evaluates to `[1,2,3]` with stable insertion order for the small
      // literal sets this backend's fixtures use). Literal set fixtures are
      // written without duplicate elements, so no de-duplication is needed
      // here.
      return `[${form.items.map((f) => emitExpr(f, env)).join(", ")}]`;
    case "sym": {
      if (!env.has(form.name)) {
        throw new SyntaxError(`tiny analyzer: unknown local ${form.name}`);
      }
      return env.get(form.name);
    }
    case "list": {
      const [head, ...args] = form.items;
      if (head.kind === "sym" && head.name === "fn") {
        return emitFn(args, env);
      }
      if (head.kind !== "sym") {
        return `(${emitExpr(head, env)})(${args.map((f) => emitExpr(f, env)).join(", ")})`;
      }
      if (head.name === "do") {
        if (args.length === 0) throw new SyntaxError("tiny analyzer: do arity");
        const bodyCode = args.map((f) => emitExpr(f, env));
        return `(function(){ ${bodyCode
          .slice(0, -1)
          .map((c) => `${c};`)
          .join(" ")} return (${bodyCode[bodyCode.length - 1]}); })()`;
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
          const initCode = emitExpr(initForm, innerEnv);
          bindPattern(nameForm, initCode, decls, innerEnv);
        }
        const bodyCode = body.map((f) => emitExpr(f, innerEnv));
        const returnCode = bodyCode[bodyCode.length - 1];
        return `(function(){ ${decls.join(" ")} return (${returnCode}); })()`;
      }
      if (head.name === "loop") {
        return emitLoop(args, env);
      }
      if (head.name === "recur") {
        throw new SyntaxError("tiny analyzer: recur outside loop tail position");
      }
      if (Object.prototype.hasOwnProperty.call(BINOPS, head.name)) {
        if (args.length !== 2) throw new SyntaxError("tiny analyzer: binary op arity");
        return `(${emitExpr(args[0], env)} ${BINOPS[head.name]} ${emitExpr(args[1], env)})`;
      }
      if (Object.prototype.hasOwnProperty.call(CMPOPS, head.name)) {
        if (args.length !== 2) throw new SyntaxError("tiny analyzer: compare op arity");
        return `(${emitExpr(args[0], env)} ${CMPOPS[head.name]} ${emitExpr(args[1], env)})`;
      }
      if (head.name === "get") {
        if (args.length !== 2) throw new SyntaxError("tiny analyzer: get arity");
        return `(${emitExpr(args[0], env)})[${emitExpr(args[1], env)}]`;
      }
      if (head.name === "nth") {
        if (args.length !== 2) throw new SyntaxError("tiny analyzer: nth arity");
        return `(${emitExpr(args[0], env)})[${emitExpr(args[1], env)}]`;
      }
      if (head.name === "count") {
        if (args.length !== 1) throw new SyntaxError("tiny analyzer: count arity");
        return `(${emitExpr(args[0], env)}).length`;
      }
      if (head.name === "conj") {
        if (args.length !== 2) throw new SyntaxError("tiny analyzer: conj arity");
        return `[...(${emitExpr(args[0], env)}), ${emitExpr(args[1], env)}]`;
      }
      if (head.name === "nil?") {
        if (args.length !== 1) throw new SyntaxError("tiny analyzer: nil? arity");
        return `(${emitExpr(args[0], env)} === null)`;
      }
      if (head.name === "assoc") {
        // Variadic `(assoc m k1 v1 k2 v2 ...)`. Keys are emitted as computed
        // properties, so keyword keys (which emit as JSON-quoted strings,
        // matching cljs's own clj->js keyword -> string-key conversion) and
        // other key expressions both work uniformly.
        if (args.length < 3 || args.length % 2 !== 1) {
          throw new SyntaxError("tiny analyzer: assoc arity");
        }
        const mapCode = emitExpr(args[0], env);
        const pairs = [];
        for (let i = 1; i < args.length; i += 2) {
          pairs.push(`[${emitExpr(args[i], env)}]: ${emitExpr(args[i + 1], env)}`);
        }
        return `{...(${mapCode}), ${pairs.join(", ")}}`;
      }
      if (head.name === "update") {
        if (args.length !== 3) throw new SyntaxError("tiny analyzer: update arity");
        const mapCode = emitExpr(args[0], env);
        const keyCode = emitExpr(args[1], env);
        const fnCode = emitExpr(args[2], env);
        const jsTemp = freshName("update_map");
        const jsKey = freshName("update_key");
        return `(function(){ let ${jsTemp} = (${mapCode}); let ${jsKey} = (${keyCode}); return {...${jsTemp}, [${jsKey}]: (${fnCode})(${jsTemp}[${jsKey}])}; })()`;
      }
      if (head.name === "when") {
        if (args.length < 2) throw new SyntaxError("tiny analyzer: when arity");
        const [test, ...body] = args;
        const bodyCode = body.map((f) => emitExpr(f, env));
        const doCode =
          bodyCode.length === 1
            ? bodyCode[0]
            : `(function(){ ${bodyCode
                .slice(0, -1)
                .map((c) => `${c};`)
                .join(" ")} return (${bodyCode[bodyCode.length - 1]}); })()`;
        return `(${emitExpr(test, env)} !== false && ${emitExpr(test, env)} !== null ? (${doCode}) : null)`;
      }
      if (head.name === "cond") {
        if (args.length % 2 !== 0) throw new SyntaxError("tiny analyzer: malformed cond");
        const buildCond = (i) => {
          if (i >= args.length) return "null";
          const testCode = emitExpr(args[i], env);
          const exprCode = emitExpr(args[i + 1], env);
          return `(${testCode} !== false && ${testCode} !== null ? (${exprCode}) : (${buildCond(i + 2)}))`;
        };
        return buildCond(0);
      }
      if (head.name === "->") {
        // Thread-first: rewrite to nested list forms at the AST level, then
        // emit once. `(-> x (op a))` -> `(op x a)`; a bare-symbol step `(->
        // x f)` -> `(f x)` (only reachable when `f` is itself a recognized
        // call head or bound local, same as any other call-head dispatch).
        if (args.length === 0) throw new SyntaxError("tiny analyzer: -> arity");
        let acc = args[0];
        for (let i = 1; i < args.length; i += 1) {
          const step = args[i];
          if (step.kind === "sym") {
            acc = { kind: "list", items: [step, acc] };
          } else if (step.kind === "list") {
            acc = { kind: "list", items: [step.items[0], acc, ...step.items.slice(1)] };
          } else {
            throw new SyntaxError("tiny analyzer: -> step must be a symbol or list");
          }
        }
        return emitExpr(acc, env);
      }
      if (env.has(head.name)) {
        const argsCode = args.map((f) => emitExpr(f, env));
        return `${env.get(head.name)}(${argsCode.join(", ")})`;
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
