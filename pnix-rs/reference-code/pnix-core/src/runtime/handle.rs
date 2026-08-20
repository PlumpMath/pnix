//! Handle abstraction (data only).
//!
//! Represents managed runtime resources without performing I/O.

use super::process::ProcessId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// 핸들 ID: 리소스 핸들 식별자
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HandleId(pub u64);

/// 핸들 상태: 핸들 상태 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HandleState {
  /// 열림
  Open,
  /// 닫힘
  Closed,
}

/// 파일 모드: 파일 접근 모드 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileMode {
  /// 읽기 전용
  Read,
  /// 쓰기 전용
  Write,
  /// 읽기/쓰기
  ReadWrite,
}

/// 소켓 프로토콜: 소켓 프로토콜 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SocketProtocol {
  /// TCP 프로토콜
  Tcp,
  /// UDP 프로토콜
  Udp,
  /// Unix 도메인 소켓
  Unix,
}

/// 윈도우 모드: 윈도우 모드 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowMode {
  /// 윈도우 모드
  Windowed,
  /// 전체화면 모드
  Fullscreen,
}

/// 파일 핸들: 파일 핸들 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHandle {
  /// 핸들 ID
  pub id: HandleId,
  /// 파일 경로
  pub path: String,
  /// 파일 모드
  pub mode: FileMode,
  /// 핸들 상태
  pub state: HandleState,
}

/// 소켓 핸들: 소켓 핸들 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketHandle {
  /// 핸들 ID
  pub id: HandleId,
  /// 주소
  pub address: String,
  /// 프로토콜
  pub protocol: SocketProtocol,
  /// 핸들 상태
  pub state: HandleState,
}

/// 윈도우 핸들: 윈도우 핸들 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowHandle {
  /// 핸들 ID
  pub id: HandleId,
  /// 윈도우 제목
  pub title: String,
  /// 윈도우 너비
  pub width: u32,
  /// 윈도우 높이
  pub height: u32,
  /// 윈도우 모드
  pub mode: WindowMode,
  /// 핸들 상태
  pub state: HandleState,
}

/// 프로세스 핸들: 프로세스 핸들 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessHandle {
  /// 핸들 ID
  pub id: HandleId,
  /// 프로세스 ID
  pub process_id: ProcessId,
  /// 핸들 상태
  pub state: HandleState,
}

/// 핸들: 모든 리소스 핸들을 통합하는 타입
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Handle {
  /// 파일 핸들
  File(FileHandle),
  /// 소켓 핸들
  Socket(SocketHandle),
  /// 윈도우 핸들
  Window(WindowHandle),
  /// 프로세스 핸들
  Process(ProcessHandle),
}

impl Handle {
  pub fn id(&self) -> HandleId {
    match self {
      Handle::File(handle) => handle.id,
      Handle::Socket(handle) => handle.id,
      Handle::Window(handle) => handle.id,
      Handle::Process(handle) => handle.id,
    }
  }

  pub fn state(&self) -> HandleState {
    match self {
      Handle::File(handle) => handle.state,
      Handle::Socket(handle) => handle.state,
      Handle::Window(handle) => handle.state,
      Handle::Process(handle) => handle.state,
    }
  }

  pub fn is_open(&self) -> bool {
    self.state() == HandleState::Open
  }

  pub fn close(&mut self) -> Result<(), HandleError> {
    match self {
      Handle::File(handle) => {
        if handle.state == HandleState::Closed {
          return Err(HandleError::AlreadyClosed(handle.id));
        }
        handle.state = HandleState::Closed;
      }
      Handle::Socket(handle) => {
        if handle.state == HandleState::Closed {
          return Err(HandleError::AlreadyClosed(handle.id));
        }
        handle.state = HandleState::Closed;
      }
      Handle::Window(handle) => {
        if handle.state == HandleState::Closed {
          return Err(HandleError::AlreadyClosed(handle.id));
        }
        handle.state = HandleState::Closed;
      }
      Handle::Process(handle) => {
        if handle.state == HandleState::Closed {
          return Err(HandleError::AlreadyClosed(handle.id));
        }
        handle.state = HandleState::Closed;
      }
    }
    Ok(())
  }
}

/// 핸들 에러: 핸들 작업 중 발생하는 에러 타입
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HandleError {
  /// 핸들을 찾을 수 없음
  NotFound(HandleId),
  /// 이미 닫힌 핸들
  AlreadyClosed(HandleId),
}

