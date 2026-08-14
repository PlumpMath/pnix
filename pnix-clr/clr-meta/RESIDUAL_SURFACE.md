# Residual surface (원칙 맵)

이 노트는 language/runtime 원칙 관점에서 **무엇이 남았는지**를 말한다 —
library version tag도, case-count bragging도, promotion claim도 아니다.

`promotion/allowed?`는 **false**로 유지된다. 여기서 residual을 닫는다는 것은
pinned corpus observation에 대한 live five-host agreement일 뿐, 그 이상 아니다.

## common slice가 지금 소유하는 것 (원칙)

| 원칙 | 이 호스트에서의 의미 |
|-----------|----------------------|
| URI as string | Deprecated URI lexer form이 plain string value로 평가됨 |
| JSON round-trip | guest value 위 `fromJSON` / `toJSON`; integer는 exact 유지; finite float는 decimal form 유지 |
| Attribute names | identifier, quoted string (빈 문자열 포함), keyword-shaped name (`true`/`false`/`null`), string interpolation의 **dynamic** key |
| Exact integers | pure int arithmetic와 compare가 signed 64-bit cell에 유지 (mantissa boundary 넘어 silent float collapse 없음) |
| Mixed numeric ops | Int with float promote; signed zero 보존; ceil/floor가 lossy int→float seam 거부 |
| Non-finite observation | Inf/NaN string form; NaN은 결코 scalar-equal 아님; **shared value cell**은 list/attrset 안에서 equal일 수 있음 |
| POSIX ERE classes | bracket 안 `[[:name:]]`은 ASCII (C locale), Unicode property 아님; unknown name은 fail closed |
| Failed thunk replay | thunk에 저장된 catchable throw가 force마다 replay; blackhole은 in-progress state뿐 |
| Kernel / math guest modules | ordinary guest program으로 실행되는 portable `.px` parser, numbers, evaluator, math surface |

## 여전히 open (원칙 갭)

| 원칙 | residual인 이유 |
|-----------|---------------------------|
| Module compile graph | mutual recursion / deep force 하에서 `compile-module`이 host stack overflow |
| Derivation host ABI | source가 이 parser가 여전히 거부하는 surface form 사용 (interp / binding shape) |
| Term-DAG guest payload | guest path가 `fromJSON`에 non-JSON fragment (`?…`)를 공급; host JSON reader가 올바르게 fail closed |
| Full self-host fixed point | **subset**의 meta-circular bootstrap은 live; full self-hosting과 IL fixed-point는 주장하지 않음 |
| Compiler stages beyond the product floor | closed self-host recompile floor 너머 stage chain, Trusting-Trust, host promotion은 open |

## slice를 키우는 방법

1. **원칙**을 이름 붙인다 (version string 아님).
2. 그 원칙을 observable하게 만드는 최소 host surface를 구현한다.
3. pinned `expected.json`에 대한 five-host common-slice 게이트로 증명한다.
4. explicit promotion receipt가 있을 때까지 `promotion/allowed? = false`를 유지한다.

“library level”을 다시 번호 매기지 말 것. “slice N” 또는 “lib-foo v3”보다
“dynamic attribute keys” 또는 “shared NaN identity” 같은 문장을 선호한다.
