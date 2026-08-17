# pnix-hy / hy-meta

Python 언어 생태계(**Hy**, Python)와 **pnix**(순수·지연·Nix 계열 함수형 언어) 사이의
**메타서큘러(meta-circular) 투영 툴킷** + 순수·결정적·자원제한 pnix 평가 샌드박스.

- **`pnix-hy/`** — pnix 런타임 + Hy↔pnix 투영 기능 + `safe_eval`/purity/cache/IR/gate/mirror/
  action. 개발자 CLI는 **`pnix-hy-project`**.
- **`hy-meta/`** — HOST 증명 레인: Hy/Python 자기컴파일·평가·재현(stage1→7 부트스트랩, 커널,
  import hook, artifact/pyc/marshal, clean replay, introspection).
- **`stage1/`** (Python), **`stage2/`** (Hy) — hy-meta 소유의 seed/self-hosting 컴파일러
  (first-party 코드, Hy 자체와 별개). Hy 인터프리터 자체는 벤더링하지 않고 flake input
  (`github:hylang/hy` 태그 고정)에서 받아오며, `proofPython`이 설치된 패키지로 제공한다
  (`HY_ROOT` = 이 저장소 루트).
- **`SCOPE_LOCK.md`** — 권위 있는 경계 선언(무엇을 바꾸기 전에 먼저 읽을 것).

> 이 프로젝트는 **현재 선언된 meta-circular-projection scope 안에서 닫혀 있음**
> (`--check` 56/56, `--gate` PASS). 즉 *선언된 범위 기준으로 완성*이지 "전부 완성"이 아니다.
> `SCOPE_LOCK.md` 참고.

### 이중 축 + 라이브러리 (필독)

정본: [`../HOST_DEV_ENV.md`](../HOST_DEV_ENV.md). 에이전트 노트: [`CLAUDE.md`](CLAUDE.md).

| 축 | 명령 / 표면 |
|------|-------------------|
| **host-main** | `python` / `hy` with `PYTHONPATH` → `pnix_hy` (`pnix-hy-python` / `pnix-hy-hy`) |
| **pnix-main** | `nix run .#pnix-hy-pnix` / `pnix-hy-project --repl pnix` |
| **library** | installable `pnix_hy` package — **host-bound**, not portable `.px` |
| **import `.px` from Python** | `import pnix_hy as ph; ph.eval_file("x.px")` |

**이름 충돌:** flake `.#pnix-hy-hy` = `--repl hy` (source tree). HM PATH
`pnix-hy-hy` = PYTHONPATH 있는 bare Hy `PYTHONPATH`. See [`../HOST_IMPORT.md`](../HOST_IMPORT.md).

---

## Python 모듈로 설치 (pip)

`import pnix_hy`는 **일반 Python(≥3.11)에서 의존성 0, 저장소 트리 없이 동작**한다 — CORE 티어
(pnix eval / `safe_eval` / purity / `gate_check` / `check_action` / IR / witness / `explain_pnix` /
cache / diagnose)는 어디서나 실행된다. 투영·증명 티어는 추가로 Hy 1.3.1과 `hy-meta` 트리가
필요하며, 트리 밖 설치본은 `PNIX_HY_HOME`으로 그 트리를 찾는다. 투영 기능은 Hy/트리가 없으면
**호출 시점에 우아하게 물러난다**(예외/`available:False`) — CORE는 영향받지 않는다.

```sh
pip install .                    # CORE: import pnix_hy; safe_eval / gate / action / ir / witness / explain
pip install '.[projection]'      # + Hy 1.3.1  -> Hy<->pnix 투영 / mirror-over-Hy
pip install '.[full]'            # + proof ladder; 추가로: export PNIX_HY_HOME=/path/to/pnix-hy 체크아웃

pnix-hy --deployment     # 설치 위치 + 어떤 티어(core/projection/full)가 되는지 표시
```

