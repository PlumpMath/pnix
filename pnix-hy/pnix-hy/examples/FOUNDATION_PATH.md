# pnix-hy 파운데이션 경로

확장 proof/연구 카탈로그 전에 이 순서로 본다.

| 단계 | 관심사 | 예제 | 소유 |
|------|--------|------|------|
| 1 | 기본 PNIX 평가 | `00-foundation/basic.py` | `pnix-hy` 실행 |
| 2 | Python/PNIX 값 | `00-foundation/interop.py` | `pnix_hy` interop |
| 3 | meta-circular 실행 | `00-foundation/meta_circular.py` | 명시적 `pnix_hy.meta` 파사드 |

`import pnix_hy` 는 기본 런타임을 로드한다. `pnix_hy.load_meta_api()` 는
기본 meta-circular 컴파일러/평가기 표면을 로드한다. proof·action·deployment·
admission API 는 둘 다 import 하지 않으며, 각자 명시적 검증 표면으로만 요청한다.

## 타입 규칙

Python 문자열은 데이터일 뿐, PNIX 타입 증인이 아니다. 프로토콜 경계는
`pnix.boundary-type.v1` 의 닫힌 구조 ADT 노드를 쓴다. `"ProbeInput"` 같은
라벨은 표 항목 이름일 수 있으나, 검증된 record/variant/result 그래프를
대신할 수 없다.

## 확장 카탈로그

| 역할 | 기존 예제 |
|------|-----------|
| 기본 평가·진단 | `01`, `10`, `13`, `28` |
| Python/Hy/PNIX interop | `04`, `07`, `08`, `14`, `15` |
| meta-circular 실행 | `03`, `11`, `19`, `20`, `24`, `33`, `35` |
| 상태·격리 메커니즘 | `12`, `22`, `23`, `30`, `31` |
| 독립 proof/연구 | `02`, `05`, `09`, `16`–`18`, `25`–`27`, `29`, `32`, `34` |

합의·증명은 중요한 merge 증거이지만, 기본 언어 결과를 소유하지 않는다.
