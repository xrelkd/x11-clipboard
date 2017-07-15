extern crate x11_clipboard;

use std::io::{ self, Write };
use x11_clipboard::Clipboard;


fn main() {
    let clipboard = Clipboard::new().unwrap();
    let output = clipboard.load(
        clipboard.getter.atoms.clipboard,
        clipboard.getter.atoms.utf8_string,
        clipboard.getter.atoms.property,
        None
    ).unwrap();

    io::stdout().write_all(&output).unwrap();
}
