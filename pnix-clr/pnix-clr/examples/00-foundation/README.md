# 00 — 파운데이션 (clr seed)

**코어 00–06** 경로의 첫 단계 ([FOUNDATION_PATH.md](../FOUNDATION_PATH.md)).

## 쉽게 말하면 (비유)
`./bin/pnix-clr some.px`는 다른 호스트의 `python some.py`, `node some.js`와
같은 감각이다 — 다만 실행되는 언어가 pnix(Nix 유사 게스트 언어)이고,
호스트 프로세스는 ClojureCLR/.NET이다.

## 무엇을
`program.px`(람다·`rec` attrset·불리언 비교가 섞인 seed 프로그램),
소스 기원 Int64 산술(`(-7) * (-6)`), CLI 자체의 outcome 계약 self-check
세 가지로 clr 호스트의 최소 실행 경로를 확인한다.

바깥 `pnix-clr/` 에서:

```sh
./bin/pnix-clr pnix-clr/examples/00-foundation/program.px
./bin/pnix-clr -e '(-7) * (-6)'
./bin/pnix-clr --production-outcome-self-check
```

## pnix-clr의 방식 (실행 결과)
```
$ ./bin/pnix-clr pnix-clr/examples/00-foundation/program.px
{"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":[true,false,true]}

$ ./bin/pnix-clr -e '(-7) * (-6)'
{"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":42}
```
첫 명령은 CLR 네이티브 seed(`program.px`)를 직접 돌린다. 둘째는 소스 기원
Int64 산술 경로. 셋째(self-check)는 CLI가 스스로의 outcome 경계 계약을
지키는지 고정 JSON으로 보고한다 — 자세한 내용은
[`08-production-outcome-self-check`](../08-production-outcome-self-check/).
float/BigInt/일반 수치 승격이나 primitive-manifest 강제를 주장하지 않는다.

## 어디에 쓰나
clr 호스트에서 pnix `.px` 파일을 처음 돌려볼 때의 최소 시작점.

## 실행
```bash
cd pnix-clr
./bin/pnix-clr pnix-clr/examples/00-foundation/program.px
```

카탈로그 색인: [../README.md](../README.md).
호스트 간 균형: 모노레포 `examples/EXAMPLES_BALANCE.md`.
