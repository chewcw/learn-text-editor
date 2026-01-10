pub(crate) mod terminal;
pub(crate) mod terminal_command;

use crate::editor::editor_command::EditorCommand;
use crate::view::terminal_command::{Direction, TerminalCommand};
use std::fmt::Display;
use std::io;
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub trait View
where
    Self: Sized + Clone,
{
    fn terminate(&self) -> io::Result<()>;
    fn move_caret_to_location(&mut self, direction: Direction) -> io::Result<()>;
    fn move_caret_to_position(&self, position: Position) -> io::Result<()>;
    fn print(&self, message: &str) -> io::Result<()>;
    fn hide_caret(&self) -> io::Result<()>;
    fn show_caret(&self) -> io::Result<()>;
    fn flush(&self) -> io::Result<()>;
    fn clear_screen(&self) -> io::Result<()>;
    fn clear_line(&self) -> io::Result<()>;
    fn render(&mut self) -> io::Result<()>;
    fn resize(&mut self, to: Size);
    fn size(&self) -> io::Result<Size>;
    fn handle_command(&mut self, command: TerminalCommand) -> io::Result<()>;
    fn evaluate_keypress<F>(&mut self, action: F) -> io::Result<()>
    where
        F: FnMut(EditorCommand);
    fn get_position(&mut self) -> io::Result<Position>;
}

/// Location is the absolute coordinates in the document
/// Location is measured in graphemes
#[derive(Clone, Copy, Default, Debug)]
pub struct Location {
    pub line_index: usize,     // Line number in the document (row)
    pub grapheme_index: usize, // Grapheme index within that line (column)
}

impl Location {
    pub fn to_position(&self, scroll_offset: Location) -> Position {
        Position {
            x: self
                .grapheme_index
                .saturating_sub(scroll_offset.grapheme_index),
            y: self.line_index.saturating_sub(scroll_offset.line_index),
        }
    }
}

/// Position is the absolute coordinates in the rendered viewport
/// Position is measured in screen cells
#[derive(Clone, Copy, Default)]
pub struct Position {
    pub x: usize, // x coordinates on the rendered screen grid
    pub y: usize, // y coordinates on the rendered screen grid
}

impl From<Location> for Position {
    fn from(location: Location) -> Self {
        Position {
            x: location.grapheme_index,
            y: location.line_index,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct Size {
    pub width: usize,
    pub height: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Line {
    pub(crate) fragments: Vec<TextFragment>,
}

impl Line {
    pub fn grapheme_count(&self) -> usize {
        self.fragments.len()
    }

    pub fn graphemes_width(&self) -> usize {
        self.fragments
            .iter()
            .map(|text_fragment| match text_fragment.rendered_width {
                GraphemeWidth::Half => 1,
                GraphemeWidth::Full => 2,
            })
            .sum()
    }

    pub fn get_visible_graphemes(&self, range: Range<usize>) -> String {
        let mut result = String::new();
        if range.start >= range.end {
            return result;
        }

        let mut fragment_start = 0;
        for fragment in &self.fragments {
            let fragment_end = fragment.rendered_width.saturating_add(fragment_start);
            if fragment_start > range.end {
                // Means starting from this fragment, it's out of the viewport.
                // We don't need to add anything to the result string.
                break;
            }
            if fragment_end > range.start {
                if fragment_start < range.start || fragment_end > range.end {
                    // Clip left or right
                    result.push('⋯');
                } else if let Some(char) = fragment.replacement {
                    result.push(char);
                } else {
                    result.push_str(&fragment.grapheme);
                }
            }
            fragment_start = fragment_end;
        }

        result
    }

    pub fn insert_char(&mut self, character: char, at: usize) {
        let mut result = String::new();
        for (index, fragment) in self.fragments.iter().enumerate() {
            if index == at {
                result.push(character);
            }
            result.push_str(&fragment.grapheme);
        }
        if at >= self.fragments.len() {
            result.push(character);
        }
        self.fragments = Line::from(result.as_str()).fragments;
    }

    pub fn append(&mut self, other: Self) {
        self.fragments.extend(other.fragments);
    }

    pub fn delete(&mut self, at: usize) {
        let mut result = String::new();
        self.fragments
            .iter()
            .enumerate()
            .for_each(|(index, fragment)| {
                if index != at {
                    result.push_str(&fragment.grapheme);
                }
            });
        self.fragments = Line::from(result.as_str()).fragments;
    }

    pub fn split(&mut self, at: usize) -> Self {
        if at > self.fragments.len() {
            return Self::default();
        }
        let remainder = self.fragments.split_off(at);
        Self {
            fragments: remainder,
        }
    }
}

impl From<&str> for Line {
    fn from(line_str: &str) -> Self {
        let fragments = line_str
            .graphemes(true)
            .map(|grapheme| {
                let unicode_width = grapheme.width();
                let mut rendered_width = match unicode_width {
                    0 | 1 => GraphemeWidth::Half,
                    _ => GraphemeWidth::Full,
                };
                let mut replacement: Option<char> = None;

                match grapheme {
                    "\t" => {
                        replacement = Some(' ');
                        rendered_width = GraphemeWidth::Half;
                    }
                    _ if unicode_width > 0 && grapheme.trim().is_empty() => {
                        replacement = Some('␣');
                        rendered_width = GraphemeWidth::Half;
                    }
                    _ if unicode_width == 0 => {
                        let mut chars = grapheme.chars();
                        if let Some(ch) = chars.next() {
                            if ch.is_control() && chars.next().is_none() {
                                replacement = Some('▯');
                                rendered_width = GraphemeWidth::Half;
                            }
                        }
                    }
                    _ => {}
                }

                // let replacement = match unicode_width {
                //     0 => {
                //         if grapheme.chars().all(|c| c.is_control()) {
                //             Some('▯')
                //         } else {
                //             Some('·')
                //         }
                //     }
                //     _ => {
                //         if grapheme.chars().all(|c| c == '\t') {
                //             Some(' ')
                //         } else if grapheme.trim().is_empty() {
                //             Some('␣')
                //         } else {
                //             None
                //         }
                //     }
                // };

                TextFragment {
                    grapheme: grapheme.to_string(),
                    rendered_width,
                    replacement,
                }
            })
            .collect();
        Self { fragments }
    }
}

impl Display for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = self
            .fragments
            .iter()
            .map(|fragment| fragment.grapheme.as_str())
            .collect::<String>();

        write!(f, "{}", result)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum GraphemeWidth {
    Half,
    Full,
}

impl GraphemeWidth {
    pub fn saturating_add(self, value: usize) -> usize {
        match self {
            GraphemeWidth::Half => value.saturating_add(1),
            GraphemeWidth::Full => value.saturating_add(2),
        }
    }
}

impl Into<u16> for GraphemeWidth {
    fn into(self) -> u16 {
        match self {
            GraphemeWidth::Half => 1,
            GraphemeWidth::Full => 2,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextFragment {
    grapheme: String,
    rendered_width: GraphemeWidth,
    replacement: Option<char>,
}

impl From<char> for TextFragment {
    fn from(value: char) -> Self {
        Self {
            grapheme: value.to_string(),
            rendered_width: GraphemeWidth::Half,
            replacement: None,
        }
    }
}

impl From<&str> for TextFragment {
    fn from(value: &str) -> Self {
        Self {
            grapheme: value.to_string(),
            rendered_width: GraphemeWidth::Half,
            replacement: None,
        }
    }
}
