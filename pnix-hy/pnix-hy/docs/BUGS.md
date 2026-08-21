# pnix-hy 버그 / 알려진 한계

이 문서는 두 종류를 같이 담는다 — 표제에서 항상 구분해서 읽을 것:

1. **진짜 버그/한계** — 언젠가 고치면 좋은 것.
2. **의도적으로 고치지 않는 것(placeholder)** — 표시가 "이건 버그 아니라
   의도된 제한"이라고 돼 있으면, 그건 재구현/수정 대상이 **아니다**.
   LLM 에이전트가 문서화된 placeholder를 gap으로 오인해서 손대는 게
   이 프로젝트의 가장 큰 위험으로 지목돼 있다(`docs/IMPLEMENTATION.md`
   §4.3) — 아래 표에 있는 항목을 "고치기"로 시작하지 말 것.

새로 뭔가 이상하게 동작하는 걸 발견하면, 먼저 이 문서에 이미 있는지
확인할 것. 없으면 진짜 새 버그일 수 있다 — `docs/TODO.md`에 추가하거나
(간단한 수정이면) 바로 고칠 것.

## 1. 의도적 placeholder — 고정(구현하지 말 것)

> 예전 `SCOPE_LOCK.md` §3(2026-07-01 수립, 2026-08-20 이 문서로 통합)의
> 표를 그대로 옮긴 것. 이것들은 설계상 그렇다 — 각각 "미구현"이 아니라
> "문서화된 이유로 부재"다. 세 번의 독립 multi-agent stub-hunt(23+15+15
> agents, `docs/IMPLEMENTATION.md` §8.1)가 전부 확인했다.

| Placeholder | 위치 | 왜 부재로 두어야 하나 |
|---|---|---|
| derivation `outPath`/`drvPath` store addressing | `pnix_runtime.py:~2694` (`derivation_value`) | pnix는 Nix store hashing을 구현하지 않음 — 의도적; self-test로 단언됨 |
| `builtins.placeholder` | `pnix_runtime.py:~3885` | 이건 진짜 Nix builtin(`=placeholder!<name>`)이지 stub 아님 |
| `trace` / `warn` value-identity | `pnix_runtime.py:~4059` | 순수 mirror는 stderr 부작용을 의도적으로 생략 |
| pnix macro / quasiquote / defmacro / reader-macro / require | `_QUASIQUOTE/_DEFMACRO/_READER_MACRO/_IMPORT_PNIX_NOTE` (hy_mirror.py) | pnix는 비동형(non-homoiconic); Hy 쪽은 투영으로 OBSERVE만. pnix 언어가 명시적으로 동형을 채택하기로 proposal 수준에서 결정하지 않는 한 구현 금지 |
| `#_pnix-gap[...]` / `#_pnix-<tag>` 투영 마커 | `pnix_mirror.py` `_pnix_to_hy`/`_python_expr_to_pnix`/... | 의도적 "clean 투영 없음" 마커, 항상 `gaps.append`와 짝 |
| fail-closed stage16 peer-review (all-None record) | `hy-meta/bootstrap.py:~6437` | all-None 값이 헌법상 유일 허용값(non-None = DRIFT) |
| host standalone/optional fallback | `interop.py`, `host_exec.py`, `hy_mirror.py` worker, oracle | hy-meta/host 모듈 부재 시 standalone 실행 보존 |
| runtime "unsupported operand/algorithm/AST tag" | `pnix_runtime.py`, `host_exec.py` guards | 이건 ERROR MESSAGE이지 stub 아님 |

**"pnix-쪽 매크로/quasiquote/reader-macro" 항목 보충**: `docs/IMPLEMENTATION.md`
§6(interop 매트릭스)에도 같은 GAP이 "의도적 GAP" 상태로 나온다 — 중복
등록이 아니라 같은 사실을 아키텍처 관점(§6)과 "고치지 말 것" 관점(여기)
양쪽에서 참조하는 것.

## 2. 다른 4개 호스트 대비 의도적으로 좁혀둔 동작

> `docs/IMPLEMENTATION.md` §3(다른 호스트와 알려진 차이점)의 서술과 짝.
> 여기는 "왜 고치면 안 되는지/왜 아직 안 고쳤는지"에 집중.

- **`pnixMounts` 빌트인이 없다.** clj/pnix-clr 소스에 이미
  `:nix-builtin? false`, `:policy :non-faithful-extension-not-nix-coverage`
  로 명시돼 있다 — 애초에 **Nix 실제 빌트인이 아니라 pnix 자체 확장
  아이디어**다. 2026-08-19에 다른 호스트 빌트인 40개 넘게 이식하면서도
  이거 하나만 일부러 안 가져왔다 — clj/clr/cljs 3개 참조 호스트끼리도
  서로 동작이 다 다르다(정책 거부/타입에러/미종결 각각 다름), 합의 기준이
  없는 상태에서 만들면 그게 바로 "네 번째 서로 다른 동작"이 되기 때문
  (hy 에이전트가 직접 3개 호스트 소스를 확인해서 내린 결론).
  **이건 버그 아니라 의도된 제한** — 미래에 5개 호스트를 통일할 방향
  아이디어는 `docs/PLANS.md`에 있다(확정 설계 아님).
