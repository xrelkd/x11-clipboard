error_chain!{
    foreign_links {
        Io(::std::io::Error);
        Nul(::std::ffi::NulError);
        Utf8(::std::string::FromUtf8Error);
    }

    errors {
        XConnection {
            description("X Connection Error")
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
