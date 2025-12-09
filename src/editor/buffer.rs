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

    // new_line inserts a new empty line after the specified index
    pub fn new_line(&mut self, after_index: usize, line: Option<Line>) {
        let line_at_index = self.lines.get_mut(after_index);
        match line_at_index {
            Some(_) => self
                .lines
                .insert(after_index + 1, line.unwrap_or_else(|| Line::from(""))),
            None => self.lines.push(line.unwrap_or_else(|| Line::from(""))),
        }
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            lines: vec!["Hello, World!".into()],
        }
    }
}
