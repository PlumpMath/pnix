# pnix-hy

Python 언어 생태계(Hy, Python)와 **pnix**(순수·지연·Nix 계열 함수형 언어) 사이의
**메타서큘러(meta-circular) 투영 툴킷** + 순수·결정적·자원제한 평가 샌드박스.

pnix-hy는 *언어 표현력*을 양방향으로 투영해 개발자가 메타서큘러 평가를 연구하게 하고,
**순수·샌드박스·재현 가능한** 로직 레이어가 필요한 곳(신뢰 못 할 사용자 로직, 설정/DSL 평가,
감사 가능한 계산)에서 실사용할 수 있다.

## 일반 Python/Hy 인터프리터 대비 왜 pnix-hy인가

| pnix 속성 | 효과 |
|---|---|
| 순수성(부작용 없음) | 평가가 **설계상 샌드박스**(`eval`과 다름) |
| 결정성 | 재현성; 내용주소 캐싱 |
| 지연성(laziness) | 구조에서 필요한 부분만 평가 |
| 메타서큘러 staging | 변환이 **의미보존 증명**을 동반(값 왕복 / closure) |
| Futamura 특화 | 고정 프로그램을 잔여코드로 컴파일(해석 오버헤드 제거) |

생 성능 범용 연산이나 임의 I/O에는 **적합하지 않다** — 그건 Python으로.

## 설치

```sh
pip install .                 # core: pnix 런타임 + safe_eval/purity/cache (순수 stdlib)
pip install '.[projection]'   # + Hy 1.3.0 (Hy<->pnix 투영 기능)
pip install '.[full]'         # + proof ladder; 추가로 PNIX_HY_HOME=<체크아웃> 설정
```

`import pnix_hy`의 **코어는 의존성 0 · 트리 없이** 동작한다. Hy↔pnix 투영 기능은 out-of-process로
Hy 1.3.0 "proof Python"을 호출한다(자동 탐색 또는 `PNIX_HY_PYTHON`). 트리 밖 설치본이 투영·증명
티어에 도달하려면 `PNIX_HY_HOME`을 저장소 체크아웃(`hy-meta/` + `hy` 포함)으로 지정한다.
`pnix-hy-project --deployment`로 현재 설치에서 되는 티어를 확인할 수 있다.

### Nix (저장소 루트의 flake)

```sh
nix build .#pnix-hy          # 설치 가능한 CLI (순수 런타임/샌드박스; import pnix_hy 단독 동작)
nix build .#hy               # 공식 상류 Hy 1.3.0 (github:hylang/hy 태그 1.3.0)
nix run   .#pnix-hy-project -- --safe-eval '1 + 2 * 3'
nix run   .#check            # 전체 56개 toolkit self-check   (저장소 루트에서)
nix run   .#gate             # sacred 레인 + toolkit           (저장소 루트에서)
nix run   .#hy-meta -- <args>  # hy-meta 호스트 proof 레인 (bootstrap.py, 저장소 루트에서)
nix develop                  # 개발셸: PNIX_HY_PYTHON에 Hy 1.3.0; 트리에서 --check/--gate 실행
```

`.#check`/`.#gate`/`.#hy-meta`는 `HY_ROOT`의 저장소 트리(`./hy`, `./hy-meta`, `./pnix-hy`)가
필요하고 `PNIX_HY_PYTHON`을 flake의 Hy 1.3.0으로 자동 설정하므로, 저장소 루트에서 실행한다.

## 빠른 시작

```python
import pnix_hy as ph

ph.safe_eval("1 + 2 * 3")                         # {'ok': True, 'value': 7, ...}
ph.safe_eval('builtins.getEnv "X"', pure_only=True)   # 거부: limit_exceeded='impure'
ph.static_purity_check("import ./x.px")           # {'pure': False, 'impure_uses': [...]}
ph.cached_eval(expensive_pure_source)             # 정본 내용으로 메모이즈
ph.specialize_pnix("let a = 1; in a + x", ("x",)) # Futamura: 잔여코드 '(+ 1 x)'
ph.meta_circular_tower("(+ (* 2 3) 4)")           # read->compile->run->pnix->collapse
ph.check_action("let a = 1; in a + 2")            # action verdict: accepted/held/rejected + witness
```

## CLI

```sh
pnix-hy-project --safe-eval '1 + 2 * 3'
pnix-hy-project --purity 'builtins.readFile "/etc/passwd"'
pnix-hy-project --tower '(+ (* 2 3) 4)'
pnix-hy-project --action-check 'let a = 1; in a + 2'   # action 판정
pnix-hy-project --deployment                            # 되는 티어 확인
pnix-hy-project --check                                 # 모든 toolkit self-check
```

(설치 없이 소스 체크아웃에서: `PYTHONPATH=. python bin/pnix-hy-project ...` 또는
`python -m pnix_hy.cli ...`.)

전체 기능 목록과 설계/로드맵은 `docs/TODO.md`(진행 중 작업)·`docs/PLANS.md`(미확정 방향),
실행되는 대비 예제는 `examples/` 참고.
