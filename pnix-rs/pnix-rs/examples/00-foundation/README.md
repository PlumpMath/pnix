# pnix-rs foundation

Run from `pnix-rs/pnix-rs`:

```sh
bash examples/00-foundation/basic.sh
bash examples/00-foundation/interop.sh
bash examples/00-foundation/meta_circular.sh
```

Set `PNIX_RS` when the executable is not named `pnix-rs`. 
The meta-circular example also needs the normal rs-meta bootstrap environment used by `substrate-check`.

실행 파일 이름이 `pnix-rs`가 아닌 경우 `PNIX_RS`를 설정합니다.
메타 순환 예제에는 'substrate-check'에서 사용하는 일반 rs-meta 부트스트랩 환경도 필요합니다.


These examples do not claim automatic code generation or a finalized in-process component invocation runtime. 
They expose the mechanisms and contracts that exist now.

이러한 예제는 자동코드 생성이나 최종처리 중인 구성요소 호출 런타임을 주장하지 않습니다.
그들은 현재 존재하는 메커니즘과 계약을 노출합니다.