- **경로 리터럴 파싱이 clr/rs/cljs보다 좁다.** 그쪽 3개는 "숫자 또는
  식별자/닫는 괄호 뒤가 아니면 경로"까지 넓게 잡는데, hy는 "숫자 뒤가
  아니면"까지만 좁힌다. 2026-08-19에 clr의 규칙을 이식하면서 숫자 제외
  부분만 가져오고 식별자/닫는 괄호 제외 부분은 **의도적으로** 안
  가져왔다 — 그 부분까지 가져오면 hy의 기존 테스트가 깨졌기 때문.
  **이건 버그 아니라 의도된 제한.** 나중에 5개 호스트 경로 리터럴 규칙을
  통일하기로 결정하면 그때 다시 논의할 것 — 지금은 "고치기" 대상 아님.

## 3. 결정된 "안 함"(WON'T-DO) — 연구까지 했지만 하지 않기로 결정

- **Stage-polymorphic 단일 평가기 (maybe-lift) — 추진하지 않기로 결정
  (2026-07-03).** `docs/audits/2026-07-03-stagepoly-decision-research.md`
  참고(딥리서치 #3). 아이디어 자체는 "인터프리터/컴파일러가 하나의
  아티팩트가 되는 λ↑↓ 스타일 재작성"(Amin&Rompf POPL'18 Pink/Purple
  계열)이었지만:
  - **(a) in-place 재작성은 불가** — sacred mirror의 소스해시가 깨짐
    (`pnix_runtime.py`는 손대지 않는다는 원칙과 정면 충돌).
  - **(b) 별도의 hand-maintained 병행 평가기도 기각** — 문헌(RPython
    계열)이 명시적으로 반대한다(drift 위험; anti-drift는 mechanized
    generation이 필요하지, 손으로 유지보수하는 두 번째 평가기가 아니다).
    검증 방법(translation validation/refinement/metamorphic/differential/
    bisimulation)도 문헌 근거가 전혀 없었다(3차 딥리서치 R3-2 결론).
    Truffle/RPython의 실제 기법(PE 어노테이션, 메타트레이싱)은 host-specific
    이라 Hy/CPython으로 옮겨질 수도 없었다(R3-1 결론).
  - **대신**: 이미 shipped된 0029(efficient cogen)의
    `compiler_from_interpreter`/`poly_mix_in_pnix` 경로가 "인터프리터
    1개 → 컴파일러 파생"이라는 원래 목표를 RPython이 권고하는 형태
    (derive, hand-maintain 금지)로 이미 충족한다고 판단했다. sacred
    무접촉.
  - **이건 버그도 미구현도 아니라, 조사 후 내린 결정**이다. 다시 "이거
    구현 안 됐네" 하고 시작하지 말 것 — 다시 하려면 새로운 근거(예:
    검증 방법론이 새로 나옴)가 있어야 proposal로 시작할 수 있다.

## 4. `unsafeGetAttrPos` — Nix 파일 모드와 `--expr`은 다른 오라클이다 (2026-08-21)

`nix eval --expr` / `nix-instantiate -E` 는 origin 파일이 없어서 **모든**
위치(리터럴 attr 포함)가 `null`이다. 위치 대조는 **파일 모드**로만 한다.

Nix 2.34.8 파일 모드로 직접 확인한 결과:

- `inherit x;` / `inherit (s) a;` — inherit 절의 이름 위치를 반환한다.
  hy/clj/cljs/clr/rs 인터프리터와 같다.
- `builtins.mapAttrs (k: v: v) { a = 1; }` — `null`. pnix 5 호스트와 같다.
- `builtins.listToAttrs [{ name = "a"; value = 1; }]` — Nix는 위치를 남긴다
  (pnix는 `null`. 기존 `*-generated-null` fixture는 `--expr` 오라클).
- `builtins.removeAttrs { a = 1; b = 2; } ["b"]` — Nix는 남은 키의 원래
  위치를 유지. pnix는 `null`.
- `{ a = 1; } // { b = 2; }` / 같은 키 override — Nix는 기여한 쪽 바인딩
  위치. hy `//` 는 위치를 버린다.

`mapAttrs`만 "생성 attrset → null"이 Nix와 같다. listToAttrs/removeAttrs/`//`
수렴은 별 슬라이스(5 호스트). `--expr`을 오라클로 쓰지 말 것.

## 5. 감사에서 발견됐지만 이후 해결된 항목 (참고용, 현재는 문제 없음)

- `hy_mirror._proj_worker_run`/`_stage7_worker_eval`의 `readline()`에
  deadline이 없어 wedged worker가 게이트를 블록할 수 있다는 지적이
  2026-07-01 mistake-hunt 감사에서 나왔고 그때는 deferred였다. 2026-07-02
  Phase A(A10)에서 `select.select(...)` 기반 deadline + `PNIX_HY_WORKER_TIMEOUT`
  env(기본 120초)로 최종 수정됐다 — **지금은 버그 아님**, 여기 적어두는
  이유는 순전히 "예전에 알려졌던 이슈가 재발하면 A10 근처를 볼 것"이라는
  포인터용.

## 6. 현재 열려있는 진짜 버그

없음(2026-08-20 기준). 열려있는 작업 항목은 전부 `docs/TODO.md`에 있고,
그중 실제 "버그 수정" 성격인 건 없다(패키징/문서 성격). 새로 버그를
발견하면 여기 새 섹션으로 추가할 것 — 의도적 placeholder(§1/§2/§3)로
오인하지 말고, 재현 스텝과 함께 기록할 것.
