use crate::{
    editor::{buffer::Buffer, editor_command::EditorCommand},
    view::{
        Line, Location, Position, Size, TextFragment, View,
        terminal_command::{Direction, SpecialKey, TerminalCommand},
    },
};
use crossterm::{
    Command,
    cursor::{self},
    event::{Event, KeyEvent, KeyEventKind, read},
    queue, style,
    terminal::{self, Clear, enable_raw_mode},
};
use std::io::{self, Write, stdout};

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default, Clone)]
pub struct Terminal {
    buffer: Buffer,
    needs_render: bool,
    size: Size,
    location: Location,
    scroll_offset: Location,
}

impl Terminal {
    pub fn new(file_content: String) -> Self {
        let terminal = Terminal {
            buffer: Buffer::new(file_content),
            needs_render: true,
            size: Size {
                width: terminal::size().unwrap_or_default().0 as usize,
                height: terminal::size().unwrap_or_default().1 as usize,
            },
            location: Location {
                grapheme_index: 0,
                line_index: 0,
            },
            scroll_offset: Location {
                grapheme_index: 0,
                line_index: 0,
            },
        };

        match enable_raw_mode() {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error enabling raw mode: {}", e);
            }
        };
        match terminal.enter_alternate_screen() {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error entering alternate screen: {}", e);
            }
        };
        match terminal.clear_screen() {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error clearing screen: {}", e);
            }
        };
        terminal
    }

    fn queue_command<T: Command>(&self, command: T) -> io::Result<&Self> {
        let mut stdout = io::stdout();
        queue!(stdout, command)?;
        Ok(self)
    }

    fn build_welcome_message(width: usize) -> io::Result<String> {
        let mut welcome_message = format!("{NAME} editor -- version {VERSION}");

        let x = width.saturating_sub(welcome_message.len()) / 2;
        let spaces = " ".repeat(x.saturating_sub(1));
        welcome_message = format!("~{spaces}{welcome_message}");
        welcome_message.truncate(width);

        Ok(welcome_message)
    }

    /// scroll_location_into_view scrolls the current location into view if it's outside the
    /// current view.
    /// row 0
    /// row 1
    /// row 2  ┌───────────────┐ ← view top (offset_row)
    /// row 3  │               │
    /// row 4  └───────────────┘ ← view bottom (offset_row + height - 1)
    /// row 5    ← target_row    ← currently outside view
    /// row 6
    fn scroll_location_into_view(&mut self) {
        let Location {
            line_index: target_row,
            grapheme_index: target_col,
        } = self.location;
        let Location {
            line_index: offset_row,
            grapheme_index: offset_col,
        } = self.scroll_offset;
        let Size { width, height } = self.size().unwrap_or_default();

        // Scroll vertically
        if target_row < offset_row {
            self.scroll_offset.line_index = target_row;
            self.needs_render = true;
        } else if target_row >= offset_row.saturating_add(height) {
            self.scroll_offset.line_index = target_row.saturating_sub(height).saturating_add(1);
            self.needs_render = true;
        }

        // Scroll horizontally
        if target_col < offset_col {
            self.scroll_offset.grapheme_index = target_col;
            self.needs_render = true;
        } else if target_col >= offset_col.saturating_add(width) {
            self.scroll_offset.grapheme_index = target_col.saturating_sub(width).saturating_add(1);
            self.needs_render = true;
        }
    }

    pub fn enter_alternate_screen(&self) -> io::Result<&Self> {
        self.queue_command(terminal::EnterAlternateScreen)?
            .flush()?;
        Ok(&self)
    }

    pub fn handle_ordinary_typing(&mut self, char: Option<char>) -> io::Result<()> {
        match char {
            None => return Ok(()),
            Some(c) => {
                let old_len = self
                    .buffer
                    .lines
                    .get(self.location.line_index)
                    .map_or(0, Line::grapheme_count);
                self.buffer.insert_char(c, self.location);
                let new_len = self
                    .buffer
                    .lines
                    .get(self.location.line_index)
                    .map_or(0, Line::grapheme_count);
                if new_len.saturating_sub(old_len) > 0 {
                    self.move_caret_to_location(Direction::Right)?;
                }
                self.needs_render = true;
                return Ok(());
            }
        }
    }

    pub fn handle_special_key(&mut self, special_key: SpecialKey) -> io::Result<()> {
        let current_caret_line = self.location.line_index;
        let current_caret_col = self.location.grapheme_index;
        // let last_line_index = self.buffer.line_count().saturating_sub(1);
        // let last_line_len = match self.buffer.lines.get(last_line_index) {
        //     Some(line) => line.fragments.len(),
        //     None => 0,
        // };

        match special_key {
            SpecialKey::Enter => {
                self.buffer.insert_newline(self.location);
                self.move_caret_to_location(Direction::Right)?;
                self.needs_render = true;
            }
            // SpecialKey::Tab => {
            //     let current_caret_line = self.location.line_index;
            //     let current_caret_col = self.location.grapheme_index;
            //     match self.buffer.lines.get_mut(current_caret_line) {
            //         Some(line) => {
            //             let mut fragments_after_caret = line.fragments.split_off(current_caret_col);
            //             // Insert the tab character as 4 spaces
            //             let tab = '\t';
            //             line.fragments.push(TextFragment::from(tab));
            //             // Re-append the fragments after the caret position
            //             line.fragments.append(&mut fragments_after_caret);
            //             // Move the caret right after the inserted tab character
            //             // Consider the tab as width of 4 spaces
            //             self.move_caret_to_location(Direction::Right)?;
            //             self.needs_render = true;
            //         }
            //         None => return Ok(()),
            //     };
            // }
            SpecialKey::BackTab => todo!(),
            SpecialKey::Delete => {
                self.buffer.delete(self.location);
                self.needs_render = true;
                return Ok(());
                // let last_grapheme = self
                //     .buffer
                //     .lines
                //     .get(current_caret_line)
                //     .map_or(0, |line| line.fragments.len().saturating_sub(1));
                //
                // // Bottom right of the document should do nothing
                // if current_caret_line == last_line_index
                //     && current_caret_col == last_line_len.saturating_sub(1)
                // {
                //     return Ok(());
                // } else {
                //     let next_line_index = current_caret_line.saturating_add(1);
                //
                //     // Normal delete within a line
                //     if current_caret_line != last_line_index && current_caret_col != last_grapheme {
                //         let line = match self.buffer.lines.get_mut(current_caret_line) {
                //             Some(line) => line,
                //             None => return Ok(()),
                //         };
                //         line.fragments.remove(current_caret_col);
                //         self.needs_render = true;
                //         return Ok(());
                //     }
                //     // Delete at the end of the line should merge with the next line
                //     if current_caret_line != last_line_index && current_caret_col == last_grapheme {
                //         // Split off all lines after this current line first
                //         let next_line_onwards = self.buffer.lines.split_off(next_line_index);
                //         // Get the current line
                //         let line = match self.buffer.lines.get_mut(current_caret_line) {
                //             Some(line) => line,
                //             None => return Ok(()),
                //         };
                //         match next_line_onwards.first() {
                //             // This is the next line
                //             Some(next_line) => {
                //                 line.fragments.extend_from_slice(&next_line.fragments);
                //                 // Append the rest of the lines after the next line
                //                 self.buffer.lines.extend_from_slice(&next_line_onwards);
                //                 // Delete the next line
                //                 self.buffer.lines.remove(next_line_index);
                //                 self.needs_render = true;
                //                 return Ok(());
                //             }
                //             None => {}
                //         };
                //     }
                // }
            }
            SpecialKey::Backspace => {
                let line = match self.buffer.lines.get_mut(current_caret_line) {
                    Some(line) => line,
                    None => return Ok(()),
                };
                // Normal backspace within a line
                if current_caret_col != 0 {
                    line.fragments.remove(current_caret_col.saturating_sub(1));
                    self.move_caret_to_location(Direction::Left)?;
                    self.needs_render = true;
                    return Ok(());
                }
                // Top left of the document should do nothing
                if current_caret_line == 0 && current_caret_col == 0 {
                    self.needs_render = true;
                    return Ok(());
                }
                if current_caret_line != 0 && current_caret_col == 0 {
                    let mut fragments_to_move = line.fragments.split_off(0);
                    // Merge with previous line
                    let previous_line_index = current_caret_line.saturating_sub(1);
                    if let Some(previous_line) = self.buffer.lines.get_mut(previous_line_index) {
                        previous_line.fragments.append(&mut fragments_to_move);
                    }
                    // Delete the current line
                    self.buffer.lines.remove(current_caret_line);
                    self.location.line_index = previous_line_index;
                    self.location.grapheme_index = match self.buffer.lines.get(previous_line_index)
                    {
                        Some(prev_line) => prev_line.fragments.len(),
                        None => 0,
                    };
                    self.needs_render = true;
                    return Ok(());
                }
            }
            SpecialKey::Insert => todo!(),
            SpecialKey::CapsLock => todo!(),
        }
        Ok(())
    }

    // pub fn typing(&mut self, command: TerminalCommand) -> io::Result<()> {
    //     match command {
    //         TerminalCommand::OrdinaryChar(key_code) => {
    //             match key_code.as_char() {
    //                 Some(c) => {
    //                     let current_caret_line = self.location.line_index;
    //                     let current_caret_col = self.location.grapheme_index;
    //
    //                     match self.buffer.lines.get_mut(current_caret_line) {
    //                         Some(line) => {
    //                             // Split the fragments after the caret position out
    //                             // TODO: Handle a bug that inserts at the end of the line with incorrect index
    //                             let mut fragments_after_caret =
    //                                 line.fragments.split_off(current_caret_col);
    //                             // Insert the new character as a fragment at the caret position
    //                             line.fragments.push(TextFragment::from(c));
    //                             // Re-append the fragments after the caret position
    //                             line.fragments.append(&mut fragments_after_caret);
    //                             // Move the caret right after the inserted character
    //                             let _ = self.move_caret_to_location(Direction::Right);
    //                             self.needs_render = true;
    //
    //                             eprintln!(
    //                                 "Inserted char '{c}' at line {current_caret_line}, col {current_caret_col}"
    //                             );
    //                             eprintln!(
    //                                 "Line after insertion: {:?}",
    //                                 self.buffer
    //                                     .lines
    //                                     .get(current_caret_line)
    //                                     .unwrap()
    //                                     .fragments
    //                                     .iter()
    //                                     .map(|fragment| &fragment.grapheme)
    //                                     .collect::<Vec<&String>>()
    //                             );
    //                         }
    //                         None => {}
    //                     };
    //                 }
    //                 None => {
    //                     eprintln!("KeyCode is not a character: {:?}", key_code);
    //                 }
    //             }
    //         }
    //         TerminalCommand::SpecialKey(key_code) => {
    //             // TODO: Handle special keys like Backspace, Delete, Enter, etc.
    //         }
    //         _ => {
    //             return Ok(());
    //         }
    //     };
    //
    //     Ok(())
    // }
}

