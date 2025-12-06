pub(crate) mod buffer;
pub(crate) mod editor_command;

use crate::{editor::editor_command::EditorCommand, view::View};
use std::{
    io::{self},
    panic::{set_hook, take_hook},
};

pub struct Editor<U: View> {
    ui: U,
    should_quit: bool,
}

impl<U> Editor<U>
where
    U: View + Sync + 'static + Send + Clone,
{
    pub fn new(ui: U) -> Self {
        let current_hook = take_hook();
        let ui_clone = ui.clone();
        set_hook(Box::new(move |panic_info| {
            let _ = ui_clone.terminate();
            current_hook(panic_info);
        }));
        Editor {
            ui,
            should_quit: false,
        }
    }

    fn refresh_screen(&mut self) -> io::Result<()> {
        let position = self.ui.get_position()?;
        self.ui.hide_caret()?;
        self.ui.render()?;
        self.ui.move_caret_to_position(position)?;
        self.ui.show_caret()?;
        self.ui.flush()?;
        Ok(())
    }

    pub fn run(&mut self) -> io::Result<()> {
        loop {
            self.refresh_screen()?;
            if self.should_quit {
                break;
            }
            self.ui.evaluate_keypress(|command| match command {
                EditorCommand::Quit => {
                    self.should_quit = true;
                }
            })?;
        }
        self.ui.terminate()?;
        Ok(())
    }
}

impl<U: View> Drop for Editor<U> {
    fn drop(&mut self) {
        let _ = self.ui.terminate();
        if self.should_quit {
            let _ = self.ui.print("Goodbye.\r\n");
            return;
        }
        eprintln!("Editor crashed unexpectedly. Terminal state restored.");
    }
}