```python
import pnix_hy as ph
ph.safe_eval("1 + 2 * 3")["value"]                 # 7      (CORE, 무의존)
ph.check_action("let a = 1; in a + 2")["status"]   # accepted
ph.deployment_info()["tiers"]                       # {core:True, projection:?, full_gate:?}
```

- **CORE 티어** = 어떤 pip 설치에서도, Hy 없이, 트리 없이 동작.
- **projection / full 티어** = `PNIX_HY_HOME`을 체크아웃(`hy-meta/` + `hy` 포함)으로 지정하고
  `projection`/`full` extra(Hy 1.3.1)를 설치, 또는 아래 Nix 방식 사용.
- 저장소 안/editable 설치는 그대로다: `PNIX_HY_HOME` 미설정 시 `HY_ROOT`는 예전처럼 저장소 sibling.

## Nix로 설치/실행 (저장소 루트의 flake)

```sh
nix build                       # .#pnix-hy 빌드 -> ./result/bin/pnix-hy-project
./result/bin/pnix-hy-project --safe-eval '1 + 2 * 3'

nix run .#pnix-hy -- --safe-eval '1 + 2 * 3'   # 위와 동일, ./result 없이
nix run .#check                 # 56개 toolkit self-check   (저장소 루트에서 실행)
nix run .#gate                  # sacred 레인 + toolkit       (저장소 루트에서 실행)
nix run .#hy-meta -- <args>     # hy-meta 호스트 proof 레인 (bootstrap.py, 저장소 루트에서)

nix develop                     # 개발셸: python + hy + pnix-hy-project 전부 PATH에

# context 유지 REPL (warm 프로세스; 저장소 루트에서):
nix run .#pnix-hy-pnix          # pnix REPL (바인딩 지속: `a = 1` 후 `a + 1` -> 2)
nix run .#pnix-hy-hy            # Hy 1.3.1 REPL
nix run .#pnix-hy-python        # python REPL (`ph` = 툴킷)
nix run .#hy-meta-hy            # hy-meta 호스트 레인용 Hy REPL
nix run .#hy-meta-python        # python REPL (sys.path에 ./hy-meta + 저장소 루트)
```

패키지: `.#pnix-hy`(CLI), `.#hy`(공식 `github:hylang/hy` 태그 `1.3.1`, `flake.lock`에 고정),
`.#proofPython`(python 3.11 + Hy 1.3.1).

**두 종류의 명령 — 이게 중요하다:**

| | 어디서 되나 | 필요 |
|---|---|---|
| 순수 기능 (`--safe-eval`, `--purity`, `--diagnose`, `--specialize`, `--ir`, `--gate-check`, `--mirror`, `--tower`, `--pnix`, `--stage-ladder`, `--receipt`, `--reify`, `--action-check`, …) | **어디서나** (설치본 `./result/bin/...`) | 없음 — 순수 stdlib |
| 투영 기능 (`--hy*`, `--quasiquote`, `--defmacro`, `--reader-macro`, `--macro-steps`, `--synth-pnix`, `--hy-roundtrip`, `--hy-closure`, `--trace`, `--interop` w/ Hy, `--correspondence`) 및 **`--check` / `--gate`** | **저장소 루트** | Hy 1.3.1 (`PNIX_HY_PYTHON`) + `HY_ROOT`의 저장소 트리 |

`.#check` / `.#gate` / `.#hy-meta`와 devShell의 `pnix-hy-project`는 `PNIX_HY_PYTHON`을 flake의
Hy 1.3.1으로 자동 설정하고 `HY_ROOT`의 **소스 트리**에서 실행하므로 저장소 루트에서 돌린다.
맨 `.#pnix-hy` / `./result` CLI는 **설치본**(순수 기능 전용; `HY_ROOT`가 Nix store라 Hy 트리 없음).

> `evaluation warning: Nixpkgs 26.05 … x86_64-darwin`, `'system' … stdenv.hostPlatform` 줄은
> 무해한 nixpkgs 알림이다.

---

## CLI (`pnix-hy-project`)

호출당 정확히 하나의 기능 플래그; 기계용 출력은 `--json` 추가.

