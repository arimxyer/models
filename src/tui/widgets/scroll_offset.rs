use std::cell::Cell;

/// Interior-mutable scroll position newtype.
///
/// Uses `Cell<u16>` so that render functions can both read and write back
/// the clamped scroll position without requiring `&mut self`.
#[derive(Default)]
pub struct ScrollOffset(Cell<u16>);

impl ScrollOffset {
    pub fn new(pos: u16) -> Self {
        Self(Cell::new(pos))
    }

    pub fn get(&self) -> u16 {
        self.0.get()
    }

    pub fn set(&self, pos: u16) {
        self.0.set(pos);
    }

    pub fn increment(&self, delta: u16) {
        self.0.set(self.0.get().saturating_add(delta));
    }

    pub fn decrement(&self, delta: u16) {
        self.0.set(self.0.get().saturating_sub(delta));
    }

    pub fn jump_top(&self) {
        self.0.set(0);
    }

    pub fn jump_bottom(&self) {
        self.0.set(u16::MAX);
    }
}

impl std::fmt::Debug for ScrollOffset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ScrollOffset").field(&self.0.get()).finish()
    }
}

impl Clone for ScrollOffset {
    fn clone(&self) -> Self {
        Self(Cell::new(self.0.get()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_get() {
        let s = ScrollOffset::new(42);
        assert_eq!(s.get(), 42);
    }

    #[test]
    fn set_overwrites() {
        let s = ScrollOffset::new(10);
        s.set(99);
        assert_eq!(s.get(), 99);
    }

    #[test]
    fn increment_saturates() {
        let s = ScrollOffset::new(u16::MAX - 1);
        s.increment(5);
        assert_eq!(s.get(), u16::MAX);
    }

    #[test]
    fn decrement_saturates() {
        let s = ScrollOffset::new(2);
        s.decrement(10);
        assert_eq!(s.get(), 0);
    }

    #[test]
    fn jump_top_sets_zero() {
        let s = ScrollOffset::new(100);
        s.jump_top();
        assert_eq!(s.get(), 0);
    }

    #[test]
    fn jump_bottom_sets_max() {
        let s = ScrollOffset::new(0);
        s.jump_bottom();
        assert_eq!(s.get(), u16::MAX);
    }

    #[test]
    fn default_is_zero() {
        let s = ScrollOffset::default();
        assert_eq!(s.get(), 0);
    }

    #[test]
    fn clone_copies_value() {
        let a = ScrollOffset::new(77);
        let b = a.clone();
        assert_eq!(b.get(), 77);
        // mutations are independent
        a.set(1);
        assert_eq!(b.get(), 77);
    }

    #[test]
    fn debug_shows_value() {
        let s = ScrollOffset::new(5);
        assert_eq!(format!("{s:?}"), "ScrollOffset(5)");
    }
}
