# 08 — production outcome self-check (로컬)

## 쉽게 말하면 (비유)
JSON 스키마 검증기가 "이 문서가 스키마를 따르나"를 검사하듯,
`--production-outcome-self-check`는 "이 CLI 자신이 선언한 outcome 경계
(`done`/`failed`/`requested`/`suspended`)가 실제로 그 이름·형태 그대로
나오나"를 CLI 스스로 검사해서 보고한다.

## 무엇을
`Pnix.Clr`의 nominal outcome 경계(성공/실패/요청/일시중단 라벨과 그
스키마)가 실행 시점에도 선언한 그대로인지 CLI가 자체 점검해 고정된 JSON을
찍는다. (이 슬롯은 예전에는 형제 `pnix-meta` 트리의 정규 케이스를 로드하는
`--pnix-meta-smoke`였다 — 그 sibling corpus 의존은 제거됐고, 지금은 이
로컬 self-check가 실제로 살아있는 검증 표면이다.)

## plain .NET의 한계
.NET에는 "내 프로그램이 선언한 결과-형태 계약을 내가 지키고 있나"를 자동
검사해주는 표준 메커니즘이 없다 — 보통 별도 계약 테스트를 손으로 짜야
한다. `--production-outcome-self-check`는 그 계약 검사를 CLI 표면 자체에
내장해 뒀다.

## pnix-clr의 방식 (실행 결과)
```
$ ./bin/pnix-clr --production-outcome-self-check
{"all_ok":true,"done":"done","failed_class":"not-callable","failed_phase":"eval",
 "guest_shape_is_outcome":false,"requested":"requested","requested_effect":"open",
 "schema":"pnix.machine.host-outcome.v1","suspended":"suspended",
 "suspended_divergence_proven":false}
```
`all_ok: true`가 이 self-check의 핵심 신호다. `./bin/pnix-clr-production-outcome-gate`가
이 JSON을 고정 기대값과 바이트 단위로 비교해 게이트로 쓴다.

## 어디에 쓰나
outcome 스키마(`pnix.machine.host-outcome.v1`)에 의존하는 상위 도구가
"이 pnix-clr 빌드가 계약을 지키고 있나"를 CI에서 값싸게 확인할 때.

## 실행
```bash
cd pnix-clr
./bin/pnix-clr --production-outcome-self-check
./bin/pnix-clr-production-outcome-gate   # 고정 기대값과 diff
```