### 순수 pnix 런타임 + 샌드박스

```sh
pnix-hy-project --safe-eval '1 + 2 * 3'                 # -> value 7 (순수 샌드박스; 부작용 없음)
pnix-hy-project --safe-eval 'builtins.readFile "/x"'    # 거부: impure
pnix-hy-project --purity 'builtins.getEnv "X"'          # effect 분류 (순수? 부작용 사용?)
pnix-hy-project --gate-check 'let a=1; in a+a'          # capability 게이트: 필요 effect, 허용 여부
pnix-hy-project --diagnose '{ a = '                     # 잘못된 소스의 파싱/평가 진단
pnix-hy-project --receipt '(+ 1 2)'                     # 내용해시 평가 영수증
pnix-hy-project --pnix '(+ (* 2 3) 4)'                  # pnix 식 평가
pnix-hy-project --ir 'let a=1; in a+2'                  # 정규화 IR (+ 해시, 평가)
pnix-hy-project --specialize 'let a=1; in a + x' --json # Futamura: dynamic vars에 대한 잔여코드
pnix-hy-project --mirror 'rec { x=1; y=x+41; }.y'       # singleton mirror facet
pnix-hy-project --stage-ladder '1 + 2'                  # pnix 런타임 stage ladder
pnix-hy-project --reify '(+ 1 2)'                       # source/form/ast/ir/value/witness 통일 물화
pnix-hy --deployment                            # 이 설치에서 어떤 티어가 되는지
```

### Action checkpoint (semantic/action VM 레이어)

```sh
pnix-hy-project --action-check   'let a = 1; in a + 2'          # verdict: accepted (순수, 한계 안)
pnix-hy-project --action-check   'builtins.readFile "/x"'       # verdict: held  (file-read 권한 필요)
pnix-hy-project --action-explain '1 +'                          # rejected + 이유(진단)
```

하나의 제안된 스텝을 **accepted / held / rejected** verdict로 묶는다 — 내용해시 witness, effect,
IR/value 해시, rollback 해시참조 포함. `safe_eval`/`gate`/`mirror`/`explain`을 재사용(두 번째 VM
없음). `docs/proposals/0009-*.md` 참고.

### Hy↔pnix 투영 (저장소 루트 + Hy 필요)

```sh
pnix-hy-project --hy '(defn f [x] (+ x 1))'       # Hy reader-form + Python lowering
pnix-hy-project --synth-pnix '(+ 1 2)'            # Hy -> pnix 소스
pnix-hy-project --roundtrip '(x: x + 1)'          # pnix -> Hy -> Python 왕복
pnix-hy-project --hy-roundtrip '(+ 1 2)'          # Hy -> pnix 값 왕복
pnix-hy-project --tower '(+ (* 2 3) 4)'           # read -> compile -> run -> pnix -> collapse
pnix-hy-project --macro-steps '(when c x)'        # Hy 매크로 확장, 단계별
pnix-hy-project --quasiquote '`(+ 1 ~x ~@ys)'     # Hy quasiquote: 정적 골격 + 구멍
pnix-hy-project --defmacro '(defmacro inc [x] `(+ ~x 1))'
pnix-hy-project --reader-macro '#_ 1 2'
pnix-hy-project --correspondence                  # Python/Hy <-> pnix AST/값 대응표
pnix-hy-project --interop 'x: x + 1'              # 명시적 Hy/Python <-> pnix interop 레코드
pnix-hy-project --trace '(+ 1 2)'                 # 호스트 실행(opcode) 트레이스
```

### 전체 toolkit 게이트

```sh
pnix-hy-project --check     # 모든 toolkit self-check (56)          -> "all_ready: True"
pnix-hy-project --gate      # + sacred 레인 (runtime self-test, rust corpus, 4-lane mirror, closure)
```

`--check`/`--gate`는 저장소 트리 + Hy 필요 — 저장소 루트에서(또는 `nix run .#check`/`.#gate`).
어떤 명령에도 `--json`으로 구조화 출력.

---

## REPL (context 유지)

