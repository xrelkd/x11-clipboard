use std::{ ptr, fmt };
use std::sync::Arc;
use std::ffi::CString;
use x11::xlib::{ self, Atom, Window };
use ::error;


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

pub struct Connection {
    pub display: *mut xlib::Display,
    pub window: Window,
    pub atoms: Atoms
}

pub struct UnsafeConnection(pub Arc<Connection>);

unsafe impl Send for UnsafeConnection {}
unsafe impl Sync for UnsafeConnection {}

impl Connection {
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

        unsafe { xlib::XSelectInput(display, window, xlib::PropertyChangeMask) };

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

        Ok(Connection { display, window, atoms })
    }

    pub fn get_atom(&self, name: &str) -> error::Result<Atom> {
        unsafe {
            let c_atom_name = CString::new(name)?;
            Ok(xlib::XInternAtom(
                self.display,
                c_atom_name.as_ptr(),
                xlib::False
            ))
        }
    }
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Connection")
            .field("display", &format_args!("{:p}", self.display))
            .field("window", &self.window)
            .field("atoms", &self.atoms)
            .finish()
    }
}
