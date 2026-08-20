//! IPC 프로토콜 모듈
//!
//! pnix-old의 symbolic_core/ipc에서 마이그레이션.

pub mod nrepl;
pub mod protocol;
pub mod server;

pub use nrepl::{
  ipc_op_to_nrepl_op, ipc_request_to_nrepl_map, ipc_response_to_nrepl_map,
  ipc_status_to_nrepl_status, nrepl_map_to_ipc_request, nrepl_map_to_ipc_response,
  nrepl_op_to_ipc_op, nrepl_status_to_ipc_status, NreplError,
};
pub use protocol::{IpcOp, IpcRequest, IpcResponse, IpcStatus};
pub use server::{IpcServer, SessionState};