두 프로젝트에 걸친 5개 REPL 모드 — **대화형·상태 유지** 탐색용. REPL은 **한 오래-사는 warm
프로세스**(인터프리터·`import hy`·평가 환경이 hot 유지)라 반복 CLI 호출보다 *빠르다*(매
`pnix-hy-project …` 호출은 Python을 새로 켜고 다시 import). "context 유지" = 세션 내 바인딩/
네임스페이스가 입력 간 지속. 두 코어는 순수 라이브러리/CLI로 두고, REPL은 얇은 프론트엔드
(평가 hot-path에 없음).

| 모드 | `nix run` 앱 | CLI (devShell/소스) | 무엇인가 |
|---|---|---|---|
| pnix-hy · **pnix** | `.#pnix-hy-pnix` | `pnix-hy-project --repl pnix` | pnix REPL(신규) — 누적 pnix env |
| pnix-hy · hy | `.#pnix-hy-hy` | `pnix-hy-project --repl hy` | Hy 1.3.1 REPL |
| pnix-hy · python | `.#pnix-hy-python` | `pnix-hy-project --repl python` | CPython REPL, `ph` = 툴킷 |
| hy-meta · hy | `.#hy-meta-hy` | — | 호스트 레인용 Hy REPL |
| hy-meta · python | `.#hy-meta-python` | — | CPython REPL, `sys.path`에 `./hy-meta` + 저장소 루트 |

위 5개 REPL 앱(`.#pnix-hy-pnix`/`.#pnix-hy-hy`/`.#pnix-hy-python`/`.#hy-meta-hy`/
`.#hy-meta-python`)은 **저장소 루트에서**(PNIX_HY_PYTHON을 flake Hy로 설정, 소스 트리 사용).
`nix develop` 안에서는 `pnix-hy-project --repl <mode>`가 PATH에 있다.

### pnix REPL

유일한 신규 인터프리터 모드. **누적되는 pnix 환경**을 런타임에 흘려 바인딩이 지속된다(순수·지연,
바인딩은 미강제 값 저장).

```text
$ nix run .#pnix-hy-pnix
pnix REPL -- pure/lazy, context-retaining. :help for commands, :quit (or Ctrl-D) to exit.
pnix> a = 20                 # 바인딩(지속); `:let a = 20` 도 가능
a bound
pnix> b = a + 22             # 이전 바인딩이 스코프에 있음
b bound
pnix> b                      # 평가; 결과 출력 + _ 에 바인딩
42
pnix> x = { k = [1 2 3]; }   # attrset/list/함수 모두 됨
x bound
pnix> x.k
[1, 2, 3]
pnix> 1 +                    # 잘못된 줄은 진단되고, 세션은 살아남음
error: PnixError: ...
pnix> _                      # `_` 는 마지막 성공 값
42
pnix> :env                   # 바인딩 목록  (:reset 초기화, :help 도움말, :quit 종료)
a, b, x
```

`name = expr`(또는 `:let name = expr`)은 바인딩; 맨 `expr`은 평가 후 결과를 `_`에 바인딩;
`:env`/`:reset`/`:help`/`:quit` 메타명령; 문법/평가 오류는 진단(diagnostic)과 함께 보고되고 세션 유지.

---

## Python API

```python
import pnix_hy as ph

ph.safe_eval("1 + 2 * 3")                          # {'ok': True, 'value': 7, ...}
ph.static_purity_check("import ./x.px")            # {'pure': False, 'impure_uses': [...]}
ph.cached_eval(source)                             # 정본 내용으로 메모이즈
ph.specialize_pnix("let a = 1; in a + x", ("x",))  # Futamura: 잔여코드 '(+ 1 x)'
ph.meta_circular_tower("(+ (* 2 3) 4)")            # read->compile->run->pnix->collapse (Hy 필요)
ph.roundtrip_host_value({"a": 1})                  # Hy/Python <-> pnix 값 fidelity 리포트
ph.check_action("let a = 1; in a + 2")             # action verdict: accepted/held/rejected + witness
ph.deployment_info()                               # 이 설치에서 되는 티어
```

