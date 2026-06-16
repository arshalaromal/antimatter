use std::ffi::c_void;

/// C-ABI compatible Outcome struct.
/// This will be linked to the QBE output.
#[repr(C)]
pub struct Outcome {
    pub status: i32,            // 0 = None, 1 = Ok, 2 = Error
    pub value: *mut c_void,     // Raw pointer to the successful data
    pub error: *mut c_void,     // Raw pointer to the error string/data
}

impl Outcome {

    // CONSTRUCTORS
    pub fn none() -> Self {
        Outcome {
            status: 0,
            value: std::ptr::null_mut(),
            error: std::ptr::null_mut(),
        }
    }

    pub fn ok(val: *mut c_void) -> Self {
        Outcome {
            status: 1,
            value: val,
            error: std::ptr::null_mut(),
        }
    }

    pub fn error(err: *mut c_void) -> Self {
        Outcome {
            status: 2,
            value: std::ptr::null_mut(),
            error: err,
        }
    }
    
    
    // CHECKS
    pub fn is_none(&self) -> bool {
        self.status == 0
    }

    pub fn is_ok(&self) -> bool {
        self.status == 1
    }

    pub fn is_error(&self) -> bool {
        self.status == 2
    }
}