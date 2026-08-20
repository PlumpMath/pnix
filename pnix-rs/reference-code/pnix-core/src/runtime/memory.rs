//! Memory abstraction (data only).
//!
//! This module describes memory regions and allocation metadata without executing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 메모리 영역 ID: 메모리 영역 식별자
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryRegionId(pub u64);

/// 메모리 영역 종류: 메모리 영역 종류 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryRegionKind {
  /// 스택 영역
  Stack,
  /// 힙 영역
  Heap,
  /// 아레나 영역
  Arena,
}

/// 아레나 할당: 아레나 할당 정보 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaAllocation {
  /// 메모리 영역 ID
  pub region: MemoryRegionId,
  /// 오프셋
  pub offset: usize,
  /// 크기
  pub size: usize,
}

/// 아레나 할당자: 아레나 할당자 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaAllocator {
  /// 메모리 영역 ID
  pub region: MemoryRegionId,
  /// 용량
  pub capacity: usize,
  /// 현재 오프셋
  pub offset: usize,
  /// 할당 목록
  pub allocations: Vec<ArenaAllocation>,
}

impl ArenaAllocator {
  pub fn new(region: MemoryRegionId, capacity: usize) -> Self {
    Self {
      region,
      capacity,
      offset: 0,
      allocations: Vec::new(),
    }
  }

  pub fn allocate(&mut self, size: usize) -> Option<ArenaAllocation> {
    if self.offset.saturating_add(size) > self.capacity {
      return None;
    }
    let allocation = ArenaAllocation {
      region: self.region,
      offset: self.offset,
      size,
    };
    self.offset = self.offset.saturating_add(size);
    self.allocations.push(allocation.clone());
    Some(allocation)
  }

  pub fn reset(&mut self) {
    self.offset = 0;
    self.allocations.clear();
  }

  pub fn remaining(&self) -> usize {
    self.capacity.saturating_sub(self.offset)
  }
}

/// 메모리 영역: 메모리 영역 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRegion {
  /// 영역 ID
  pub id: MemoryRegionId,
  /// 영역 종류
  pub kind: MemoryRegionKind,
  /// 용량
  pub capacity: usize,
  /// 사용량
  pub used: usize,
}

/// 메모리 블록: 메모리 블록 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBlock {
  /// 메모리 영역 ID
  pub region: MemoryRegionId,
  /// 오프셋
  pub offset: usize,
  /// 크기
  pub size: usize,
}

/// 메모리 매니저: 메모리 관리자 구조
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MemoryManager {
  /// 다음 영역 ID
  next_id: u64,
  /// 영역 맵 (영역 ID → 영역)
  regions: HashMap<MemoryRegionId, MemoryRegion>,
  /// 아레나 맵 (영역 ID → 아레나 할당자)
  arenas: HashMap<MemoryRegionId, ArenaAllocator>,
}

impl MemoryManager {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn create_region(&mut self, kind: MemoryRegionKind, capacity: usize) -> MemoryRegionId {
    let id = MemoryRegionId(self.next_id);
    self.next_id = self.next_id.saturating_add(1);
    let region = MemoryRegion {
      id,
      kind,
      capacity,
      used: 0,
    };
    self.regions.insert(id, region);
    id
  }

  pub fn create_arena(&mut self, capacity: usize) -> MemoryRegionId {
    let id = self.create_region(MemoryRegionKind::Arena, capacity);
    let allocator = ArenaAllocator::new(id, capacity);
    self.arenas.insert(id, allocator);
    id
  }

  pub fn region(&self, id: MemoryRegionId) -> Option<&MemoryRegion> {
    self.regions.get(&id)
  }

  pub fn arena(&self, id: MemoryRegionId) -> Option<&ArenaAllocator> {
    self.arenas.get(&id)
  }

  pub fn arena_mut(&mut self, id: MemoryRegionId) -> Option<&mut ArenaAllocator> {
    self.arenas.get_mut(&id)
  }

  pub fn allocate(&mut self, id: MemoryRegionId, size: usize) -> Option<MemoryBlock> {
    let region = self.regions.get_mut(&id)?;
    if region.used.saturating_add(size) > region.capacity {
      return None;
    }
    let offset = region.used;
    region.used = region.used.saturating_add(size);
    Some(MemoryBlock {
      region: id,
      offset,
      size,
    })
  }

  pub fn allocate_arena(&mut self, id: MemoryRegionId, size: usize) -> Option<ArenaAllocation> {
    let arena = self.arenas.get_mut(&id)?;
    let allocation = arena.allocate(size)?;
    if let Some(region) = self.regions.get_mut(&id) {
      region.used = arena.offset;
    }
    Some(allocation)
  }

  pub fn reset_arena(&mut self, id: MemoryRegionId) -> bool {
    let arena = match self.arenas.get_mut(&id) {
      Some(arena) => arena,
      None => return false,
    };
    arena.reset();
    if let Some(region) = self.regions.get_mut(&id) {
      region.used = 0;
    }
    true
  }

  pub fn free(&mut self, block: MemoryBlock) -> bool {
    let region = match self.regions.get_mut(&block.region) {
      Some(region) => region,
      None => return false,
    };
    region.used = region.used.saturating_sub(block.size);
    true
  }

pub fn regions(&self) -> impl Iterator<Item = &MemoryRegion> {
  self.regions.values()
}
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn arena_allocator_tracks_offset_and_reset() {
    let mut arena = ArenaAllocator::new(MemoryRegionId(0), 16);
    let first = arena.allocate(4).expect("first allocation");
    let second = arena.allocate(6).expect("second allocation");

    assert_eq!(first.offset, 0);
    assert_eq!(second.offset, 4);
    assert_eq!(arena.offset, 10);
    assert_eq!(arena.remaining(), 6);

    arena.reset();
    assert_eq!(arena.offset, 0);
    assert!(arena.allocations.is_empty());
    assert_eq!(arena.remaining(), 16);
  }

  #[test]
  fn arena_allocator_rejects_overflow() {
    let mut arena = ArenaAllocator::new(MemoryRegionId(1), 8);
    assert!(arena.allocate(8).is_some());
    assert!(arena.allocate(1).is_none());
  }

  #[test]
  fn memory_manager_arena_updates_region_usage() {
    let mut manager = MemoryManager::new();
    let arena_id = manager.create_arena(32);

    let alloc = manager.allocate_arena(arena_id, 12).expect("arena alloc");
    assert_eq!(alloc.offset, 0);
    let region = manager.region(arena_id).expect("region");
    assert_eq!(region.used, 12);

    let alloc2 = manager.allocate_arena(arena_id, 8).expect("second alloc");
    assert_eq!(alloc2.offset, 12);
    let region = manager.region(arena_id).expect("region");
    assert_eq!(region.used, 20);

    assert!(manager.reset_arena(arena_id));
    let region = manager.region(arena_id).expect("region");
    assert_eq!(region.used, 0);
  }
}