`import pnix_hy`는 Hy 없이도 동작; 투영 기능은 Hy 트리가 없으면 우아하게 물러난다.
"일반 인터프리터 대비 왜 나은가" 표는 `pnix-hy/README.md` 참고.

---

## hy-meta (호스트 proof 레인)

```sh
python hy-meta/bootstrap.py <args>     # 저장소 루트에서 (./hy, ./stage1 필요)
nix run .#hy-meta -- <args>            # 위와 동일, flake의 Hy 1.3.1으로
```

hy-meta는 Hy/Python 자기컴파일·평가·재현 proof 레인이고, pnix-hy는 그 위에 얹힌다.
pip 패키지가 아니라 저장소 트리(`HY_ROOT`)에서 실행된다.

---

## Nix 없이 개발

```sh
python3.11 -m venv /tmp/pnix-hy-py311-venv
/tmp/pnix-hy-py311-venv/bin/pip install 'hy==1.3.1'
export PNIX_HY_PYTHON=/tmp/pnix-hy-py311-venv/bin/python   # 투영 "proof Python"
cd pnix-hy && PYTHONPATH=. python bin/pnix-hy --check
```

코어 eval/샌드박스는 무의존; 투영 기능 + `--check`/`--gate`만 Hy 1.3.1 필요(자동 탐색 또는
`PNIX_HY_PYTHON`).

**트리 밖 / pip 설치본**이 투영·증명 티어에 도달하려면 체크아웃을 가리키게 한다:

```sh
pip install 'pnix-hy[full]'
export PNIX_HY_PYTHON=/path/to/python-with-hy-1.3.1     # proof Python
export PNIX_HY_HOME=/path/to/pnix-hy                     # hy-meta/ + hy 가 있는 체크아웃
pnix-hy --deployment                             # projection/full = True 확인
```

`PNIX_HY_HOME` 미설정 시 동작은 정확히 in-repo 기본값(`HY_ROOT` = 저장소 sibling).

---

## 더 보기

- `pnix-hy/README.md` — pnix-hy 패키지 상세 + 설계 근거.
- `pnix-hy/examples/` — "plain의 한계 vs meta-circular 능력" 대비 예제(한글, 실행되는 데모).
- `pnix-hy/docs/` — `IMPLEMENTATION_AUDIT.md`, `INTEROP_ROLE_MATRIX.md`, `SEPARATION.md`,
  `proposals/`(변경 프로세스; 새 기능은 proposal로 시작, `SCOPE_LOCK.md` §7).
- `pnix-hy/todo.md` / `hy-meta/todo.md` — 전체 기능 목록 + 상태.

> 참고: `docs/`의 감사/설계/proposal 문서와 `todo.md`는 코드 식별자·해시·날짜가 섞인 **엔지니어링
> 기록**이라 영어/혼용으로 둔다(정밀·이력 보존 목적). 사용자용 매뉴얼(이 README와 예제)은 한글이다.


## 실행 테스트해봄.
https://teu5us.github.io/nix-lib.html

builtins.zipAttrsWith (name: values: { inherit name values; }) [ { a = "x"; } { a = "y"; b = "z"; } ]
>> { a = { name = "a"; values = [ "x" "y" ]; }; b = { name = "b"; values = [ "z" ]; }; }

builtins.typeOf 1
>> "int"
builtins.typeOf true
>> "bool"
builtins.typeOf "hello"
>> "string"
## 사용방법 모르는것 >> "path" 가 나오는방법은?
builtins.typeOf null
>> "null"
builtins.typeOf {a=1;}
>> "set"
builtins.typeOf [ 1 2 "a" ]
>> "list"
builtins.typeOf (arg: 1+arg)
>> "lambda"
builtins.typeOf 1.2
>> "float"

builtins.tryEval (1 + 2)
>> { success = true; value = 3; }

builtins.trace "여기까지 실행됨" 42
>> 42