impl View for Terminal {
    fn terminate(&self) -> io::Result<()> {
        self.queue_command(terminal::LeaveAlternateScreen)?;
        self.show_caret()?;
        self.flush()?;
        terminal::disable_raw_mode()
    }

    fn move_caret_to_location(&mut self, direction: Direction) -> io::Result<()> {
        let Size { height, .. } = self.size()?;
        if let Some(curr_line) = self.buffer.lines.get(self.location.line_index) {
            let (row, col) = (self.location.line_index, self.location.grapheme_index);
            match direction {
                Direction::Up => {
                    // Move up within the document
                    self.location.line_index = if row > 0 {
                        let line_index = self.location.line_index.saturating_sub(1);

                        // If the current column is beyond the longest column in the new line,
                        // adjust it to the end of that line
                        let longest_col_in_line = match self.buffer.lines.get(line_index) {
                            Some(line) => line.graphemes_width(),
                            None => 0,
                        };
                        if self.location.grapheme_index > longest_col_in_line {
                            self.location.grapheme_index = longest_col_in_line;
                        }
                        line_index
                    } else {
                        // Stay at the top if already at row
                        0
                    }
                }
                Direction::Down => {
                    // Move down within the document
                    if row < self.buffer.line_count().saturating_sub(1) {
                        self.location.line_index = row.saturating_add(1);
                        // If the current column is beyond the longest column in the new line,
                        // adjust it to the end of that line
                        let longest_col_in_line =
                            match self.buffer.lines.get(self.location.line_index) {
                                Some(line) => line.graphemes_width(),
                                None => 0,
                            };
                        if self.location.grapheme_index > longest_col_in_line {
                            self.location.grapheme_index = longest_col_in_line;
                        }
                    }
                }
                Direction::Left => {
                    // Move left within the current line
                    if col > 0 {
                        self.location.grapheme_index = col.saturating_sub(1);
                    }
                    // Move to end of previous line if at beginning of current line
                    if col == 0 && row > 0 {
                        self.location.line_index = row.saturating_sub(1);
                        if let Some(prev_line) = self.buffer.lines.get(self.location.line_index) {
                            self.location.grapheme_index =
                                prev_line.fragments.len().saturating_sub(1);
                        }
                    }
                }
                Direction::Right => {
                    // Move right within the current line
                    if col < curr_line.graphemes_width().saturating_sub(1) {
                        self.location.grapheme_index = col.saturating_add(1);
                    }
                    // Move to beginning of next line if at end of current line
                    if col >= curr_line.graphemes_width().saturating_sub(1) {
                        if let Some(_) = self.buffer.lines.get(row.saturating_add(1)) {
                            self.location.line_index = row.saturating_add(1);
                            self.location.grapheme_index = 0;
                        }
                    }
                }
                Direction::PageUp => {
                    // Move up by one page, but do not exceed top of document
                    if row < height {
                        self.location.line_index = 0;
                    } else {
                        self.location.line_index = row.saturating_sub(height);
                    }
                }
                Direction::PageDown => {
                    // Move down by one page, but do not exceed buffer line count
                    if self.buffer.line_count() > row.saturating_add(height) {
                        self.location.line_index = row.saturating_add(height);
                    } else {
                        self.location.line_index = self.buffer.line_count().saturating_sub(1);
                    }
                }
                Direction::Home => {
                    self.location.grapheme_index = 0;
                }
                Direction::End => {
                    self.location.grapheme_index = curr_line.fragments.len() - 1;
                }
            }
        };

        self.scroll_location_into_view();
        self.flush()?;
        Ok(())
    }