impl fmt::Display for HandleError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      HandleError::NotFound(id) => write!(f, "handle {:?} not found", id),
      HandleError::AlreadyClosed(id) => write!(f, "handle {:?} already closed", id),
    }
  }
}

impl std::error::Error for HandleError {}

/// 핸들 레지스트리: 핸들 레지스트리 구조
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct HandleRegistry {
  /// 다음 핸들 ID
  next_id: u64,
  /// 핸들 맵 (핸들 ID → 핸들)
  handles: HashMap<HandleId, Handle>,
}

impl HandleRegistry {
  pub fn new() -> Self {
    Self::default()
  }

  fn allocate_id(&mut self) -> HandleId {
    let id = HandleId(self.next_id);
    self.next_id = self.next_id.saturating_add(1);
    id
  }

  pub fn open_file(&mut self, path: impl Into<String>, mode: FileMode) -> HandleId {
    let id = self.allocate_id();
    let handle = FileHandle {
      id,
      path: path.into(),
      mode,
      state: HandleState::Open,
    };
    self.handles.insert(id, Handle::File(handle));
    id
  }

  pub fn open_socket(&mut self, address: impl Into<String>, protocol: SocketProtocol) -> HandleId {
    let id = self.allocate_id();
    let handle = SocketHandle {
      id,
      address: address.into(),
      protocol,
      state: HandleState::Open,
    };
    self.handles.insert(id, Handle::Socket(handle));
    id
  }

  pub fn open_window(
    &mut self,
    title: impl Into<String>,
    width: u32,
    height: u32,
    mode: WindowMode,
  ) -> HandleId {
    let id = self.allocate_id();
    let handle = WindowHandle {
      id,
      title: title.into(),
      width,
      height,
      mode,
      state: HandleState::Open,
    };
    self.handles.insert(id, Handle::Window(handle));
    id
  }

  pub fn open_process(&mut self, process_id: ProcessId) -> HandleId {
    let id = self.allocate_id();
    let handle = ProcessHandle {
      id,
      process_id,
      state: HandleState::Open,
    };
    self.handles.insert(id, Handle::Process(handle));
    id
  }

  pub fn get(&self, id: HandleId) -> Option<&Handle> {
    self.handles.get(&id)
  }

  pub fn get_mut(&mut self, id: HandleId) -> Option<&mut Handle> {
    self.handles.get_mut(&id)
  }

  pub fn close(&mut self, id: HandleId) -> Result<(), HandleError> {
    let handle = self.handles.get_mut(&id).ok_or(HandleError::NotFound(id))?;
    handle.close()
  }

  pub fn remove(&mut self, id: HandleId) -> Result<Handle, HandleError> {
    self.handles.remove(&id).ok_or(HandleError::NotFound(id))
  }

  pub fn iter(&self) -> impl Iterator<Item = (&HandleId, &Handle)> {
    self.handles.iter()
  }

  pub fn len(&self) -> usize {
    self.handles.len()
  }

  pub fn is_empty(&self) -> bool {
    self.handles.is_empty()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn registry_opens_and_closes_handles() {
    let mut registry = HandleRegistry::new();
    let file = registry.open_file("/tmp/demo.txt", FileMode::ReadWrite);
    let socket = registry.open_socket("127.0.0.1:8080", SocketProtocol::Tcp);

    assert_eq!(registry.len(), 2);
    assert!(registry.get(file).unwrap().is_open());
    registry.close(file).expect("close");
    assert!(!registry.get(file).unwrap().is_open());
    assert!(registry.close(file).is_err());

    assert!(registry.remove(socket).is_ok());
    assert_eq!(registry.len(), 1);
  }

  #[test]
  fn registry_opens_process_and_window() {
    let mut registry = HandleRegistry::new();
    let process = registry.open_process(ProcessId(42));
    let window = registry.open_window("Main", 800, 600, WindowMode::Windowed);

    match registry.get(process).unwrap() {
      Handle::Process(handle) => assert_eq!(handle.process_id, ProcessId(42)),
      _ => panic!("expected process handle"),
    }

    match registry.get(window).unwrap() {
      Handle::Window(handle) => {
        assert_eq!(handle.title, "Main");
        assert_eq!(handle.width, 800);
        assert_eq!(handle.height, 600);
      }
      _ => panic!("expected window handle"),
    }
  }
}