builtins.toXML {a=1;}
>> '<?xml version=\'1.0\' encoding=\'utf-8\'?>\n<expr>\n  <attrs>\n    <attr name="a">\n      <int>1</int>\n    </attr>\n  </attrs>\n</expr>\n'

builtins.toString [ "foo" "bar" ]
>> "foo bar"

builtins.toJSON { a = 1; b = true; }
>> "{\"a\":1,\"b\":true}"

builtins.toFile "hello.txt" "안녕하세요"
>> '/private/var/folders/_r/n1cw146x1059bcb2w68qrdg00000gn/T/pnix-nix-store/2c68318e352971113645cbc72861e1ec-hello.txt'

builtins.throw "에러 발생!"
>> error: PnixCatchableError: 에러 발생!

builtins.substring 0 3 "abcdef"
>> "abc"

builtins.readFile ./hello.txt
>> 'hello world\n'

builtins.readDir ./pnix
>> {'.github': 'directory', '.gitignore': 'regular', 'LICENSE': 'regular', 'README.md': 'regular', 'pnix-clj': 'directory', 'pnix-cljs': 'directory', 'pnix-clr': 'directory', 'pnix-hy': 'directory', 'pnix-rs': 'directory'}

builtins.pathExists ./hello.txt
>> True

builtins.fetchurl "https://bootstrap.pypa.io/get-pip.py"
>> '/private/var/folders/20/drddxc8x52x63llrn6kbfvw00000gn/T/pnix-nix-store/7857aef9f8c57b58885cd8fe5ad4fb78-get-pip.py'

builtins.fetchTarball { url = "https://www.svp-team.com/files/svp4-latest.php?mac"; sha256 = "04phzhyw0haiz77j494s1rz0as5yg70gb33i864riylfj776h27v"; }
>> '/private/var/folders/20/drddxc8x52x63llrn6kbfvw00000gn/T/pnix-nix-store/d8bcf39b8267074da5e5993ddf8046cb-tarball-unpacked'

builtins.fetchGit { url = "https://github.com/NixOS/nixpkgs.git"; rev = "abcdef1234567890"; }
>> {'lastModified': 0, 'outPath': '/private/var/folders/20/drddxc8x52x63llrn6kbfvw00000gn/T/pnix-nix-store/23c625374b5d229691c6612d8440ae26-nixpkgs', 'rev': 'b78ee463a44bd56f935ec34724f54611e0b569a2', 'revCount': 0, 'shortRev': 'b78ee46', 'submodules': False}

builtins.attrNames { a = 1; b = 2; }
>> [ "a", "b" ]

builtins.attrValues { a = 1; b = 2; }
>> [ 1, 2 ]

builtins.hasAttr "a" { a = 1; }
>> true

builtins.getAttr "a" { a = 1; }
>> 1

builtins.getAttrFromPath [ "foo" "bar" ] { foo = { bar = 42; }; }
>> 42

builtins.mapAttrs (name: value: value + 1) { a = 1; b = 2; }
>> {'a': 2, 'b': 3}

builtins.filterAttrs (name: value: value > 1) { a = 1; b = 2; }
>> {'b': 2}

builtins.listToAttrs [ { name="a"; value=1; }  { name="b"; value=2; } ]
>> {'a': 1, 'b': 2}

builtins.length [1 2 3]
>> 3

builtins.head [ "a" "b" "c" ]
>> "a"

builtins.tail [ "a" "b" "c" ]
>> [ 'b', 'c' ]

lib.last [ "a" "b" "c" ] 
>> 'c'

lib.init [ "a" "b" "c" ] 
>> ['a', 'b']

builtins.elem "b" [ "a" "b" "c" ]
>> True

builtins.concatLists [ [1 2] [3 4] ]
>> [ 1, 2, 3, 4 ]

builtins.flatten [ [1 2] [3 [4 5]] ]
>> [1,2,3,4,5]

builtins.concatStringsSep ", " [ "a" "b" "c" ]
>> 'a, b, c'

lib.concatMapStringsSep "-" (x: toString x) [1 2 3]
>> '1-2-3'

