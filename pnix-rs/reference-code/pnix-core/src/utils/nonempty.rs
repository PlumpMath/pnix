//! NonEmptyVec - 최소 1개 원소를 보장하는 Vec 래퍼
//!
//! pnix-old의 symbolic_core/src/ast/nonempty.rs에서 마이그레이션
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 컬렉션 타입, 값 연산 없음
//!
//! ## 설계 원칙
//!
//! - 컴파일 타임에 빈 벡터 방지
//! - 기존 Vec API와 최대한 호환
//! - serde 직렬화 지원

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::ops::Index;
use std::slice::Iter;

/// 최소 1개 원소를 보장하는 Vec: 빈 벡터를 방지하는 Vec 래퍼
///
/// # 불변식
/// - `inner.len() >= 1` 항상 참
///
/// 헌법 P0-1 준수: 구조 정의만, 값 계산 없음
///
/// # 예시
/// ```ignore
/// use pnix_core::utils::NonEmptyVec;
///
/// let v = NonEmptyVec::new(1, vec![2, 3]);
/// assert_eq!(v.first(), &1);
/// assert_eq!(v.len(), 3);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NonEmptyVec<T> {
  head: T,
  tail: Vec<T>,
}

impl<T> NonEmptyVec<T> {
  /// 새 NonEmptyVec 생성
  ///
  /// # Arguments
  /// - `head`: 첫 번째 원소 (필수)
  /// - `tail`: 나머지 원소들
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(head: T, tail: Vec<T>) -> Self {
    Self { head, tail }
  }

  /// 단일 원소로 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn singleton(value: T) -> Self {
    Self {
      head: value,
      tail: Vec::new(),
    }
  }

  /// 2개 원소로 생성 (이항 연산용)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn pair(first: T, second: T) -> Self {
    Self {
      head: first,
      tail: vec![second],
    }
  }

  /// Vec에서 변환 시도
  ///
  /// 빈 Vec이면 None 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn from_vec(mut v: Vec<T>) -> Option<Self> {
    if v.is_empty() {
      return None;
    }
    let head = v.remove(0);
    Some(Self { head, tail: v })
  }

  /// Vec에서 변환 (빈 경우 기본값 사용)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn from_vec_or(v: Vec<T>, default: T) -> Self {
    Self::from_vec(v).unwrap_or_else(|| Self::singleton(default))
  }

  /// 첫 번째 원소 참조
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn first(&self) -> &T {
    &self.head
  }

  /// 첫 번째 원소 가변 참조
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn first_mut(&mut self) -> &mut T {
    &mut self.head
  }

  /// 마지막 원소 참조
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn last(&self) -> &T {
    self.tail.last().unwrap_or(&self.head)
  }

  /// 길이
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn len(&self) -> usize {
    1 + self.tail.len()
  }

  /// 항상 false (NonEmpty이므로)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_empty(&self) -> bool {
    false
  }

  /// 원소 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn push(&mut self, value: T) {
    self.tail.push(value);
  }

  /// 마지막 원소 제거
  ///
  /// 원소가 1개만 남으면 None 반환 (불변식 보호)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 제거만, 값 계산 없음
  pub fn pop(&mut self) -> Option<T> {
    self.tail.pop()
  }

  /// 이터레이터
  pub fn iter(&self) -> NonEmptyIter<'_, T> {
    NonEmptyIter {
      head: Some(&self.head),
      tail: self.tail.iter(),
    }
  }

  /// Vec으로 변환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn into_vec(self) -> Vec<T> {
    let mut v = vec![self.head];
    v.extend(self.tail);
    v
  }

  /// Vec 참조로 변환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn as_slice(&self) -> NonEmptySlice<'_, T> {
    NonEmptySlice {
      head: &self.head,
      tail: &self.tail,
    }
  }

  /// map 변환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn map<U, F: FnMut(T) -> U>(self, mut f: F) -> NonEmptyVec<U> {
    NonEmptyVec {
      head: f(self.head),
      tail: self.tail.into_iter().map(f).collect(),
    }
  }

  /// 참조 map 변환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn map_ref<U, F: FnMut(&T) -> U>(&self, mut f: F) -> NonEmptyVec<U> {
    NonEmptyVec {
      head: f(&self.head),
      tail: self.tail.iter().map(f).collect(),
    }
  }
}

