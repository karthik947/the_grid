use crate::types::Kline;
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct RingBuffer<T> {
    buffer: VecDeque<T>,
    capacity: usize,
}

impl<T> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn push(&mut self, item: T) -> Option<T> {
        let evicted = if self.buffer.len() >= self.capacity {
            self.buffer.pop_front()
        } else {
            None
        };

        self.buffer.push_back(item);
        evicted
    }

    pub fn replace_last(&mut self, item: T) -> Option<T> {
        let last = self.buffer.pop_back()?;
        self.buffer.push_back(item);
        Some(last)
    }

    pub fn front(&self) -> Option<&T> {
        self.buffer.front()
    }

    pub fn back(&self) -> Option<&T> {
        self.buffer.back()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.buffer.iter()
    }

    pub fn iter_without_last(&self) -> impl Iterator<Item = &T> {
        let len = self.buffer.len().saturating_sub(1);
        self.buffer.iter().take(len)
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

impl RingBuffer<Kline> {
    pub fn retain_by_open_time<F>(&mut self, mut predicate: F)
    where
        F: FnMut(i64) -> bool,
    {
        self.buffer.retain(|bar| predicate(bar.open_time));
    }
}
