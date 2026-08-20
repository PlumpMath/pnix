//! pnix-executor-graph 메인 진입점
//!
//! FxCore Graph 실행기 - 의미 그래프를 백엔드 런타임에 적용하는 CLI 도구

use pnix_executor_graph::ExecutionResult;

/// 메인 함수: CLI 인자를 파싱하여 실행
#[tokio::main]
async fn main() -> ExecutionResult<()> {
  pnix_executor_graph::run_cli(std::env::args().collect()).await
}
