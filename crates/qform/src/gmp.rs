use core::ffi::{c_char, c_int, c_long, c_ulong};
use gmp_mpfr_sys::gmp;
use std::ffi::{CStr, CString};
use std::mem::MaybeUninit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseMpzError {
    InvalidInput,
    InvalidRadix,
    InteriorNul,
}

pub struct Mpz {
    raw: gmp::mpz_t,
}

impl Default for Mpz {
    fn default() -> Self {
        unsafe {
            let mut raw = MaybeUninit::<gmp::mpz_t>::uninit();
            gmp::mpz_init(raw.as_mut_ptr());
            Self {
                raw: raw.assume_init(),
            }
        }
    }
}

impl Clone for Mpz {
    fn clone(&self) -> Self {
        unsafe {
            let mut raw = MaybeUninit::<gmp::mpz_t>::uninit();
            gmp::mpz_init_set(raw.as_mut_ptr(), self.as_raw());
            Self {
                raw: raw.assume_init(),
            }
        }
    }
}

impl std::fmt::Debug for Mpz {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Mpz")
            .field(&self.to_string_radix(10))
            .finish()
    }
}

impl Drop for Mpz {
    fn drop(&mut self) {
        unsafe {
            gmp::mpz_clear(self.as_raw_mut());
        }
    }
}

impl Mpz {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_i64(value: i64) -> Self {
        let mut x = Self::new();
        x.set_i64(value);
        x
    }

    pub fn from_u64(value: u64) -> Self {
        let mut x = Self::new();
        x.set_u64(value);
        x
    }

    pub fn from_str_radix(value: &str, radix: u32) -> Result<Self, ParseMpzError> {
        let mut x = Self::new();
        x.set_str_radix(value, radix)?;
        Ok(x)
    }

    pub fn set_i64(&mut self, value: i64) {
        unsafe {
            gmp::mpz_set_si(self.as_raw_mut(), value as c_long);
        }
    }

    pub fn set_u64(&mut self, value: u64) {
        unsafe {
            gmp::mpz_set_ui(self.as_raw_mut(), value as c_ulong);
        }
    }

    pub fn set_str_radix(&mut self, value: &str, radix: u32) -> Result<(), ParseMpzError> {
        if !(2..=36).contains(&radix) {
            return Err(ParseMpzError::InvalidRadix);
        }
        let c_value = CString::new(value).map_err(|_| ParseMpzError::InteriorNul)?;
        let rc = unsafe { gmp::mpz_set_str(self.as_raw_mut(), c_value.as_ptr(), radix as c_int) };
        if rc == 0 {
            Ok(())
        } else {
            Err(ParseMpzError::InvalidInput)
        }
    }

    pub fn to_string_radix(&self, radix: u32) -> String {
        assert!((2..=36).contains(&radix), "radix must be in 2..=36");
        unsafe {
            let digits = gmp::mpz_sizeinbase(self.as_raw(), radix as c_int) as usize + 3;
            let mut buf = vec![0 as c_char; digits];
            let ptr = gmp::mpz_get_str(buf.as_mut_ptr(), radix as c_int, self.as_raw());
            CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    }

    fn as_raw(&self) -> gmp::mpz_srcptr {
        &self.raw as *const gmp::mpz_t
    }

    fn as_raw_mut(&mut self) -> gmp::mpz_ptr {
        &mut self.raw as *mut gmp::mpz_t
    }
}
