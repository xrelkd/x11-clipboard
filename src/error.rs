error_chain!{
    foreign_links {
        Io(::std::io::Error);
        Nul(::std::ffi::NulError);
        Utf8(::std::string::FromUtf8Error);
        Sender(::std::sync::mpsc::SendError<::x11::xlib::Atom>);
    }

    errors {
        Lock {
            description("store lock poison")
        }
        XConnection {
            description("X Connection Error")
        }
        BadTarget {
            description("Bad Target")
        }
        BadOwner {
            description("Bad selection owner")
        }
        Timeout {
            description("Load selection timeout")
        }
    }
}

macro_rules! err {
    ( $kind:ident ) => {
        $crate::error::Error::from($crate::error::ErrorKind::$kind)
    };
    ( $kind:ident, $err:expr ) => {
        $crate::error::Error::from($crate::error::ErrorKind::$kind($err))
    };
}
