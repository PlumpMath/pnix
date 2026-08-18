# 06 — meta 쌍 경계

## 쉽게 말하면 (비유)
모노레포의 다섯 호스트는 전부 "제품 런타임 절반 + 메타 증명 절반"의 쌍이다.
이 예제는 그 쌍의 **어느 쪽을 이 카탈로그가 다루는지**를 명시적으로 긋는다 —
`00`–`17`의 모든 명령은 pnix-clr **제품** 쪽만 건드리고, `clr-meta`의
Stage1–15/N 사다리 쪽은 건드리지 않는다.

| 절반 | 역할 |
|------|------|
| **pnix-clr** (이 패키지) | pnix 런타임 + 라이브러리 export + C# host-main |
| **clr-meta** (`pnix-clr/clr-meta/`) | ClojureCLR meta 컴파일러 사다리 (Stage 설계 문서) |

제품 예제는 여기에 둔다. Stage 사다리 정직성/영수증은 `clr-meta/` 에 있다 —
이 카탈로그에서 Stage15/N 을 승격하지 않는다.

## 왜 분리하나
`pnix-clr`는 ClojureCLR 위에서 pnix 소스를 파싱/평가하고 C#으로 export하는
**제품 런타임**이고, `clr-meta`는 ClojureCLR 자기 자신의 self-host/AOT
증명(`runtime-artifact.edn`, Stage1/2 nested-interpreter 사다리)을 다루는
**pnix-agnostic** 레인이다 — 섞어서 다루면 "pnix-clr가 뭘 하나"와
"clr-meta가 ClojureCLR을 스스로 컴파일할 수 있나"라는 서로 다른 질문의
답이 뒤섞인다.

## 어디에 쓰나
"이 예제가 제품 주장인가 meta 증명 주장인가"를 헷갈릴 때 참고 기준점.

## 실행
읽기 전용 문서다 — 코드 실행은 없다.

## 관련

- 모노레포 다섯 호스트 쌍 표
- `pnix-clr/clr-meta/STATUS.md`
