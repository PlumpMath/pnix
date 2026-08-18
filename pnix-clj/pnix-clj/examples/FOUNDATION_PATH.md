# pnix-clj 파운데이션 경로

역사적 검증/연구 카탈로그 전에 이 순서로 본다.

| 단계 | 관심사 | 예제 | 소유 |
|------|--------|------|------|
| 1 | PNIX 값과 기본 평가 | `00-foundation/basic.clj` | `pnix-clj` 실행 |
| 2 | PNIX 값이 Clojure 로 건너감 | `00-foundation/interop.clj` | `pnix-clj.interop` |
| 3 | meta-circular 호스트 실행 | `00-foundation/meta_circular.clj` | `clj-meta` 메커니즘 (proof 전제 없음) |

## “기본”의 뜻

```text
PNIX 소스 -> 파스/평가 또는 lower/실행 -> 언어 결과
```

meta-circular 컴파일/평가도 기본이다. compile 영수증, mirror, 재컴파일
증인, 배포 결정, owner 승인은 구현을 *검증*할 수 있지만, 기본 결과를 얻는
데 **필수 전제**가 될 수 없다.

## 타입은 구조이지 이름이 아니다

`"I64"`, `"ProbeInput"`, `"~type"` 은 텍스트 값이다. 타입 권위를 주지 않는다.
호스트 interop 은 검증된 `pnix.boundary-type.v1` 노드 전체(레코드 필드,
variant, 자식 타입 포함)를 받는다.

## 확장 카탈로그

이 경로 이후, 역할별로 번호 예제를 본다:

| 역할 | 기존 예제 |
|------|-----------|
| lazy 평가·기계 동작 | `01`, `61`, `78`, `79`, `81`, `90` |
| Clojure/PNIX interop | `04`, `75`, `80` |
| meta-circular 실행 | `11`, `19`, `33`, `35`, `70`, `71` |
| import·모듈 | `51`, `89` |
| 구조화 실패 | `40`, `63`, `76`, `77`, `82` |
| 독립 proof/연구 | `05`, `16`, `20`–`30`, `34`, `46`–`49`, `93`–`94` |

proof/연구 예제는 기본 런타임의 자식이 아니다. **형제 검증 레인**이다.
