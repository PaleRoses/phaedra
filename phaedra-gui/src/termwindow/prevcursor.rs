use std::time::Instant;

#[derive(Clone)]
pub struct PrevCursorPos {
    when: Instant,
}

impl PrevCursorPos {
    pub fn new() -> Self {
        PrevCursorPos {
            when: Instant::now(),
        }
    }

    /// Make the cursor look like it moved
    pub fn bump(&mut self) {
        self.when = Instant::now();
    }

    pub fn last_cursor_movement(&self) -> Instant {
        self.when
    }
}