lib.removePrefix "foo" "foobar"
>> 'bar'

lib.removeSuffix ".txt" "hello.txt" 
>> 'hello'

builtins.hasPrefix "foo" "foobar"
>> True

builtins.hasSuffix ".txt" "hello.txt" 
>> True

lib.splitString ":" "a:b:c"
>> ['a', 'b', 'c']

lib.toLower "Hello"
>> 'hello'

lib.toUpper "Hello"
>> 'HELLO'

lib.boolToString true
>> 'true'

lib.implies true false
>> False

lib.optional true "foo"
>> ['foo']

lib.optionals false [1 2 3] 
>> []

lib.optionalAttrs true { a = 1; }
>> {'a': 1}

lib.when false "foo"
>> None

lib.id 42
>> 42

lib.const "foo" "bar"
>> 'foo'

lib.flip (a: b: a - b) 3 10
>> 7

lib.pipe 2 [ (x: x + 3) (x: x * 2) ]
>> 10

builtins.foldl (acc: x: acc + x) 0 [1 2 3]
>> 6

builtins.foldr (x: acc: x + acc) 0 [1 2 3]
>> 6

lib.fix (self: { a = 1; b = self.a + 1; })
>> {'a': 1, 'b': 2}

lib.min 3 7
>> 3

lib.max 3 7
>> 7

lib.range 1 5
>> [1, 2, 3, 4, 5]

builtins.genList (x: x * 2) 4
>> [ 0, 2, 4, 6 ]

lib.sum [1 2 3 4]
>> 10

lib.product [1 2 3 4] 
>> 24

lib.recursiveUpdate { a = { b = 1; }; } { a = { c = 2; }; }
>> {'a': {'b': 1, 'c': 2}}

lib.updateManyAttrs [ { a = 1; } { b = 2; } ] { c = 3; }
>> {'a': 1, 'b': 2, 'c': 3}

builtins.attrByPath [ "foo" "bar" ] 0 { foo.bar = 42; }
>> 42

lib.attrsets.isAttrs { a = 1; }
>> True

lib.attrsets.mapAttrsToList (n: v: "${n}=${toString v}") { a = 1; b = 2; }
>> ['a=1', 'b=2']

lib.attrsets.zipAttrs [ { a = 1; } { a = 2; b = 3; } ]
>> {'a': [1, 2], 'b': [3]}

lib.getName { name = "hello-1.0"; }
>> 'hello'

lib.getVersion { version = "1.0"; }
>> '1.0'

lib.getAttrFromPathOr { meta = { description = "테스트"; }; } [ "meta" "description" ] "없음"
>> '테스트'

builtins.hasAttrByPath [ "meta" "license" ] { meta.license = "MIT"; }
>> True

lib.filterAttrsRecursive (name: value: name == "license") { meta = { license = "MIT"; }; }
>> {}

lib.mapAttrsRecursive (path: value: toString value) { a = { b = 1; }; }
>> {'a': {'b': '1'}}

lib.unique [1 2 2 3 1]
>> [1, 2, 3]

lib.intersectLists [1 2 3] [2 3 4]
>> [2, 3]

lib.subtractLists [1 2 3] [2]
>> []

builtins.concatMap (x: [x x]) [1 2 3]
>> [ 1, 1, 2, 2, 3, 3 ]

builtins.partition (x: x > 2) [1 2 3 4]
>> {'right': [3, 4], 'wrong': [1, 2]}

lib.zipLists [1 2] ["a" "b"]
>> [{'fst': 1, 'snd': 'a'}, {'fst': 2, 'snd': 'b'}]

lib.zipListsWith (a: b: "${toString a}${b}") [1 2] ["a" "b"]
>> ['1a', '2b']

builtins.warn "경고: deprecated 함수 사용" "foo"
>> 'foo'

lib.assert (1 + 1 == 2) "수학이 잘못됨!"
>> True

builtins.assert (1 + 1 == 2) "수학이 잘못됨!"
>> True

