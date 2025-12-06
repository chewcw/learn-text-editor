use crate::view::Size;
use crossterm::event::{Event as CrossTermEvent, KeyCode, KeyEvent, KeyModifiers};

pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
}

pub enum TerminalCommand {
    MoveCaret(Direction),
    Resize(Size),
    Quit,
}

impl TryFrom<CrossTermEvent> for TerminalCommand {
    type Error = String;

    fn try_from(event: CrossTermEvent) -> Result<Self, Self::Error> {
        match event {
            CrossTermEvent::Key(KeyEvent {
                code, modifiers, ..
            }) => match code {
                KeyCode::Left => Ok(Self::MoveCaret(Direction::Left)),
                KeyCode::Right => Ok(Self::MoveCaret(Direction::Right)),
                KeyCode::Up => Ok(Self::MoveCaret(Direction::Up)),
                KeyCode::Down => Ok(Self::MoveCaret(Direction::Down)),
                KeyCode::Home => Ok(Self::MoveCaret(Direction::Home)),
                KeyCode::End => Ok(Self::MoveCaret(Direction::End)),
                KeyCode::PageUp => Ok(Self::MoveCaret(Direction::PageUp)),
                KeyCode::PageDown => Ok(Self::MoveCaret(Direction::PageDown)),
                KeyCode::Char('q') if modifiers.contains(KeyModifiers::CONTROL) => Ok(Self::Quit),
                _ => Err(format!("Unsupported key event for EditorCommand: {code:?}").to_string()),
            },
            CrossTermEvent::Resize(width_u16, height_u16) => {
                let height = height_u16 as usize;
                let width = width_u16 as usize;
                Ok(Self::Resize(Size { width, height }))
            }
            _ => Err(format!("Unsupported event for EditorCommand: {event:?}").to_string()),
        }
    }
}
