//! pnix: PNIX 실행기 메인 진입점

use pnix_executor_graph::ExecutionResult;

/// 메인 함수: CLI 실행
#[tokio::main]
async fn main() -> ExecutionResult<()> {
  pnix_executor_graph::run_cli(std::env::args().collect()).await
}