impl<T: Clone> NonEmptyVec<T> {
  /// Vec 슬라이스에서 변환 시도
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn from_slice(s: &[T]) -> Option<Self> {
    if s.is_empty() {
      return None;
    }
    Some(Self {
      head: s[0].clone(),
      tail: s[1..].to_vec(),
    })
  }
}

// ─────────────────────────────────────────────
// Iterator
// ─────────────────────────────────────────────

/// NonEmptyVec 이터레이터: NonEmptyVec의 원소를 순회하는 이터레이터
pub struct NonEmptyIter<'a, T> {
  head: Option<&'a T>,
  tail: Iter<'a, T>,
}

impl<'a, T> Iterator for NonEmptyIter<'a, T> {
  type Item = &'a T;

  fn next(&mut self) -> Option<Self::Item> {
    if let Some(h) = self.head.take() {
      Some(h)
    } else {
      self.tail.next()
    }
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    let len = if self.head.is_some() { 1 } else { 0 } + self.tail.len();
    (len, Some(len))
  }
}

impl<T> ExactSizeIterator for NonEmptyIter<'_, T> {}

// ─────────────────────────────────────────────
// NonEmptySlice (참조용)
// ─────────────────────────────────────────────

/// NonEmptyVec의 불변 슬라이스: NonEmptyVec의 참조 슬라이스
pub struct NonEmptySlice<'a, T> {
  head: &'a T,
  tail: &'a [T],
}

impl<'a, T> NonEmptySlice<'a, T> {
  /// 첫 번째 원소
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn first(&self) -> &T {
    self.head
  }

  /// 길이
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn len(&self) -> usize {
    1 + self.tail.len()
  }

  /// 항상 false
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_empty(&self) -> bool {
    false
  }

  /// 이터레이터
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn iter(&self) -> NonEmptyIter<'_, T> {
    NonEmptyIter {
      head: Some(self.head),
      tail: self.tail.iter(),
    }
  }
}

// ─────────────────────────────────────────────
// Trait 구현
// ─────────────────────────────────────────────

impl<T> Index<usize> for NonEmptyVec<T> {
  type Output = T;

  fn index(&self, index: usize) -> &Self::Output {
    if index == 0 {
      &self.head
    } else {
      &self.tail[index - 1]
    }
  }
}

impl<T> IntoIterator for NonEmptyVec<T> {
  type Item = T;
  type IntoIter = std::vec::IntoIter<T>;

  fn into_iter(self) -> Self::IntoIter {
    self.into_vec().into_iter()
  }
}

impl<'a, T> IntoIterator for &'a NonEmptyVec<T> {
  type Item = &'a T;
  type IntoIter = NonEmptyIter<'a, T>;

  fn into_iter(self) -> Self::IntoIter {
    self.iter()
  }
}

// ─────────────────────────────────────────────
// Serde
// ─────────────────────────────────────────────

impl<T: Serialize> Serialize for NonEmptyVec<T> {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(self.len()))?;
    seq.serialize_element(&self.head)?;
    for item in &self.tail {
      seq.serialize_element(item)?;
    }
    seq.end()
  }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for NonEmptyVec<T> {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let v = Vec::<T>::deserialize(deserializer)?;
    Self::from_vec(v)
      .ok_or_else(|| serde::de::Error::custom("NonEmptyVec requires at least one element"))
  }
}

// ─────────────────────────────────────────────
// From/Into
// ─────────────────────────────────────────────

