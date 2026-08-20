//! pnix-setoc-pack: SETO Pack 파일 생성 도구

#[cfg(feature = "seto")]
use std::path::PathBuf;

#[cfg(feature = "seto")]
/// 메인 함수: SETO Pack 파일 생성
fn main() -> Result<(), Box<dyn std::error::Error>> {
  // pack fast-path를 끄고 legacy 경로로 로드
  std::env::set_var("PNIX_SETO_PACK", "off");

  let out = std::env::args()
    .nth(1)
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from("data/seto/setopack.msgpack"));

  let mut reg = pnix_runtime_kernel::SetoRegistry::new();
  reg.load_defaults()?;
  reg.loader().write_pack_lite_v2(&out)?;

  eprintln!("[pnix_setoc_pack] wrote {}", out.display());
  Ok(())
}

#[cfg(not(feature = "seto"))]
fn main() {
  eprintln!("This binary requires --features seto");
}