    fn move_caret_to_position(&self, position: Position) -> io::Result<()> {
        self.queue_command(cursor::MoveTo(position.x as u16, position.y as u16))?
            .flush()?;
        Ok(())
    }

    fn print(&self, message: &str) -> io::Result<()> {
        self.queue_command(style::Print(message))?.flush()?;
        Ok(())
    }

    fn hide_caret(&self) -> io::Result<()> {
        self.queue_command(cursor::Hide)?.flush()?;
        Ok(())
    }

    fn show_caret(&self) -> io::Result<()> {
        self.queue_command(cursor::Show)?.flush()?;
        Ok(())
    }

    fn flush(&self) -> io::Result<()> {
        stdout().flush()
    }

    fn clear_screen(&self) -> io::Result<()> {
        self.queue_command(Clear(terminal::ClearType::All))?.flush()
    }

    fn clear_line(&self) -> io::Result<()> {
        self.queue_command(Clear(terminal::ClearType::CurrentLine))?
            .flush()?;
        Ok(())
    }

    /// render renders the current view of the buffer to the terminal.
    /// row 0
    /// row 1
    /// row 2  ┌───────────────┐ ← view top (offset_row)
    /// row 3  │               │
    /// row 4  └───────────────┘ ← view bottom (offset_row + height - 1)
    /// row 5
    /// row 6
    fn render(&mut self) -> io::Result<()> {
        if !self.needs_render {
            return Ok(());
        }
        let Size { width, height } = self.size()?;
        if width == 0 || width == 0 {
            return Ok(());
        }
        let top = self.scroll_offset.line_index;
        for view_row in 0..height {
            let abs_view_row = view_row.saturating_add(top);
            match self.buffer.lines.get(abs_view_row) {
                Some(line) => {
                    let left = self.scroll_offset.grapheme_index;
                    let right = if left.saturating_add(width) > line.graphemes_width() {
                        line.graphemes_width()
                    } else {
                        left.saturating_add(width)
                    };
                    let content_in_view = line.get_visible_graphemes(left..right);
                    self.move_caret_to_position(Position { x: 0, y: view_row })?;
                    self.clear_line()?;
                    self.print(content_in_view.as_str())?;
                }
                None => {
                    // Show the welcome message if we're at 1/3rd of the screen height
                    // and the buffer is empty
                    if view_row == height / 3 && self.buffer.line_count() == 0 {
                        let welcome_mesage = Self::build_welcome_message(width)?;
                        self.move_caret_to_position(Position { x: 0, y: view_row })?;
                        self.clear_line()?;
                        self.print(welcome_mesage.as_str())?;
                    } else {
                        self.move_caret_to_position(Position { x: 0, y: view_row })?;
                        self.clear_line()?;
                        self.print("~")?;
                    }
                }
            }
        }
        self.needs_render = false;

        Ok(())
    }

