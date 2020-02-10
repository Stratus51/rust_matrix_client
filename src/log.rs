pub const GUI: bool = false;
pub const NET_MATRIX: bool = true;

#[macro_export]
macro_rules! spec_dbg_log {
    ($cond:ident, $str: expr, $head: expr) => {
        if crate::log::$cond {
            eprintln!(concat!($str, $head));
        }
    };
    ($cond:ident, $str: expr, $head: expr, $($tts:tt)*) => {
        if crate::log::$cond {
            eprintln!(concat!($str, $head), $($tts)*);
        }
    }
}

#[macro_export]
macro_rules! gui_dbg {
    ($($tts:tt)*) => {
        crate::spec_dbg_log!(GUI, "GUI: ", $($tts)*);
    };
}

#[macro_export]
macro_rules! net_matrix_dbg {
    ($($tts:tt)*) => {
        crate::spec_dbg_log!(NET_MATRIX, "N_MAT: ", $($tts)*);
    };
}
