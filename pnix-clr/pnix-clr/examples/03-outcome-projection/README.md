# 03 — 결과 투영 (outcome)

## 쉽게 말하면 (비유)
모든 `pnix-clr` CLI 호출은 "성공(`done`)했다" 또는 "구조화된 이유로
실패(`failed`)했다"는 하나의 JSON 봉투로 결과를 감싼다 — 예외를 콘솔에
찍고 프로세스가 죽는 대신, 실패도 성공처럼 값으로 관측할 수 있다.

## 무엇을
생산 경로의 결과 모양(성공 값 vs 구조화 실패)을 CLI로 확인한다. clj 전
레인 receipt 타워와 같은 깊이는 아니다 — `outcome_kind`/`error.class`
수준의 seed 확인.

## plain .NET의 한계
평범한 .NET 콘솔 앱은 성공하면 stdout에 값을, 실패하면 예외 스택트레이스를
stderr에 찍고 종료 코드를 다르게 하는 게 보통이다 — 성공/실패를 **같은
스키마의 JSON**으로 균일하게 관측하려면 그 감싸기를 직접 짜야 한다.

## pnix-clr의 방식 (실행 결과)
```
$ ./bin/pnix-clr -e 'true && !false'
{"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":true}

$ ./bin/pnix-clr -e 'if true then 40 + 2 else 0'
{"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":42}

$ ./bin/pnix-clr -e '1 / 0'
{"error":{"class":"division-by-zero","evidence":{"operator":"/"},"phase":"eval"},
 "host":"pnix-clr","outcome_kind":"failed","schema":"pnix-clr.cli-result.v1"}

$ ./bin/pnix-clr pnix-clr/examples/03-outcome-projection/ok.px
{"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":42}
```
`outcome_kind`가 `done`/`failed` 둘 중 하나로 항상 있고, 실패일 때만
`error.class`/`error.phase`가 붙는다 — 호출자가 매번 같은 스키마로 분기할
수 있다.

## 어디에 쓰나
CI나 상위 도구가 pnix-clr 호출 결과를 파싱해 성공/실패를 자동 판단할 때
(종료 코드보다 구조화된 JSON이 실패 이유까지 담고 있어 더 유용하다).

## 실행
```bash
cd pnix-clr
./bin/pnix-clr -e 'true && !false'
./bin/pnix-clr -e 'if true then 40 + 2 else 0'
./bin/pnix-clr pnix-clr/examples/03-outcome-projection/ok.px
# 선택:
#   ./bin/pnix-clr-production-outcome-gate
```
