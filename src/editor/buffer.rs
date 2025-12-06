use crate::view::Line;

#[derive(Clone)]
pub struct Buffer {
    pub lines: Vec<Line>,
}

impl Buffer {
    pub fn new(content: String) -> Self {
        let lines = content.lines().into_iter().map(|s| s.into()).collect();
        Self { lines }
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            lines: vec!["Hello, World!".into()],
        }
    }
}
