# pnix-rs — Rust ↔ pnix meta-circular toolkit

두 개의 zero-dependency Rust lane으로 구성됩니다:

- **`rs-meta/`** — 독립형 **Rust-in-Rust meta-circular 컴파일러/평가기**(stage15-N).
  Rust로 쓰여 Rust를 평가하며, in-Rust 인터프리터와 rustc 네이티브 tier를
  translation-validation(interp stdout == rustc stdout)으로 등가 유지합니다.
- **`pnix-rs/`** — **rs-meta를 기판으로 하는 pnix 런타임 프론트엔드**. px(Nix류) 값을
  Rust로 사영하고 Rust를 px로 물화하며, 그 왕복을 rs-meta 자신이 판정합니다.
  `src/px.rs`(sacred 런타임)는 **rs-meta의 평가 subset 안에서** 쓰여, substrate-check가
  `rs-meta interp == rustc == native`를 3-way로 증명합니다(반증 가능한 의존 증명).

두 lane 모두 **crates.io 의존 0**(std만). rs-meta는 네이티브 tier용으로 `rustc`를
toolchain으로 호출할 뿐입니다.

## 설치 (Nix flake)

```sh
# CLI 실행 (pure 기능은 어디서나 동작)
nix run github:…/pnix-rs#pnix-rs -- px-eval -c 'let a = 1; b = a + 2; in a + b'   # -> 4
nix run .#rs-meta   -- run -c 'fn main() { println!("{}", 6 * 7); }'              # -> 42

# 전체 self-check (소스 디렉터리에서 — 체크가 소스 파일을 읽음)
cd rs-meta  && nix run ..#rs-meta-check     # 52-gate rs-meta self-check
cd pnix-rs  && nix run ..#pnix-rs-check     # 16-report all_ready aggregate
cd pnix-rs  && nix run ..#substrate-check   # rs-meta ↔ pnix-rs 3-way 의존 증명

# 개발 셸 (bootstrap/pnix-rs가 PATH에, RS_META_BOOTSTRAP 자동 설정)
nix develop
```

`packages`: `rs-meta`, `pnix-rs`(default), **`pnix-rs-library`** (embeddable
rlib/a/dylib + `pnix_rs.h`). `apps`: `pnix-rs`, `rs-meta`, `pnix-rs-pnix`,
`pnix-rs-rust`, `pnix-rs-px-eval`, `rs-meta-check`, `pnix-rs-check`,
`substrate-check`, `gate`.

### Dual-axis + library (read this)

Canonical: [`../HOST_DEV_ENV.md`](../HOST_DEV_ENV.md). Agent notes: [`CLAUDE.md`](CLAUDE.md).

| Axis | Command / surface |
|------|-------------------|
| **host-main** | `cargo` / `rustc` with `PNIX_RS_LIB_DIR` / `PNIX_RS_INCLUDE_DIR` |
| **pnix-main** | `nix run .#pnix-rs-pnix` / `pnix-rs px-eval` |
| **library** | `nix build .#pnix-rs-library` — **host-bound**, not portable `.px` |
| **import `.px` from Rust** | `pnix_rs::eval_file` / C `pnix_rs_eval` |

Never install full `pnix-rs` + `pnix-rs-library` into the same `buildEnv`
(both ship `libpnix_rs.dylib`). Use CLI-only + library package separately.

## 예제 — plain Rust의 한계 vs Rust↔pnix 방식

`pnix-rs/examples/`에 15개 섹션(각 `limit_rust.rs` 한계 + `pnix_rs_way.sh` 방식 +
`README.md`)이 있습니다. 순수 샌드박스·내용주소 해시·mirror 왕복·**Rust↔px 사영**·
witness/gate·Futamura 사영·증분 평가·격리·action verdict·기판 계약·자기호스팅 타워·
BTA·peer-engine·runners/REPL·**호스트 Rust에 pnix 임베드**까지 — 전부 실제 실행됩니다. (Rust와 pnix 두 언어를 각각 meta-circular로 구현하고 interop시키는 것이지, 섞은 새 언어가 아닙니다.)

```sh
nix develop
cd pnix-rs && ./examples/run-all.sh
```

## 소스에서 직접 빌드

```sh
CARGO_TARGET_DIR=/tmp/rs-meta-target  cargo build --release --manifest-path rs-meta/Cargo.toml
CARGO_TARGET_DIR=/tmp/pnix-rs-target  cargo build --release --manifest-path pnix-rs/Cargo.toml
RS_META_BOOTSTRAP=/tmp/rs-meta-target/release/bootstrap \
  /tmp/pnix-rs-target/release/pnix-rs check
```

## 실행 테스트해봄.
https://teu5us.github.io/nix-lib.html
pnix-rs px-eval -c '

builtins.zipAttrsWith (name: values: { inherit name values; }) [ { a = "x"; } { a = "y"; b = "z"; } ]'
>> { a = { name = "a"; values = [ "x" "y" ]; }; b = { name = "b"; values = [ "z" ]; }; }