    fn resize(&mut self, to: Size) {
        self.size = to;
        self.scroll_location_into_view();
        self.needs_render = true;
    }

    fn size(&self) -> io::Result<Size> {
        Ok(self.size)
    }

    fn handle_command(&mut self, command: TerminalCommand) -> io::Result<()> {
        match command {
            TerminalCommand::MoveCaret(direction) => match self.move_caret_to_location(direction) {
                Ok(_) => Ok(()),
                Err(err) => Err(err),
            },
            TerminalCommand::OrdinaryChar(key_code) => {
                let c = key_code.as_char();
                match self.handle_ordinary_typing(c) {
                    Ok(_) => Ok(()),
                    Err(_) => Ok(()), // Just ignore the error for now
                }
            }
            TerminalCommand::SpecialKey(key_code) => {
                match self.handle_special_key(key_code) {
                    Ok(_) => Ok(()),
                    Err(_) => Ok(()), // Just ignore the error for now
                }
            }
            // TerminalCommand::InsertChar(char) => {
            //     match self.handle_ordinary_typing(Some(char)) {
            //         Ok(_) => Ok(()),
            //         Err(_) => Ok(()), // Just ignore the error for now
            //     }
            // }
            TerminalCommand::FunctionKey(n) => Ok(()),
            TerminalCommand::Resize(size) => Ok(self.resize(size)),
            _ => Ok(()),
        }
    }

    fn evaluate_keypress<F>(&mut self, mut action: F) -> io::Result<()>
    where
        F: FnMut(EditorCommand),
    {
        let (event, should_proceed) = match read() {
            Ok(event) => match event {
                Event::Key(KeyEvent { kind, .. }) if kind == KeyEventKind::Press => (event, true),
                Event::Resize(_, _) => (event, true),
                _ => (event, false),
            },
            Err(err) => return Err(err),
        };

        if !should_proceed {
            return Ok(());
        }

        match TerminalCommand::try_from(event) {
            Ok(command) if matches!(command, TerminalCommand::Quit) => {
                self.terminate()?;
                action(EditorCommand::Quit);
            }
            Ok(command) if !matches!(command, TerminalCommand::Unknown) => {
                self.handle_command(command)?;
            }
            Ok(_) => {}
            Err(err) => {
                #[cfg(debug_assertions)]
                {
                    panic!("Could not handle command: {err}");
                }
            }
        }

        Ok(())
    }

    fn get_position(&mut self) -> io::Result<Position> {
        Ok(self.location.to_position(self.scroll_offset))
    }
}
