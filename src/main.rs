pub(crate) mod editor;
pub(crate) mod view;

use crate::editor::Editor;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut file_content = String::new();
    if let Some(arg1) = args.get(1) {
        match std::fs::read_to_string(arg1) {
            Ok(content) => {
                file_content = content;
            }
            Err(e) => {
                eprintln!("Error reading file {}: {}", arg1, e);
            }
        }
    }

    let terminal = view::terminal::Terminal::new(file_content);
    let mut editor = Editor::new(terminal);
    if let Err(e) = editor.run() {
        eprintln!("Error: {e}");
    }
}