builtins.typeOf 1'
>> "int"
builtins.typeOf true'
>> "bool"
builtins.typeOf "hello"'
>> "string"
## 사용방법 모르는것 >> "path" 가 나오는방법은?
builtins.typeOf null'
>> "null"
builtins.typeOf {a=1;}'
>> "set"
builtins.typeOf [ 1 2 "a" ]'
>> "list"
builtins.typeOf (arg: 1+arg)'
>> "lambda"
builtins.typeOf 1.2'
>> "float"

builtins.tryEval (1 + 2)'
>> { success = true; value = 3; }


builtins.trace "여기까지 실행됨" 42
>> trace: 여기까지 실행됨
>> 42

builtins.toXML {a=1;}
>> "<?xml version='1.0' encoding='utf-8'?>\n<attrs>\n  <attr name=\"a\">\n    <int>1</int>\n  </attr>\n</attrs>\n"

builtins.toString [ "foo" "bar" ]
>> "foo bar"

builtins.toJSON { a = 1; b = true; }
>> "{\"a\":1,\"b\":true}"


builtins.toFile "hello.txt" "안녕하세요"
>> "/tmp/pnix-nix-store/2c68318e352971113645cbc72861e1ec-hello.txt"

lib.throw "강제 에러 발생!"
>> pnix-rs: px: throw: 강제 에러 발생!

builtins.substring 0 3 "abcdef"
>> "abc"

builtins.readFile ./hello.txt
>> "helo\n"


builtins.readDir ./pnix
>> { .github = "directory"; .gitignore = "regular"; LICENSE = "regular"; README.md = "regular"; pnix-clj = "directory"; pnix-cljs = "directory"; pnix-clr = "directory"; pnix-hy = "directory"; pnix-rs = "directory"; }

builtins.pathExists ./hello.txt
>> true

builtins.fetchurl "https://bootstrap.pypa.io/get-pip.py"
>> "/tmp/pnix-nix-store/7857aef9f8c57b58885cd8fe5ad4fb78-src"

builtins.fetchTarball { url = "https://www.svp-team.com/files/svp4-latest.php?mac"; sha256 = "04phzhyw0haiz77j494s1rz0as5yg70gb33i864riylfj776h27v"; }
>> "/tmp/pnix-nix-store/d8bcf39b8267074da5e5993ddf8046cb-tarball"

builtins.fetchGit { url = "https://github.com/NixOS/nixpkgs.git"; rev = "abcdef1234567890"; }
>> { outPath = "/tmp/pnix-nix-store/347094f58d7b4d80a6892fa991717f28-git"; rev = "abcdef1234567890"; url = "https://github.com/NixOS/nixpkgs.git"; }

lib.attrNames { a = 1; b = 2; }
>> [ "a" "b" ]

builtins.attrNames { a = 1; b = 2; }
>> [ "a" "b" ]

lib.attrValues { a = 1; b = 2; }
>> [ 1 2 ]

builtins.attrValues { a = 1; b = 2; }
>> [ 1 2 ]

lib.hasAttr "a" { a = 1; }
>> true

builtins.hasAttr "a" { a = 1; }
>> true

lib.getAttr "a" { a = 1; }
>> 1

builtins.getAttr "a" { a = 1; }
>> 1

lib.getAttrFromPath [ "foo" "bar" ] { foo = { bar = 42; }; }
>> 42

lib.mapAttrs (name: value: value + 1) { a = 1; b = 2; }
>> { a = 2; b = 3; }

builtins.mapAttrs (name: value: value + 1) { a = 1; b = 2; }
>> { a = 2; b = 3; }

lib.filterAttrs (name: value: value > 1) { a = 1; b = 2; }
>> { b = 2; }

lib.listToAttrs [ { name="a"; value=1; }  { name="b"; value=2; } ]
>> { a = 1; b = 2; }

builtins.listToAttrs [ { name="a"; value=1; }  { name="b"; value=2; } ]
>> { a = 1; b = 2; }

lib.length [1 2 3]
>> 3

builtins.length [1 2 3]
>> 3

lib.head [ "a" "b" "c" ]
>> "a"

builtins.head [ "a" "b" "c" ]
>> "a"

lib.tail [ "a" "b" "c" ]
>> [ "b" "c" ]

builtins.tail [ "a" "b" "c" ]
>> [ "b" "c" ]

lib.last [ "a" "b" "c" ]
>> "c"

lib.init [ "a" "b" "c" ]
>> [ "a" "b" ]

lib.elem "b" [ "a" "b" "c" ]
>> true

builtins.elem "b" [ "a" "b" "c" ]
>> true

lib.concatLists [ [1 2] [3 4] ]
>> [ 1 2 3 4 ]

builtins.concatLists [ [1 2] [3 4] ]
>> [ 1 2 3 4 ]

lib.flatten [ [1 2] [3 [4 5]] ]
>> [ 1 2 3 4 5 ]

lib.concatStringsSep ", " [ "a" "b" "c" ]
>> "a, b, c"

