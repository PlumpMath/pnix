# 09 — artifact 게이트 (fail-closed)

## 쉽게 말하면 (비유)
컨테이너 이미지의 다이제스트 검증과 비슷하다 — `pnix-clr`는 소스를 즉석
컴파일해 실행하지 않는다. **검증된 AOT artifact**(정확한 8-namespace
manifest, hash 일치)에 묶여서만 실행되고, artifact가 없거나 조작됐거나
불완전하면 조용히 소스 경로로 떨어지는 대신 그 자리에서 거부한다.

## 무엇을
제품 실행은 **검증된 AOT artifact**에 묶인다. 없거나 깨지면 조용히 소스
경로로 떨어지지 않는다(fail-closed). `pnix-clr-artifact-gate`가 이 계약을
정상 경로 + 조작된 20가지 negative 케이스로 검증한다.

## plain .NET의 한계
평범한 .NET 앱은 빌드 산출물이 없으면 대개 그냥 소스에서 다시 빌드하거나,
있는 대로 관대하게 로드를 시도한다 — "artifact가 정확히 이 hash·이
manifest 형태가 아니면 절대 실행하지 않는다"는 검증을 기본 제공하지
않는다.

## pnix-clr의 방식 (`pnix-clr-artifact-gate`, 실행 결과)
```
$ ./bin/build-pnix-clr-artifact
$ ./bin/pnix-clr-artifact-gate --no-build
pnix-clr artifact gate: PASS (clr-meta AOT; source fallback absent;
  cwd shadow rejected; negative matrix 20/20)
```
`negative matrix 20/20`이 핵심 — manifest 누락, hash 불일치, 네임스페이스
개수 어긋남, cwd 네임스페이스 shadow 등 20가지 조작 시나리오 전부가
차단됨을 확인한다.

## 어디에 쓰나
배포 파이프라인에서 "이 artifact가 정확히 선언된 소스로부터 나온 게
맞나"를 실행 직전에 강제하고 싶을 때(공급망 무결성).

## 실행
```bash
cd pnix-clr
./bin/build-pnix-clr-artifact
./bin/pnix-clr-artifact-gate --no-build
./bin/pnix-clr-gate
```

상세: 제품 `README.md`의 artifact 계약 절.
