use crate::view::{Line, Location};

#[derive(Clone)]
pub struct Buffer {
    pub lines: Vec<Line>,
}

impl Buffer {
    pub fn new(content: String) -> Self {
        let lines = content.lines().into_iter().map(|s| s.into()).collect();
        Self { lines }
    }

    pub fn line_count(&mut self) -> usize {
        self.lines.len()
    }

    // insert_newline inserts a new empty line after the specified index
    pub fn insert_newline(&mut self, after_index: usize, line: Option<Line>) {
        let line_at_index = self.lines.get_mut(after_index);
        match line_at_index {
            Some(_) => self
                .lines
                .insert(after_index + 1, line.unwrap_or_else(|| Line::from(""))),
            None => self.lines.push(line.unwrap_or_else(|| Line::from(""))),
        }
    }

    pub fn insert_char(&mut self, character: char, at: Location) {
        let height = self.lines.len();
        if at.line_index > height {
            return;
        }
        if at.line_index == height {
            let line: Line = character.to_string().as_str().into();
            self.lines.push(line);
        } else if let Some(line) = self.lines.get_mut(at.line_index) {
            line.insert_char(character, at.grapheme_index);
        }
    }

    pub fn delete(&mut self, at: Location) {
        if let Some(line) = self.lines.get(at.line_index) {
            // If the caret position is at the end of the second last line
            eprintln!(
                "DEBUG: at.grapheme_index = {}, line.fragments.len() = {}",
                at.grapheme_index,
                line.fragments.len(),
            );
            if at.grapheme_index >= line.fragments.len()
                && self.lines.len() > at.line_index.saturating_add(1)
            {
                let next_line = self.lines.remove(at.line_index.saturating_add(1));
                // Merge the next line into the current line
                self.lines[at.line_index].append(next_line);
            } else if at.grapheme_index < line.fragments.len()
                || at.grapheme_index == line.fragments.len()
            {
                self.lines[at.line_index].delete(at.grapheme_index);
            }
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