builtins.concatStringsSep ", " [ "a" "b" "c" ]
>> "a, b, c"

lib.concatMapStringsSep "-" (x: toString x) [1 2 3]
>> "1-2-3"

lib.removePrefix "foo" "foobar"
>> "bar"

lib.removeSuffix ".txt" "hello.txt"
>> "hello"

lib.hasPrefix "foo" "foobar"
>> true

lib.hasSuffix ".txt" "hello.txt"
>> true

lib.splitString ":" "a:b:c"
>> [ "a" "b" "c" ]

lib.toLower "Hello"
>> "hello"

lib.toUpper "Hello"
>> "HELLO"

lib.boolToString true
>> "true"

lib.implies true false
>> false

lib.optional true "foo"
>> [ "foo" ]

lib.optionals false [1 2 3]
>> [ ]

lib.optionalAttrs true { a = 1; }
>> { a = 1; }

lib.when false "foo"
>> null

lib.id 42
>> 42

lib.const "foo" "bar"
>> "foo"

lib.flip (a: b: a - b) 3 10
>> 7

lib.pipe 2 [ (x: x + 3) (x: x * 2) ]
>> 10

lib.foldl (acc: x: acc + x) 0 [1 2 3]
>> 6

builtins.foldl (acc: x: acc + x) 0 [1 2 3]
>> 6

lib.foldr (x: acc: x + acc) 0 [1 2 3]
>> 6

lib.fix (self: { a = 1; b = self.a + 1; })
>> { a = 1; b = 2; }

lib.min 3 7
>> 3

builtins.min 3 7
>> 3

lib.max 3 7
>> 7

builtins.max 3 7
>> 7

lib.range 1 5
>> [ 1 2 3 4 5 ]

lib.genList (x: x * 2) 4
>> [ 0 2 4 6 ]

builtins.genList (x: x * 2) 4
>> [ 0 2 4 6 ]

lib.sum [1 2 3 4]
>> 10

lib.product [1 2 3 4]
>> 24

lib.recursiveUpdate { a = { b = 1; }; } { a = { c = 2; }; }
>> { a = { b = 1; c = 2; }; }

lib.updateManyAttrs [ { a = 1; } { b = 2; } ] { c = 3; }
>> { a = 1; b = 2; c = 3; }

lib.attrByPath [ "foo" "bar" ] 0 { foo.bar = 42; }
>> 42

builtins.attrByPath [ "foo" "bar" ] 0 { foo.bar = 42; }
>> 42

lib.attrsets.isAttrs { a = 1; }
>> true

lib.attrsets.mapAttrsToList (n: v: "${n}=${toString v}") { a = 1; b = 2; }
>> [ "a=1" "b=2" ]

lib.attrsets.zipAttrs [ { a = 1; } { a = 2; b = 3; } ]
>> { a = [ 1 2 ]; b = [ 3 ]; }

lib.getName { name = "hello-1.0"; }
>> "hello"

lib.getVersion { version = "1.0"; }
>> "1.0"

lib.getAttrFromPathOr { meta = { description = "테스트"; }; } [ "meta" "description" ] "없음"
>> "테스트"

lib.hasAttrByPath [ "meta" "license" ] { meta.license = "MIT"; }
>> true

builtins.hasAttrByPath [ "meta" "license" ] { meta.license = "MIT"; }
>> true

lib.filterAttrsRecursive (name: value: name == "license") { meta = { license = "MIT"; }; }
>> { meta = { license = "MIT"; }; }

lib.mapAttrsRecursive (path: value: toString value) { a = { b = 1; }; }
>> { a = { b = "1"; }; }

lib.unique [1 2 2 3 1]
>> [ 1 2 3 ]

lib.intersectLists [1 2 3][2 3 4]
>> [ 2 3 ]

lib.subtractLists [1 2 3] [2]
>> [ 1 3 ]

lib.concatMap (x: [x x]) [1 2 3]
>> [ 1 1 2 2 3 3 ]

builtins.concatMap (x: [x x]) [1 2 3]
>> [ 1 1 2 2 3 3 ]

lib.partition (x: x > 2) [1 2 3 4]
>> { right = [ 3 4 ]; wrong = [ 1 2 ]; }

builtins.partition (x: x > 2) [1 2 3 4]
>> { right = [ 3 4 ]; wrong = [ 1 2 ]; }

lib.zipLists [1 2] ["a" "b"]
>> [ { fst = 1; snd = "a"; } { fst = 2; snd = "b"; } ]

lib.zipListsWith (a: b: "${toString a}${b}") [1 2] ["a" "b"]
>> [ "1a" "2b" ]

lib.warn "경고: deprecated 함수 사용" "foo"
>> trace: warning: 경고: deprecated 함수 사용
>> "foo"

lib.assert (1 + 1 == 2) "수학이 잘못됨!"
>> "수학이 잘못됨!"

# builtins.assert (1 + 1 == 2) "수학이 잘못됨!"
>> pnix-rs: px: attrset has no attribute assert

