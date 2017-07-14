#[macro_use] extern crate error_chain;
extern crate x11;

#[macro_use] pub mod error;

use std::ptr;
use std::ffi::CString;
use x11::xlib::{ self, Atom, Window };


#[derive(Clone, Debug)]
pub struct Atoms {
    pub primary: Atom,
    pub clipboard: Atom,
    pub property: Atom,
    pub targets: Atom,
    pub string: Atom,
    pub utf8_string: Atom,
    pub incr: Atom
}

pub struct Context {
    pub connection: *mut xlib::Display,
    pub window: Window,
    pub atoms: Atoms
}

impl Context {
    pub fn new(name: Option<&str>) -> error::Result<Self> {
        let display = if let Some(name) = name {
            let c_display_name = CString::new(name)?;
            unsafe { xlib::XOpenDisplay(c_display_name.as_ptr()) }
        } else {
            unsafe { xlib::XOpenDisplay(ptr::null()) }
        };
        if display.is_null() { return Err(err!(XConnection)) };

        let window = unsafe {
            let black = xlib::XBlackPixel(display, xlib::XDefaultScreen(display));
            xlib::XCreateSimpleWindow(
                display,
                xlib::XDefaultRootWindow(display),
                0, 0, 1, 1,
                0, black, black
            )
        };

        macro_rules! intern_atom {
            ( $name:expr ) => {
                unsafe { xlib::XInternAtom(
                    display,
                    concat!($name, '\0').as_ptr() as _,
                    xlib::False
                ) }
            }
        }

        let atoms = Atoms {
            primary: xlib::XA_PRIMARY,
            clipboard: intern_atom!("CLIPBOARD"),
            property: intern_atom!("THIS_CLIPBOARD_OUT"),
            targets: intern_atom!("TARGETS"),
            string: xlib::XA_STRING,
            utf8_string: intern_atom!("UTF8_STRING"),
            incr: intern_atom!("INCR")
        };

        Ok(Context { connection: display, window, atoms })
    }

    pub fn get_atom(&self, name: &str) -> error::Result<Atom> {
        let c_atom_name = CString::new(name)?;
        Ok(unsafe { xlib::XInternAtom(
            self.connection,
            c_atom_name.as_ptr(),
            xlib::False
        ) })
    }
}
