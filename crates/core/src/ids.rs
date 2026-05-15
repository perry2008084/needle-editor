use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BufferId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(pub u64);

static NEXT_BUFFER_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_VIEW_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_WINDOW_ID: AtomicU64 = AtomicU64::new(1);

impl BufferId {
    pub fn next() -> Self {
        Self(NEXT_BUFFER_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl ViewId {
    pub fn next() -> Self {
        Self(NEXT_VIEW_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl WindowId {
    pub fn next() -> Self {
        Self(NEXT_WINDOW_ID.fetch_add(1, Ordering::Relaxed))
    }
}
