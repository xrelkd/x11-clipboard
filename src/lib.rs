#[macro_use] extern crate error_chain;
extern crate x11;

#[macro_use] pub mod error;
mod connection;
mod context;

use std::{ cmp, mem, slice, thread };
use std::time::{ Duration, Instant };
use std::sync::{ Arc, RwLock };
use std::sync::mpsc::{ Sender, channel };
use std::collections::HashMap;
use x11::xlib::{ self, Atom };
use connection::UnsafeConnection;
pub use connection::{ Atoms, Connection };


pub const INCR_CHUNK_SIZE: usize = 4000;
pub const POLL_DURATION: u64 = 50;
type SetMap = Arc<RwLock<HashMap<Atom, (Atom, Vec<u8>)>>>;


pub struct Clipboard {
    pub getter: Connection,
    pub setter: Arc<Connection>,
    setmap: SetMap,
    sender: Sender<Atom>
}

impl Clipboard {
    pub fn new() -> error::Result<Self> {
        let getter = Connection::new(None)?;
        let setter = Arc::new(Connection::new(None)?);
        let setter2 = UnsafeConnection(setter.clone());
        let setmap: SetMap = Arc::new(RwLock::new(HashMap::new()));
        let setmap2 = setmap.clone();

        let (sender, receiver) = channel();
        let max = unsafe { xlib::XMaxRequestSize(setter.display) as _ };
        let max = (cmp::max(65536, max) << 2) - 100;

        thread::spawn(move || context::run(setter2, setmap2, max, receiver));

        Ok(Clipboard { getter, setter, setmap, sender })
    }

    pub fn load<T>(&self, selection: Atom, target: Atom, property: Atom, timeout: T)
        -> error::Result<Vec<u8>>
        where T: Into<Option<Duration>>
    {
        let mut buf = Vec::new();
        let mut is_incr = false;
        let timeout = timeout.into();
        let start_time = timeout.as_ref()
            .map(|_| Instant::now());

        unsafe {
            xlib::XConvertSelection(
                self.getter.display,
                selection, target, property,
                self.getter.window,
                xlib::CurrentTime
                    // FIXME ^
                    // Clients should not use CurrentTime for the time argument of a ConvertSelection request.
                    // Instead, they should use the timestamp of the event that caused the request to be made.
            );
            xlib::XSync(self.getter.display, xlib::False);
        }

        loop {
            if timeout.iter()
                .zip(&start_time)
                .next()
                .map(|(&timeout, &time)| (Instant::now() - time) >= timeout)
                .unwrap_or(false)
            {
                return Err(err!(Timeout));
            }

            unsafe {
                let mut event = mem::uninitialized();

                if xlib::XPending(self.getter.display) > 0 {
                    xlib::XNextEvent(self.getter.display, &mut event);
                } else {
                    thread::park_timeout(Duration::from_millis(POLL_DURATION));
                    continue
                }

                match event.get_type() {
                    xlib::SelectionNotify => {
                        let event = xlib::XSelectionEvent::from(&event);
                        if event.selection != selection { continue };
                        if event.property == 0 { return Err(err!(BadTarget)) };

                        let (mut type_, mut format, mut length, mut bytesafter, mut value) =
                            mem::zeroed();

                        xlib::XGetWindowProperty(
                            event.display, event.requestor, event.property,
                            buf.len() as _, 1000000,
                            xlib::False, xlib::AnyPropertyType as _,
                            &mut type_, &mut format, &mut length, &mut bytesafter, &mut value
                        );

                        if type_ == self.getter.atoms.incr {
                            assert_eq!(format, 32);
                            if let Some(&size) = slice::from_raw_parts(value as *const i32, 1).get(0) {
                                buf.reserve(size as usize);
                            }

                            xlib::XDeleteProperty(event.display, event.requestor, event.property);
                            xlib::XSync(event.display, xlib::False);
                            xlib::XFree(value as _);

                            is_incr = true;
                            continue
                        } else if type_ != target {
                            continue
                        }

                        buf.extend_from_slice(slice::from_raw_parts(value as *const u8, length as _));
                        xlib::XFree(value as _);
                        break
                    },
                    xlib::PropertyNotify if is_incr => {
                        let event = xlib::XPropertyEvent::from(&event);
                        if event.state != xlib::PropertyNewValue { continue };

                        let (mut type_, mut format, mut length, mut bytesafter, mut value) =
                            mem::zeroed();

                        xlib::XGetWindowProperty(
                            self.getter.display, event.window, property,
                            0, 0,
                            xlib::False, target,
                            &mut type_, &mut format, &mut length, &mut bytesafter, &mut value
                        );

                        xlib::XGetWindowProperty(
                            self.getter.display, event.window, property,
                            0, bytesafter as _,
                            xlib::False, target,
                            &mut type_, &mut format, &mut length, &mut bytesafter, &mut value
                        );

                        if length != 0 {
                            buf.extend_from_slice(slice::from_raw_parts(value as *const u8, length as _));
                            xlib::XFree(value as _);
                        } else {
                            break
                        }
                    },
                    _ => ()
                }
            }
        }

        unsafe {
            xlib::XDeleteProperty(self.getter.display, self.getter.window, property);
            xlib::XSync(self.getter.display, xlib::False);
        }

        Ok(buf)
    }

    pub fn store<T>(&self, selection: Atom, target: Atom, value: T)
        -> error::Result<()>
        where T: Into<Vec<u8>>
    {
        self.sender.send(selection)?;
        self.setmap
            .write()
            .map_err(|_| err!(Lock))?
            .insert(selection, (target, value.into()));

        let owner = unsafe {
            xlib::XSetSelectionOwner(
                self.setter.display, selection,
                self.setter.window, xlib::CurrentTime
            );

            xlib::XGetSelectionOwner(self.setter.display, selection)
        };

        if owner == self.setter.window {
            Ok(())
        } else {
            Err(err!(BadOwner))
        }
    }
}


#[test]
fn it_work() {
    let data = format!("{:?}", Instant::now());
    let clipboard = Clipboard::new().unwrap();

    let atom_clipboard = clipboard.setter.atoms.clipboard;
    let atom_utf8string = clipboard.setter.atoms.utf8_string;
    let atom_property = clipboard.setter.atoms.property;

    clipboard.store(atom_clipboard, atom_utf8string, data.as_bytes()).unwrap();

    let output = clipboard.load(atom_clipboard, atom_utf8string, atom_property, None).unwrap();
    assert_eq!(output, data.as_bytes());

    let data = format!("{:?}", Instant::now());
    clipboard.store(atom_clipboard, atom_utf8string, data.as_bytes()).unwrap();

    let output = clipboard.load(atom_clipboard, atom_utf8string, atom_property, None).unwrap();
    assert_eq!(output, data.as_bytes());

    let output = clipboard.load(atom_clipboard, atom_utf8string, atom_property, None).unwrap();
    assert_eq!(output, data.as_bytes());
}