impl<T> From<NonEmptyVec<T>> for Vec<T> {
  fn from(v: NonEmptyVec<T>) -> Self {
    v.into_vec()
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_singleton() {
    let v = NonEmptyVec::singleton(42);
    assert_eq!(v.len(), 1);
    assert_eq!(v.first(), &42);
    assert_eq!(v.last(), &42);
    assert!(!v.is_empty());
  }

  #[test]
  fn test_pair() {
    let v = NonEmptyVec::pair(1, 2);
    assert_eq!(v.len(), 2);
    assert_eq!(v[0], 1);
    assert_eq!(v[1], 2);
  }

  #[test]
  fn test_new() {
    let v = NonEmptyVec::new(1, vec![2, 3, 4]);
    assert_eq!(v.len(), 4);
    assert_eq!(v.first(), &1);
    assert_eq!(v.last(), &4);
  }

  #[test]
  fn test_from_vec() {
    assert!(NonEmptyVec::<i32>::from_vec(vec![]).is_none());
    assert!(NonEmptyVec::from_vec(vec![1]).is_some());
    assert!(NonEmptyVec::from_vec(vec![1, 2, 3]).is_some());
  }

  #[test]
  fn test_from_vec_or() {
    let v = NonEmptyVec::from_vec_or(vec![], 0);
    assert_eq!(v.len(), 1);
    assert_eq!(v.first(), &0);

    let v2 = NonEmptyVec::from_vec_or(vec![1, 2], 0);
    assert_eq!(v2.len(), 2);
    assert_eq!(v2.first(), &1);
  }

  #[test]
  fn test_push_pop() {
    let mut v = NonEmptyVec::singleton(1);
    v.push(2);
    v.push(3);
    assert_eq!(v.len(), 3);

    assert_eq!(v.pop(), Some(3));
    assert_eq!(v.pop(), Some(2));
    assert_eq!(v.pop(), None); // 마지막 원소는 pop 불가
    assert_eq!(v.len(), 1);
  }

  #[test]
  fn test_iter() {
    let v = NonEmptyVec::new(1, vec![2, 3]);
    let collected: Vec<_> = v.iter().copied().collect();
    assert_eq!(collected, vec![1, 2, 3]);
  }

  #[test]
  fn test_into_vec() {
    let v = NonEmptyVec::new(1, vec![2, 3]);
    assert_eq!(v.into_vec(), vec![1, 2, 3]);
  }

  #[test]
  fn test_map() {
    let v = NonEmptyVec::new(1, vec![2, 3]);
    let doubled = v.map(|x| x * 2);
    assert_eq!(doubled.into_vec(), vec![2, 4, 6]);
  }

  #[test]
  fn test_map_ref() {
    let v = NonEmptyVec::new(1, vec![2, 3]);
    let doubled = v.map_ref(|x| x * 2);
    assert_eq!(doubled.into_vec(), vec![2, 4, 6]);
  }

  #[test]
  fn test_index() {
    let v = NonEmptyVec::new("a", vec!["b", "c"]);
    assert_eq!(v[0], "a");
    assert_eq!(v[1], "b");
    assert_eq!(v[2], "c");
  }

  #[test]
  fn test_from_slice() {
    assert!(NonEmptyVec::<i32>::from_slice(&[]).is_none());
    let v = NonEmptyVec::from_slice(&[1, 2, 3]).unwrap();
    assert_eq!(v.len(), 3);
  }

  #[test]
  fn test_as_slice() {
    let v = NonEmptyVec::new(1, vec![2, 3]);
    let slice = v.as_slice();
    assert_eq!(slice.first(), &1);
    assert_eq!(slice.len(), 3);
    assert!(!slice.is_empty());
  }

  #[test]
  fn test_serde_roundtrip() {
    let v = NonEmptyVec::new(1, vec![2, 3]);
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "[1,2,3]");

    let restored: NonEmptyVec<i32> = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.into_vec(), vec![1, 2, 3]);
  }

  #[test]
  fn test_serde_empty_fails() {
    let result: Result<NonEmptyVec<i32>, _> = serde_json::from_str("[]");
    assert!(result.is_err());
  }
}
