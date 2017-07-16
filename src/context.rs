use std::{ cmp, mem, ptr };
use std::sync::mpsc::Receiver;
use std::collections::HashMap;
use x11::xlib;
use ::connection::UnsafeConnection;
use ::{ INCR_CHUNK_SIZE, SetMap, Atom };


macro_rules! try_continue {
    ( $expr:expr ) => {
        match $expr {
            Some(val) => val,
            None => continue
        }
    };
}

struct IncrState {
    selection: Atom,
    requestor: Atom,
    property: Atom,
    pos: usize
}

pub fn run(UnsafeConnection(conn): UnsafeConnection, setmap: SetMap, max: usize, receiver: Receiver<Atom>) {
    let mut incr_map = HashMap::new();
    let mut state_map = HashMap::new();

    loop {
        unsafe {
            let mut event = mem::zeroed();
            xlib::XNextEvent(conn.display, &mut event);

            if let Some(property) = receiver
                .try_recv().ok()
                .and_then(|selection| incr_map.remove(&selection))
            {
                state_map.remove(&property);
            }

            match event.get_type() {
                0 => break,
                xlib::SelectionRequest => {
                    let event = xlib::XSelectionRequestEvent::from(&event);
                    let read_map = try_continue!(setmap.read().ok());
                    let &(target, ref value) = try_continue!(read_map.get(&event.selection));

                    if event.target == conn.atoms.targets {
                        xlib::XChangeProperty(
                            conn.display, event.requestor, event.property,
                            xlib::XA_ATOM, 32, xlib::PropModeReplace,
                            [conn.atoms.targets, target].as_ptr() as _, 2
                        );
                    } else if event.target == target {
                        if value.len() < max {
                            xlib::XChangeProperty(
                                conn.display, event.requestor, event.property,
                                target, 8, xlib::PropModeReplace,
                                value.as_ptr(), value.len() as _
                            );
                        } else {
                            xlib::XSelectInput(conn.display, event.requestor, xlib::PropertyChangeMask);
                            xlib::XChangeProperty(
                                conn.display, event.requestor, event.property,
                                conn.atoms.incr, 32, xlib::PropModeReplace,
                                ptr::null(), 0
                            );

                            incr_map.insert(event.selection, event.property);
                            state_map.insert(
                                event.property,
                                IncrState {
                                    selection: event.selection,
                                    requestor: event.requestor,
                                    property: event.property,
                                    pos: 0
                                }
                            );
                        }
                    } else {
                        continue
                    }

                    let mut ev = xlib::XEvent::from(xlib::XSelectionEvent {
                        type_: xlib::SelectionNotify,
                        display: event.display,
                        requestor: event.requestor,
                        selection: event.selection,
                        property: event.property,
                        target: event.target,
                        time: event.time,
                        serial: mem::uninitialized(),
                        send_event: mem::uninitialized()
                    });
                    xlib::XSendEvent(
                        conn.display, event.requestor, xlib::False, 0,
                        &mut ev
                    );
                    xlib::XSync(conn.display, xlib::False);
                },
                xlib::PropertyNotify => {
                    let event = xlib::XPropertyEvent::from(&event);
                    if event.state != xlib::PropertyDelete { continue };

                    let eof = {
                        let state = try_continue!(state_map.get_mut(&event.atom));
                        let read_setmap = try_continue!(setmap.read().ok());
                        let &(target, ref value) = try_continue!(read_setmap.get(&state.selection));

                        let len = cmp::min(INCR_CHUNK_SIZE, value.len() - state.pos);
                        xlib::XChangeProperty(
                            conn.display, state.requestor, state.property,
                            target, 8, xlib::PropModeReplace,
                            value[state.pos..].as_ptr(), len as _
                        );
                        state.pos += len;

                        len == 0
                    };

                    if eof {
                        state_map.remove(&event.atom);
                    }

                    xlib::XSync(conn.display, xlib::False);
                },
                xlib::SelectionClear => {
                    let event = xlib::XSelectionClearEvent::from(&event);
                    if let Some(property) = incr_map.remove(&event.selection) {
                        state_map.remove(&property);
                    }
                    if let Ok(mut write_setmap) = setmap.write() {
                        write_setmap.remove(&event.selection);
                    }
                },
                _ => ()
            }
        }
    }
}
