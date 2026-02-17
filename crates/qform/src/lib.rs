use optarith::raw as opt_raw;
use qform_sys::*;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_long};

pub mod raw {
    pub use qform_sys::*;
}

#[cfg(feature = "gmp")]
pub mod gmp;
#[cfg(feature = "gmp")]
pub use gmp::{Mpz, ParseMpzError};

unsafe extern "C" {
    #[link_name = "__gmpz_init"]
    fn gmpz_init(x: *mut __mpz_struct);
    #[link_name = "__gmpz_clear"]
    fn gmpz_clear(x: *mut __mpz_struct);
    #[link_name = "__gmpz_set_si"]
    fn gmpz_set_si(x: *mut __mpz_struct, val: c_long);
    #[link_name = "__gmpz_set_str"]
    fn gmpz_set_str(x: *mut __mpz_struct, s: *const c_char, base: c_int) -> c_int;
    #[link_name = "__gmpz_sizeinbase"]
    fn gmpz_sizeinbase(x: *const __mpz_struct, base: c_int) -> usize;
    #[link_name = "__gmpz_get_str"]
    fn gmpz_get_str(out: *mut c_char, base: c_int, x: *const __mpz_struct) -> *mut c_char;
    fn group_pow_init(pow: *mut group_pow_t, group: *mut group_t);
    fn group_pow_clear(pow: *mut group_pow_t);
}

fn i128_to_s128(value: i128) -> s128_t {
    s128_t {
        v0: value as u64,
        v1: (value >> 64) as i64,
    }
}

fn s128_to_i128(value: s128_t) -> i128 {
    ((value.v1 as i128) << 64) | (value.v0 as i128)
}

fn mpz_to_string(value: *const __mpz_struct) -> Option<String> {
    unsafe {
        let len = gmpz_sizeinbase(value, 10) + 3;
        let mut buf = vec![0_u8; len];
        if gmpz_get_str(buf.as_mut_ptr() as *mut c_char, 10, value).is_null() {
            return None;
        }
        CStr::from_ptr(buf.as_ptr() as *const c_char)
            .to_str()
            .ok()
            .map(str::to_owned)
    }
}

fn with_mpz_str<T>(value: &str, f: impl FnOnce(*const __mpz_struct) -> T) -> Option<T> {
    let Ok(c_value) = CString::new(value) else {
        return None;
    };
    unsafe {
        let mut tmp: mpz_t = std::mem::zeroed();
        gmpz_init(tmp.as_mut_ptr());
        let ok = gmpz_set_str(tmp.as_mut_ptr(), c_value.as_ptr(), 10) == 0;
        let out = if ok { Some(f(tmp.as_ptr())) } else { None };
        gmpz_clear(tmp.as_mut_ptr());
        out
    }
}

fn split_ambiguous_str(
    n: &str,
    f: impl FnOnce(*mut __mpz_struct, *const __mpz_struct) -> c_int,
) -> Option<String> {
    with_mpz_str(n, |n_ptr| unsafe {
        let mut d: mpz_t = std::mem::zeroed();
        gmpz_init(d.as_mut_ptr());
        let ok = f(d.as_mut_ptr(), n_ptr) != 0;
        let out = if ok { mpz_to_string(d.as_ptr()) } else { None };
        gmpz_clear(d.as_mut_ptr());
        out
    })?
}

unsafe fn with_group_pow<T>(group: *mut group_t, f: impl FnOnce(*mut group_pow_t) -> T) -> T {
    struct PowGuard {
        pow: group_pow_t,
    }

    impl Drop for PowGuard {
        fn drop(&mut self) {
            unsafe { group_pow_clear(&mut self.pow) };
        }
    }

    let mut guard = PowGuard {
        pow: unsafe { std::mem::zeroed() },
    };
    unsafe { group_pow_init(&mut guard.pow, group) };
    f(&mut guard.pow)
}

unsafe fn clear_mpz_qform_raw(form: &mut mpz_qform_t) {
    unsafe {
        gmpz_clear(form.a.as_mut_ptr());
        gmpz_clear(form.b.as_mut_ptr());
        gmpz_clear(form.c.as_mut_ptr());
    }
}

fn validate_prime_index(prime_index: i32) -> Option<i32> {
    if prime_index < 0 {
        return None;
    }
    let count = unsafe { opt_raw::prime_list_count };
    if (prime_index as u32) >= count {
        None
    } else {
        Some(prime_index)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct S64Form {
    pub a: i32,
    pub b: i32,
    pub c: i64,
}

impl From<s64_qform_t> for S64Form {
    fn from(value: s64_qform_t) -> Self {
        Self {
            a: value.a,
            b: value.b,
            c: value.c,
        }
    }
}

impl From<S64Form> for s64_qform_t {
    fn from(value: S64Form) -> Self {
        Self {
            a: value.a,
            b: value.b,
            c: value.c,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct S128Form {
    pub a: i64,
    pub b: i64,
    pub c: i128,
}

impl From<s128_qform_t> for S128Form {
    fn from(value: s128_qform_t) -> Self {
        Self {
            a: value.a,
            b: value.b,
            c: s128_to_i128(value.c),
        }
    }
}

impl From<S128Form> for s128_qform_t {
    fn from(value: S128Form) -> Self {
        Self {
            a: value.a,
            b: value.b,
            c: i128_to_s128(value.c),
        }
    }
}

pub struct S64Group {
    raw: s64_qform_group_t,
}

impl S64Group {
    fn ptr(&self) -> *mut s64_qform_group_t {
        &self.raw as *const _ as *mut _
    }

    fn qgroup_ptr(&self) -> *mut qform_group_t {
        unsafe { &mut (*self.ptr()).desc }
    }

    fn group_ptr(&self) -> *mut group_t {
        unsafe { &mut (*self.qgroup_ptr()).group }
    }

    pub fn new(discriminant: i64) -> Option<Self> {
        let mut raw: s64_qform_group_t = unsafe { std::mem::zeroed() };
        unsafe {
            s64_qform_group_init(&mut raw);
            s64_qform_group_set_discriminant_s64(&mut raw, discriminant);
        }
        Some(Self { raw })
    }

    pub fn set_discriminant(&self, discriminant: i64) {
        unsafe { s64_qform_group_set_discriminant_s64(self.ptr(), discriminant) };
    }

    pub fn set_discriminant_str(&self, discriminant: &str) -> bool {
        with_mpz_str(discriminant, |d| unsafe {
            s64_qform_group_set_discriminant(self.ptr(), d);
            true
        })
        .unwrap_or(false)
    }

    #[cfg(feature = "gmp")]
    pub fn set_discriminant_mpz(&self, discriminant: &Mpz) -> bool {
        let d = discriminant.to_string_radix(10);
        self.set_discriminant_str(&d)
    }

    pub fn identity(&self) -> S64Form {
        let mut out: s64_qform_t = unsafe { std::mem::zeroed() };
        unsafe { s64_qform_set_id(self.ptr(), &mut out) };
        out.into()
    }

    pub fn init_form(&self, form: &mut S64Form) {
        let mut raw = s64_qform_t::from(*form);
        unsafe { s64_qform_init(self.ptr(), &mut raw) };
        *form = raw.into();
    }

    pub fn clear_form(&self, form: &mut S64Form) {
        let mut raw = s64_qform_t::from(*form);
        unsafe { s64_qform_clear(self.ptr(), &mut raw) };
        *form = raw.into();
    }

    pub fn hash32(&self, form: &S64Form) -> u32 {
        let raw = s64_qform_t::from(*form);
        unsafe { s64_qform_hash32(self.ptr(), &raw) }
    }

    pub fn is_id(&self, form: &S64Form) -> bool {
        let raw = s64_qform_t::from(*form);
        unsafe { s64_qform_is_id(self.ptr(), &raw) != 0 }
    }

    pub fn is_ambiguous(&self, form: &S64Form) -> bool {
        let raw = s64_qform_t::from(*form);
        unsafe { s64_qform_is_ambiguous(self.ptr(), &raw) != 0 }
    }

    pub fn equal(&self, a: &S64Form, b: &S64Form) -> bool {
        let raw_a = s64_qform_t::from(*a);
        let raw_b = s64_qform_t::from(*b);
        unsafe { s64_qform_equal(self.ptr(), &raw_a, &raw_b) != 0 }
    }

    pub fn set(&self, dst: &mut S64Form, src: &S64Form) {
        let mut raw_dst = s64_qform_t::from(*dst);
        let raw_src = s64_qform_t::from(*src);
        unsafe { s64_qform_set(self.ptr(), &mut raw_dst, &raw_src) };
        *dst = raw_dst.into();
    }

    pub fn inverse(&self, form: &mut S64Form) {
        let mut raw = s64_qform_t::from(*form);
        unsafe { s64_qform_inverse(self.ptr(), &mut raw) };
        *form = raw.into();
    }

    pub fn reduce(&self, form: &mut S64Form) {
        let mut raw = s64_qform_t::from(*form);
        unsafe { s64_qform_reduce(self.ptr(), &mut raw) };
        *form = raw.into();
    }

    pub fn compose(&self, a: &S64Form, b: &S64Form) -> S64Form {
        let raw_a = s64_qform_t::from(*a);
        let raw_b = s64_qform_t::from(*b);
        let mut out: s64_qform_t = unsafe { std::mem::zeroed() };
        unsafe { s64_qform_compose(self.ptr(), &mut out, &raw_a, &raw_b) };
        out.into()
    }

    pub fn square(&self, a: &S64Form) -> S64Form {
        let raw = s64_qform_t::from(*a);
        let mut out: s64_qform_t = unsafe { std::mem::zeroed() };
        unsafe { s64_qform_square(self.ptr(), &mut out, &raw) };
        out.into()
    }

    pub fn cube(&self, a: &S64Form) -> S64Form {
        let raw = s64_qform_t::from(*a);
        let mut out: s64_qform_t = unsafe { std::mem::zeroed() };
        unsafe { s64_qform_cube(self.ptr(), &mut out, &raw) };
        out.into()
    }

    pub fn print(&self, form: &S64Form) {
        let raw = s64_qform_t::from(*form);
        unsafe { s64_qform_print(self.ptr(), &raw) };
    }

    pub fn set3(&self, a: i32, b: i32, c: i64) -> S64Form {
        let mut out: s64_qform_t = unsafe { std::mem::zeroed() };
        unsafe { s64_qform_set3(self.ptr(), &mut out, a, b, c) };
        out.into()
    }

    pub fn prime_form(&self, p: i32) -> Option<S64Form> {
        let mut raw: s64_qform_t = unsafe { std::mem::zeroed() };
        let ok = unsafe { s64_qform_is_primeform(self.ptr(), &mut raw, p) };
        if ok == 0 { None } else { Some(raw.into()) }
    }

    pub fn random_prime_form(&self) -> S64Form {
        let mut raw: s64_qform_t = unsafe { std::mem::zeroed() };
        unsafe { qform_random_primeform(self.qgroup_ptr(), (&mut raw as *mut s64_qform_t).cast()) };
        raw.into()
    }

    pub fn next_prime_form(&self, prime_index: i32) -> Option<(i32, S64Form)> {
        let prime_index = validate_prime_index(prime_index)?;
        let mut raw: s64_qform_t = unsafe { std::mem::zeroed() };
        let out = unsafe {
            qform_next_primeform(
                self.qgroup_ptr(),
                (&mut raw as *mut s64_qform_t).cast(),
                prime_index,
            )
        };
        if out < 0 {
            None
        } else {
            Some((out, raw.into()))
        }
    }

    pub fn pow_u32(&self, base: &S64Form, exp: u32) -> S64Form {
        let raw_base = s64_qform_t::from(*base);
        let mut out: s64_qform_t = unsafe { std::mem::zeroed() };
        unsafe {
            with_group_pow(self.group_ptr(), |pow| {
                qform_pow_u32(
                    pow,
                    (&mut out as *mut s64_qform_t).cast(),
                    (&raw_base as *const s64_qform_t).cast(),
                    exp,
                );
            });
        }
        out.into()
    }

    pub fn split_ambiguous_str(&self, form: &S64Form, n: &str) -> Option<String> {
        let raw = s64_qform_t::from(*form);
        split_ambiguous_str(n, |d, n_ptr| unsafe {
            s64_qform_split_ambiguous(self.ptr(), d, n_ptr, &raw)
        })
    }

    #[cfg(feature = "gmp")]
    pub fn split_ambiguous_mpz(&self, form: &S64Form, n: &Mpz) -> Option<Mpz> {
        let out = self.split_ambiguous_str(form, &n.to_string_radix(10))?;
        Mpz::from_str_radix(&out, 10).ok()
    }
}

impl Drop for S64Group {
    fn drop(&mut self) {
        unsafe { s64_qform_group_clear(&mut self.raw) };
    }
}

pub struct S128Group {
    raw: s128_qform_group_t,
}

impl S128Group {
    fn ptr(&self) -> *mut s128_qform_group_t {
        &self.raw as *const _ as *mut _
    }

    fn qgroup_ptr(&self) -> *mut qform_group_t {
        unsafe { &mut (*self.ptr()).desc }
    }

    fn group_ptr(&self) -> *mut group_t {
        unsafe { &mut (*self.qgroup_ptr()).group }
    }

    pub fn new(discriminant: i128) -> Option<Self> {
        let mut raw: s128_qform_group_t = unsafe { std::mem::zeroed() };
        unsafe {
            s128_qform_group_init(&mut raw);
            let d = i128_to_s128(discriminant);
            s128_qform_group_set_discriminant_s128(&mut raw, &d);
        }
        Some(Self { raw })
    }

    pub fn set_discriminant(&self, discriminant: i128) {
        let d = i128_to_s128(discriminant);
        unsafe { s128_qform_group_set_discriminant_s128(self.ptr(), &d) };
    }

    pub fn set_discriminant_str(&self, discriminant: &str) -> bool {
        with_mpz_str(discriminant, |d| unsafe {
            s128_qform_group_set_discriminant(self.ptr(), d);
            true
        })
        .unwrap_or(false)
    }

    #[cfg(feature = "gmp")]
    pub fn set_discriminant_mpz(&self, discriminant: &Mpz) -> bool {
        let d = discriminant.to_string_radix(10);
        self.set_discriminant_str(&d)
    }

    pub fn identity(&self) -> S128Form {
        let mut out: s128_qform_t = unsafe { std::mem::zeroed() };
        unsafe { s128_qform_set_id(self.ptr(), &mut out) };
        out.into()
    }

    pub fn init_form(&self, form: &mut S128Form) {
        let mut raw = s128_qform_t::from(*form);
        unsafe { s128_qform_init(self.ptr(), &mut raw) };
        *form = raw.into();
    }

    pub fn clear_form(&self, form: &mut S128Form) {
        let mut raw = s128_qform_t::from(*form);
        unsafe { s128_qform_clear(self.ptr(), &mut raw) };
        *form = raw.into();
    }

    pub fn hash32(&self, form: &S128Form) -> u32 {
        let raw = s128_qform_t::from(*form);
        unsafe { s128_qform_hash32(self.ptr(), &raw) }
    }

    pub fn is_id(&self, form: &S128Form) -> bool {
        let raw = s128_qform_t::from(*form);
        unsafe { s128_qform_is_id(self.ptr(), &raw) != 0 }
    }

    pub fn is_ambiguous(&self, form: &S128Form) -> bool {
        let raw = s128_qform_t::from(*form);
        unsafe { s128_qform_is_ambiguous(self.ptr(), &raw) != 0 }
    }

    pub fn equal(&self, a: &S128Form, b: &S128Form) -> bool {
        let raw_a = s128_qform_t::from(*a);
        let raw_b = s128_qform_t::from(*b);
        unsafe { s128_qform_equal(self.ptr(), &raw_a, &raw_b) != 0 }
    }

    pub fn set(&self, dst: &mut S128Form, src: &S128Form) {
        let mut raw_dst = s128_qform_t::from(*dst);
        let raw_src = s128_qform_t::from(*src);
        unsafe { s128_qform_set(self.ptr(), &mut raw_dst, &raw_src) };
        *dst = raw_dst.into();
    }

    pub fn inverse(&self, form: &mut S128Form) {
        let mut raw = s128_qform_t::from(*form);
        unsafe { s128_qform_inverse(self.ptr(), &mut raw) };
        *form = raw.into();
    }

    pub fn reduce(&self, form: &mut S128Form) {
        let mut raw = s128_qform_t::from(*form);
        unsafe { s128_qform_reduce(self.ptr(), &mut raw) };
        *form = raw.into();
    }

    pub fn compose(&self, a: &S128Form, b: &S128Form) -> S128Form {
        let raw_a = s128_qform_t::from(*a);
        let raw_b = s128_qform_t::from(*b);
        let mut out: s128_qform_t = unsafe { std::mem::zeroed() };
        unsafe { s128_qform_compose(self.ptr(), &mut out, &raw_a, &raw_b) };
        out.into()
    }

    pub fn square(&self, a: &S128Form) -> S128Form {
        let raw = s128_qform_t::from(*a);
        let mut out: s128_qform_t = unsafe { std::mem::zeroed() };
        unsafe { s128_qform_square(self.ptr(), &mut out, &raw) };
        out.into()
    }

    pub fn cube(&self, a: &S128Form) -> S128Form {
        let raw = s128_qform_t::from(*a);
        let mut out: s128_qform_t = unsafe { std::mem::zeroed() };
        unsafe { s128_qform_cube(self.ptr(), &mut out, &raw) };
        out.into()
    }

    pub fn print(&self, form: &S128Form) {
        let raw = s128_qform_t::from(*form);
        unsafe { s128_qform_print(self.ptr(), &raw) };
    }

    pub fn set3(&self, a: i64, b: i64, c: i128) -> S128Form {
        let c_raw = i128_to_s128(c);
        let mut out: s128_qform_t = unsafe { std::mem::zeroed() };
        unsafe { s128_qform_set3(self.ptr(), &mut out, a, b, &c_raw) };
        out.into()
    }

    pub fn prime_form(&self, p: i32) -> Option<S128Form> {
        let mut raw: s128_qform_t = unsafe { std::mem::zeroed() };
        let ok = unsafe { s128_qform_is_primeform(self.ptr(), &mut raw, p) };
        if ok == 0 { None } else { Some(raw.into()) }
    }

    pub fn random_prime_form(&self) -> S128Form {
        let mut raw: s128_qform_t = unsafe { std::mem::zeroed() };
        unsafe {
            qform_random_primeform(self.qgroup_ptr(), (&mut raw as *mut s128_qform_t).cast())
        };
        raw.into()
    }

    pub fn next_prime_form(&self, prime_index: i32) -> Option<(i32, S128Form)> {
        let prime_index = validate_prime_index(prime_index)?;
        let mut raw: s128_qform_t = unsafe { std::mem::zeroed() };
        let out = unsafe {
            qform_next_primeform(
                self.qgroup_ptr(),
                (&mut raw as *mut s128_qform_t).cast(),
                prime_index,
            )
        };
        if out < 0 {
            None
        } else {
            Some((out, raw.into()))
        }
    }

    pub fn pow_u32(&self, base: &S128Form, exp: u32) -> S128Form {
        let raw_base = s128_qform_t::from(*base);
        let mut out: s128_qform_t = unsafe { std::mem::zeroed() };
        unsafe {
            with_group_pow(self.group_ptr(), |pow| {
                qform_pow_u32(
                    pow,
                    (&mut out as *mut s128_qform_t).cast(),
                    (&raw_base as *const s128_qform_t).cast(),
                    exp,
                );
            });
        }
        out.into()
    }

    pub fn split_ambiguous_str(&self, form: &S128Form, n: &str) -> Option<String> {
        let raw = s128_qform_t::from(*form);
        split_ambiguous_str(n, |d, n_ptr| unsafe {
            s128_qform_split_ambiguous(self.ptr(), d, n_ptr, &raw)
        })
    }

    #[cfg(feature = "gmp")]
    pub fn split_ambiguous_mpz(&self, form: &S128Form, n: &Mpz) -> Option<Mpz> {
        let out = self.split_ambiguous_str(form, &n.to_string_radix(10))?;
        Mpz::from_str_radix(&out, 10).ok()
    }
}

impl Drop for S128Group {
    fn drop(&mut self) {
        unsafe { s128_qform_group_clear(&mut self.raw) };
    }
}

pub struct MpzForm {
    raw: mpz_qform_t,
}

impl MpzForm {
    fn as_ptr(&self) -> *const mpz_qform_t {
        &self.raw
    }

    fn as_mut_ptr(&mut self) -> *mut mpz_qform_t {
        &mut self.raw
    }
}

impl Drop for MpzForm {
    fn drop(&mut self) {
        unsafe { clear_mpz_qform_raw(&mut self.raw) }
    }
}

pub struct MpzGroup {
    raw: mpz_qform_group_t,
}

impl MpzGroup {
    fn ptr(&self) -> *mut mpz_qform_group_t {
        &self.raw as *const _ as *mut _
    }

    fn qgroup_ptr(&self) -> *mut qform_group_t {
        unsafe { &mut (*self.ptr()).desc }
    }

    fn group_ptr(&self) -> *mut group_t {
        unsafe { &mut (*self.qgroup_ptr()).group }
    }

    pub fn new_i64(discriminant: i64) -> Option<Self> {
        let mut raw: mpz_qform_group_t = unsafe { std::mem::zeroed() };
        unsafe { mpz_qform_group_init(&mut raw) };
        let group = Self { raw };
        group.set_discriminant_i64(discriminant);
        Some(group)
    }

    pub fn new_i128(discriminant: i128) -> Option<Self> {
        let mut raw: mpz_qform_group_t = unsafe { std::mem::zeroed() };
        unsafe { mpz_qform_group_init(&mut raw) };
        let group = Self { raw };
        let _ = group.set_discriminant_str(&discriminant.to_string());
        Some(group)
    }

    pub fn new_str(discriminant: &str) -> Option<Self> {
        let mut raw: mpz_qform_group_t = unsafe { std::mem::zeroed() };
        unsafe { mpz_qform_group_init(&mut raw) };
        let group = Self { raw };
        if group.set_discriminant_str(discriminant) {
            Some(group)
        } else {
            None
        }
    }

    #[cfg(feature = "gmp")]
    pub fn new_mpz(discriminant: &Mpz) -> Option<Self> {
        Self::new_str(&discriminant.to_string_radix(10))
    }

    pub fn set_discriminant_i64(&self, discriminant: i64) {
        unsafe {
            let mut d: mpz_t = std::mem::zeroed();
            gmpz_init(d.as_mut_ptr());
            gmpz_set_si(d.as_mut_ptr(), discriminant as c_long);
            mpz_qform_group_set_discriminant(self.ptr(), d.as_ptr());
            gmpz_clear(d.as_mut_ptr());
        }
    }

    pub fn set_discriminant_str(&self, discriminant: &str) -> bool {
        with_mpz_str(discriminant, |d| unsafe {
            mpz_qform_group_set_discriminant(self.ptr(), d);
            true
        })
        .unwrap_or(false)
    }

    #[cfg(feature = "gmp")]
    pub fn set_discriminant_mpz(&self, discriminant: &Mpz) -> bool {
        self.set_discriminant_str(&discriminant.to_string_radix(10))
    }

    pub fn new_form(&self) -> Option<MpzForm> {
        let mut form = MpzForm {
            raw: unsafe { std::mem::zeroed() },
        };
        unsafe { mpz_qform_init(self.ptr(), form.as_mut_ptr()) };
        Some(form)
    }

    pub fn identity(&self) -> Option<MpzForm> {
        let mut form = self.new_form()?;
        self.set_id(&mut form);
        Some(form)
    }

    pub fn set_id(&self, form: &mut MpzForm) {
        unsafe { mpz_qform_set_id(self.ptr(), form.as_mut_ptr()) };
    }

    pub fn clear_form(&self, form: &mut MpzForm) {
        unsafe {
            mpz_qform_clear(self.ptr(), form.as_mut_ptr());
            mpz_qform_init(self.ptr(), form.as_mut_ptr());
        }
    }

    pub fn hash32(&self, form: &MpzForm) -> u32 {
        unsafe { mpz_qform_hash32(self.ptr(), form.as_ptr()) }
    }

    pub fn is_id(&self, form: &MpzForm) -> bool {
        unsafe { mpz_qform_is_id(self.ptr(), form.as_ptr()) != 0 }
    }

    pub fn is_ambiguous(&self, form: &MpzForm) -> bool {
        unsafe { mpz_qform_is_ambiguous(self.ptr(), form.as_ptr()) != 0 }
    }

    pub fn equal(&self, a: &MpzForm, b: &MpzForm) -> bool {
        unsafe { mpz_qform_equal(self.ptr(), a.as_ptr(), b.as_ptr()) != 0 }
    }

    pub fn set(&self, dst: &mut MpzForm, src: &MpzForm) {
        unsafe { mpz_qform_set(self.ptr(), dst.as_mut_ptr(), src.as_ptr()) };
    }

    pub fn inverse(&self, form: &mut MpzForm) {
        unsafe { mpz_qform_inverse(self.ptr(), form.as_mut_ptr()) };
    }

    pub fn reduce(&self, form: &mut MpzForm) {
        unsafe { mpz_qform_reduce(self.ptr(), form.as_mut_ptr()) };
    }

    pub fn compose(&self, out: &mut MpzForm, a: &MpzForm, b: &MpzForm) {
        unsafe { mpz_qform_compose(self.ptr(), out.as_mut_ptr(), a.as_ptr(), b.as_ptr()) };
    }

    pub fn square(&self, out: &mut MpzForm, a: &MpzForm) {
        unsafe { mpz_qform_square(self.ptr(), out.as_mut_ptr(), a.as_ptr()) };
    }

    pub fn cube(&self, out: &mut MpzForm, a: &MpzForm) {
        unsafe { mpz_qform_cube(self.ptr(), out.as_mut_ptr(), a.as_ptr()) };
    }

    pub fn print(&self, form: &MpzForm) {
        unsafe { mpz_qform_print(self.ptr(), form.as_ptr()) };
    }

    pub fn is_primeform(&self, form: &mut MpzForm, p: i32) -> bool {
        unsafe { mpz_qform_is_primeform(self.ptr(), form.as_mut_ptr(), p) != 0 }
    }

    pub fn prime_form(&self, p: i32) -> Option<MpzForm> {
        let mut form = self.new_form()?;
        if self.is_primeform(&mut form, p) {
            Some(form)
        } else {
            None
        }
    }

    pub fn coefficients(&self, form: &MpzForm) -> Option<(String, String, String)> {
        let a = mpz_to_string(form.raw.a.as_ptr())?;
        let b = mpz_to_string(form.raw.b.as_ptr())?;
        let c = mpz_to_string(form.raw.c.as_ptr())?;
        Some((a, b, c))
    }

    pub fn set_coefficients_str(&self, form: &mut MpzForm, a: &str, b: &str, c: &str) -> bool {
        let Ok(a_c) = CString::new(a) else {
            return false;
        };
        let Ok(b_c) = CString::new(b) else {
            return false;
        };
        let Ok(c_c) = CString::new(c) else {
            return false;
        };
        unsafe {
            gmpz_set_str(form.raw.a.as_mut_ptr(), a_c.as_ptr(), 10) == 0
                && gmpz_set_str(form.raw.b.as_mut_ptr(), b_c.as_ptr(), 10) == 0
                && gmpz_set_str(form.raw.c.as_mut_ptr(), c_c.as_ptr(), 10) == 0
        }
    }

    #[cfg(feature = "gmp")]
    pub fn set_coefficients_mpz(&self, form: &mut MpzForm, a: &Mpz, b: &Mpz, c: &Mpz) -> bool {
        self.set_coefficients_str(
            form,
            &a.to_string_radix(10),
            &b.to_string_radix(10),
            &c.to_string_radix(10),
        )
    }

    #[cfg(feature = "gmp")]
    pub fn coefficients_mpz(&self, form: &MpzForm) -> Option<(Mpz, Mpz, Mpz)> {
        let (a, b, c) = self.coefficients(form)?;
        Some((
            Mpz::from_str_radix(&a, 10).ok()?,
            Mpz::from_str_radix(&b, 10).ok()?,
            Mpz::from_str_radix(&c, 10).ok()?,
        ))
    }

    pub fn random_prime_form(&self) -> Option<MpzForm> {
        let mut form = self.new_form()?;
        unsafe { qform_random_primeform(self.qgroup_ptr(), form.as_mut_ptr().cast()) };
        Some(form)
    }

    pub fn next_prime_form(&self, prime_index: i32) -> Option<(i32, MpzForm)> {
        let prime_index = validate_prime_index(prime_index)?;
        let mut form = self.new_form()?;
        let out = unsafe {
            qform_next_primeform(self.qgroup_ptr(), form.as_mut_ptr().cast(), prime_index)
        };
        if out < 0 { None } else { Some((out, form)) }
    }

    pub fn pow_u32(&self, base: &MpzForm, exp: u32) -> Option<MpzForm> {
        let mut out = self.new_form()?;
        unsafe {
            with_group_pow(self.group_ptr(), |pow| {
                qform_pow_u32(pow, out.as_mut_ptr().cast(), base.as_ptr().cast(), exp);
            });
        }
        Some(out)
    }

    pub fn split_ambiguous_str(&self, form: &MpzForm, n: &str) -> Option<String> {
        split_ambiguous_str(n, |d, n_ptr| unsafe {
            mpz_qform_split_ambiguous(self.ptr(), d, n_ptr, form.as_ptr())
        })
    }

    #[cfg(feature = "gmp")]
    pub fn split_ambiguous_mpz(&self, form: &MpzForm, n: &Mpz) -> Option<Mpz> {
        let out = self.split_ambiguous_str(form, &n.to_string_radix(10))?;
        Mpz::from_str_radix(&out, 10).ok()
    }
}

impl Drop for MpzGroup {
    fn drop(&mut self) {
        unsafe { mpz_qform_group_clear(&mut self.raw) };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenBackend {
    S64,
    S128,
    Mpz,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenCoefficients {
    S64(S64Form),
    S128(S128Form),
    Mpz { a: String, b: String, c: String },
}

pub struct GenGroup {
    raw: gen_qform_group_t,
}

pub struct GenForm {
    raw: gen_qform_t,
}

impl GenGroup {
    fn ptr(&self) -> *mut gen_qform_group_t {
        &self.raw as *const _ as *mut _
    }

    fn qgroup_ptr(&self) -> *mut qform_group_t {
        unsafe { &mut (*self.ptr()).desc }
    }

    fn group_ptr(&self) -> *mut group_t {
        unsafe { &mut (*self.qgroup_ptr()).group }
    }

    fn backend_code(&self) -> i32 {
        self.raw.logD
    }

    fn is_s64_backend(&self) -> bool {
        self.backend_code() <= s64_qform_group_max_bits as i32
    }

    fn is_s128_backend(&self) -> bool {
        self.backend_code() <= s128_qform_group_max_bits as i32
    }

    fn is_s128_only_backend(&self) -> bool {
        self.is_s128_backend() && !self.is_s64_backend()
    }

    fn is_mpz_backend(&self) -> bool {
        !self.is_s128_backend()
    }

    pub fn new_i64(discriminant: i64) -> Option<Self> {
        let mut raw: gen_qform_group_t = unsafe { std::mem::zeroed() };
        unsafe { gen_qform_group_init(&mut raw) };
        let group = Self { raw };
        group.set_discriminant_i64(discriminant);
        Some(group)
    }

    pub fn new_i128(discriminant: i128) -> Option<Self> {
        let mut raw: gen_qform_group_t = unsafe { std::mem::zeroed() };
        unsafe { gen_qform_group_init(&mut raw) };
        let group = Self { raw };
        group.set_discriminant_i128(discriminant);
        Some(group)
    }

    pub fn new_str(discriminant: &str) -> Option<Self> {
        let mut raw: gen_qform_group_t = unsafe { std::mem::zeroed() };
        unsafe { gen_qform_group_init(&mut raw) };
        let group = Self { raw };
        if group.set_discriminant_str(discriminant) {
            Some(group)
        } else {
            None
        }
    }

    #[cfg(feature = "gmp")]
    pub fn new_mpz(discriminant: &Mpz) -> Option<Self> {
        let d = discriminant.to_string_radix(10);
        Self::new_str(&d)
    }

    pub fn set_discriminant_i64(&self, discriminant: i64) {
        unsafe {
            let mut d: mpz_t = std::mem::zeroed();
            gmpz_init(d.as_mut_ptr());
            gmpz_set_si(d.as_mut_ptr(), discriminant as c_long);
            gen_qform_group_set_discriminant(self.ptr(), d.as_ptr());
            gmpz_clear(d.as_mut_ptr());
        }
    }

    pub fn set_discriminant_i128(&self, discriminant: i128) {
        let d = discriminant.to_string();
        let _ = self.set_discriminant_str(&d);
    }

    pub fn set_discriminant_str(&self, discriminant: &str) -> bool {
        let Ok(c_discriminant) = CString::new(discriminant) else {
            return false;
        };

        unsafe {
            let mut d: mpz_t = std::mem::zeroed();
            gmpz_init(d.as_mut_ptr());
            let ok = gmpz_set_str(d.as_mut_ptr(), c_discriminant.as_ptr(), 10) == 0;
            if ok {
                gen_qform_group_set_discriminant(self.ptr(), d.as_ptr());
            }
            gmpz_clear(d.as_mut_ptr());
            ok
        }
    }

    #[cfg(feature = "gmp")]
    pub fn set_discriminant_mpz(&self, discriminant: &Mpz) -> bool {
        let d = discriminant.to_string_radix(10);
        self.set_discriminant_str(&d)
    }

    pub fn backend(&self) -> GenBackend {
        if self.is_s64_backend() {
            GenBackend::S64
        } else if self.is_s128_backend() {
            GenBackend::S128
        } else {
            GenBackend::Mpz
        }
    }

    pub fn new_form(&self) -> Option<GenForm> {
        let mut form = GenForm {
            raw: unsafe { std::mem::zeroed() },
        };
        unsafe { gen_qform_init(self.ptr(), &mut form.raw) };
        Some(form)
    }

    pub fn identity(&self) -> Option<GenForm> {
        let mut form = self.new_form()?;
        self.set_id(&mut form);
        Some(form)
    }

    pub fn set_id(&self, form: &mut GenForm) {
        unsafe { gen_qform_set_id(self.ptr(), &mut form.raw) };
    }

    pub fn clear_form(&self, form: &mut GenForm) {
        unsafe {
            gen_qform_clear(self.ptr(), &mut form.raw);
            gen_qform_init(self.ptr(), &mut form.raw);
        }
    }

    pub fn hash32(&self, form: &GenForm) -> u32 {
        unsafe { gen_qform_hash32(self.ptr(), &form.raw) }
    }

    pub fn is_id(&self, form: &GenForm) -> bool {
        unsafe { gen_qform_is_id(self.ptr(), &form.raw) != 0 }
    }

    pub fn is_ambiguous(&self, form: &GenForm) -> bool {
        unsafe { gen_qform_is_ambiguous(self.ptr(), &form.raw) != 0 }
    }

    pub fn equal(&self, a: &GenForm, b: &GenForm) -> bool {
        unsafe { gen_qform_equal(self.ptr(), &a.raw, &b.raw) != 0 }
    }

    pub fn set(&self, dst: &mut GenForm, src: &GenForm) {
        unsafe { gen_qform_set(self.ptr(), &mut dst.raw, &src.raw) };
    }

    pub fn inverse(&self, form: &mut GenForm) {
        unsafe { gen_qform_inverse(self.ptr(), &mut form.raw) };
    }

    pub fn reduce(&self, form: &mut GenForm) {
        unsafe { gen_qform_reduce(self.ptr(), &mut form.raw) };
    }

    pub fn compose(&self, out: &mut GenForm, a: &GenForm, b: &GenForm) {
        unsafe { gen_qform_compose(self.ptr(), &mut out.raw, &a.raw, &b.raw) };
    }

    pub fn square(&self, out: &mut GenForm, a: &GenForm) {
        unsafe { gen_qform_square(self.ptr(), &mut out.raw, &a.raw) };
    }

    pub fn cube(&self, out: &mut GenForm, a: &GenForm) {
        unsafe { gen_qform_cube(self.ptr(), &mut out.raw, &a.raw) };
    }

    pub fn is_primeform(&self, form: &mut GenForm, p: i32) -> bool {
        unsafe { gen_qform_is_primeform(self.ptr(), &mut form.raw, p) != 0 }
    }

    pub fn prime_form(&self, p: i32) -> Option<GenForm> {
        let mut form = self.new_form()?;
        if self.is_primeform(&mut form, p) {
            Some(form)
        } else {
            None
        }
    }

    pub fn random_prime_form(&self) -> Option<GenForm> {
        let mut form = self.new_form()?;
        unsafe {
            qform_random_primeform(
                self.qgroup_ptr(),
                (&mut form.raw as *mut gen_qform_t).cast(),
            )
        };
        Some(form)
    }

    pub fn next_prime_form(&self, prime_index: i32) -> Option<(i32, GenForm)> {
        let prime_index = validate_prime_index(prime_index)?;
        let mut form = self.new_form()?;
        let out = unsafe {
            qform_next_primeform(
                self.qgroup_ptr(),
                (&mut form.raw as *mut gen_qform_t).cast(),
                prime_index,
            )
        };
        if out < 0 { None } else { Some((out, form)) }
    }

    pub fn pow_u32(&self, base: &GenForm, exp: u32) -> Option<GenForm> {
        let mut out = self.new_form()?;
        unsafe {
            with_group_pow(self.group_ptr(), |pow| {
                gen_qform_pow_u32(pow, &mut out.raw, &base.raw, exp);
            });
        }
        Some(out)
    }

    pub fn print(&self, form: &GenForm) {
        unsafe { gen_qform_print(self.ptr(), &form.raw) };
    }

    pub fn split_ambiguous_str(&self, form: &GenForm, n: &str) -> Option<String> {
        split_ambiguous_str(n, |d, n_ptr| unsafe {
            gen_qform_split_ambiguous(self.ptr(), d, n_ptr, &form.raw)
        })
    }

    #[cfg(feature = "gmp")]
    pub fn split_ambiguous_mpz(&self, form: &GenForm, n: &Mpz) -> Option<Mpz> {
        let out = self.split_ambiguous_str(form, &n.to_string_radix(10))?;
        Mpz::from_str_radix(&out, 10).ok()
    }

    pub fn set_from_s64(&self, form: &mut GenForm, value: S64Form) -> bool {
        if !self.is_s64_backend() {
            return false;
        }
        form.raw.s64_qform = value.into();
        true
    }

    pub fn get_s64(&self, form: &GenForm) -> Option<S64Form> {
        if !self.is_s64_backend() {
            return None;
        }
        Some(form.raw.s64_qform.into())
    }

    pub fn set_from_s128(&self, form: &mut GenForm, value: S128Form) -> bool {
        if !self.is_s128_only_backend() {
            return false;
        }
        form.raw.s128_qform = value.into();
        true
    }

    pub fn get_s128(&self, form: &GenForm) -> Option<S128Form> {
        if !self.is_s128_only_backend() {
            return None;
        }
        Some(form.raw.s128_qform.into())
    }

    pub fn set_from_mpz_strings(&self, form: &mut GenForm, a: &str, b: &str, c: &str) -> bool {
        if !self.is_mpz_backend() {
            return false;
        }

        let Ok(a_c) = CString::new(a) else {
            return false;
        };
        let Ok(b_c) = CString::new(b) else {
            return false;
        };
        let Ok(c_c) = CString::new(c) else {
            return false;
        };

        unsafe {
            gmpz_set_str(form.raw.mpz_qform.a.as_mut_ptr(), a_c.as_ptr(), 10) == 0
                && gmpz_set_str(form.raw.mpz_qform.b.as_mut_ptr(), b_c.as_ptr(), 10) == 0
                && gmpz_set_str(form.raw.mpz_qform.c.as_mut_ptr(), c_c.as_ptr(), 10) == 0
        }
    }

    #[cfg(feature = "gmp")]
    pub fn set_from_mpz(&self, form: &mut GenForm, a: &Mpz, b: &Mpz, c: &Mpz) -> bool {
        let a = a.to_string_radix(10);
        let b = b.to_string_radix(10);
        let c = c.to_string_radix(10);
        self.set_from_mpz_strings(form, &a, &b, &c)
    }

    pub fn mpz_coefficients(&self, form: &GenForm) -> Option<(String, String, String)> {
        if !self.is_mpz_backend() {
            return None;
        }

        let a = mpz_to_string(form.raw.mpz_qform.a.as_ptr())?;
        let b = mpz_to_string(form.raw.mpz_qform.b.as_ptr())?;
        let c = mpz_to_string(form.raw.mpz_qform.c.as_ptr())?;
        Some((a, b, c))
    }

    #[cfg(feature = "gmp")]
    pub fn get_mpz(&self, form: &GenForm) -> Option<(Mpz, Mpz, Mpz)> {
        let (a, b, c) = self.mpz_coefficients(form)?;
        let a = Mpz::from_str_radix(&a, 10).ok()?;
        let b = Mpz::from_str_radix(&b, 10).ok()?;
        let c = Mpz::from_str_radix(&c, 10).ok()?;
        Some((a, b, c))
    }

    pub fn coefficients(&self, form: &GenForm) -> Option<GenCoefficients> {
        match self.backend() {
            GenBackend::S64 => self.get_s64(form).map(GenCoefficients::S64),
            GenBackend::S128 => self.get_s128(form).map(GenCoefficients::S128),
            GenBackend::Mpz => self
                .mpz_coefficients(form)
                .map(|(a, b, c)| GenCoefficients::Mpz { a, b, c }),
        }
    }
}

impl Drop for GenGroup {
    fn drop(&mut self) {
        unsafe { gen_qform_group_clear(&mut self.raw) };
    }
}

impl Drop for GenForm {
    fn drop(&mut self) {
        unsafe {
            clear_mpz_qform_raw(&mut self.raw.mpz_qform);
        }
    }
}

pub struct SanityForm {
    raw: sanity_qform_t,
}

impl SanityForm {
    fn as_ptr(&self) -> *const sanity_qform_t {
        &self.raw
    }

    fn as_mut_ptr(&mut self) -> *mut sanity_qform_t {
        &mut self.raw
    }
}

impl Drop for SanityForm {
    fn drop(&mut self) {
        unsafe {
            clear_mpz_qform_raw(&mut self.raw.mpz_qform);
            clear_mpz_qform_raw(&mut self.raw.gen_qform.mpz_qform);
        }
    }
}

pub struct SanityGroup {
    raw: sanity_qform_group_t,
}

impl SanityGroup {
    fn ptr(&self) -> *mut sanity_qform_group_t {
        &self.raw as *const _ as *mut _
    }

    fn qgroup_ptr(&self) -> *mut qform_group_t {
        unsafe { &mut (*self.ptr()).desc }
    }

    fn group_ptr(&self) -> *mut group_t {
        unsafe { &mut (*self.qgroup_ptr()).group }
    }

    fn sync_pow_tables(&self) {
        unsafe {
            let desc = &mut (*self.ptr()).desc;
            let logd = (*self.ptr()).gen_group.logD;
            if logd <= s64_qform_group_max_bits as c_int {
                desc.pow_rep_sizes = std::ptr::addr_of_mut!(s64_pow_rep_sizes).cast::<c_int>();
                desc.pow_reps =
                    std::ptr::addr_of_mut!(s64_pow_reps).cast::<*mut factored_two_three_term16_t>();
            } else if logd <= s128_qform_group_max_bits as c_int {
                desc.pow_rep_sizes = std::ptr::addr_of_mut!(s128_pow_rep_sizes).cast::<c_int>();
                desc.pow_reps = std::ptr::addr_of_mut!(s128_pow_reps)
                    .cast::<*mut factored_two_three_term16_t>();
            } else {
                desc.pow_rep_sizes = std::ptr::addr_of_mut!(mpz_pow_rep_sizes).cast::<c_int>();
                desc.pow_reps =
                    std::ptr::addr_of_mut!(mpz_pow_reps).cast::<*mut factored_two_three_term16_t>();
            }
        }
    }

    pub fn new_i64(discriminant: i64) -> Option<Self> {
        let mut raw: sanity_qform_group_t = unsafe { std::mem::zeroed() };
        unsafe { sanity_qform_group_init(&mut raw) };
        let group = Self { raw };
        group.set_discriminant_i64(discriminant);
        Some(group)
    }

    pub fn new_i128(discriminant: i128) -> Option<Self> {
        let mut raw: sanity_qform_group_t = unsafe { std::mem::zeroed() };
        unsafe { sanity_qform_group_init(&mut raw) };
        let group = Self { raw };
        let _ = group.set_discriminant_str(&discriminant.to_string());
        Some(group)
    }

    pub fn new_str(discriminant: &str) -> Option<Self> {
        let mut raw: sanity_qform_group_t = unsafe { std::mem::zeroed() };
        unsafe { sanity_qform_group_init(&mut raw) };
        let group = Self { raw };
        if group.set_discriminant_str(discriminant) {
            Some(group)
        } else {
            None
        }
    }

    #[cfg(feature = "gmp")]
    pub fn new_mpz(discriminant: &Mpz) -> Option<Self> {
        Self::new_str(&discriminant.to_string_radix(10))
    }

    pub fn set_discriminant_i64(&self, discriminant: i64) {
        unsafe {
            let mut d: mpz_t = std::mem::zeroed();
            gmpz_init(d.as_mut_ptr());
            gmpz_set_si(d.as_mut_ptr(), discriminant as c_long);
            sanity_qform_group_set_discriminant(self.ptr(), d.as_ptr());
            gmpz_clear(d.as_mut_ptr());
        }
        self.sync_pow_tables();
    }

    pub fn set_discriminant_str(&self, discriminant: &str) -> bool {
        let ok = with_mpz_str(discriminant, |d| unsafe {
            sanity_qform_group_set_discriminant(self.ptr(), d);
            true
        })
        .unwrap_or(false);
        if ok {
            self.sync_pow_tables();
        }
        ok
    }

    #[cfg(feature = "gmp")]
    pub fn set_discriminant_mpz(&self, discriminant: &Mpz) -> bool {
        self.set_discriminant_str(&discriminant.to_string_radix(10))
    }

    pub fn new_form(&self) -> Option<SanityForm> {
        let mut form = SanityForm {
            raw: unsafe { std::mem::zeroed() },
        };
        unsafe { sanity_qform_init(self.ptr(), form.as_mut_ptr()) };
        Some(form)
    }

    pub fn identity(&self) -> Option<SanityForm> {
        let mut form = self.new_form()?;
        self.set_id(&mut form);
        Some(form)
    }

    pub fn set_id(&self, form: &mut SanityForm) {
        unsafe { sanity_qform_set_id(self.ptr(), form.as_mut_ptr()) };
    }

    pub fn clear_form(&self, form: &mut SanityForm) {
        unsafe {
            sanity_qform_clear(self.ptr(), form.as_mut_ptr());
            sanity_qform_init(self.ptr(), form.as_mut_ptr());
        }
    }

    pub fn hash32(&self, form: &SanityForm) -> u32 {
        unsafe { sanity_qform_hash32(self.ptr(), form.as_ptr()) }
    }

    pub fn is_id(&self, form: &SanityForm) -> bool {
        unsafe { sanity_qform_is_id(self.ptr(), form.as_ptr()) != 0 }
    }

    pub fn is_ambiguous(&self, form: &SanityForm) -> bool {
        unsafe { sanity_qform_is_ambiguous(self.ptr(), form.as_ptr()) != 0 }
    }

    pub fn equal(&self, a: &SanityForm, b: &SanityForm) -> bool {
        unsafe { sanity_qform_equal(self.ptr(), a.as_ptr(), b.as_ptr()) != 0 }
    }

    pub fn set(&self, dst: &mut SanityForm, src: &SanityForm) {
        unsafe { sanity_qform_set(self.ptr(), dst.as_mut_ptr(), src.as_ptr()) };
    }

    pub fn inverse(&self, form: &mut SanityForm) {
        unsafe { sanity_qform_inverse(self.ptr(), form.as_mut_ptr()) };
    }

    pub fn reduce(&self, form: &mut SanityForm) {
        unsafe { sanity_qform_reduce(self.ptr(), form.as_mut_ptr()) };
    }

    pub fn compose(&self, out: &mut SanityForm, a: &SanityForm, b: &SanityForm) {
        unsafe { sanity_qform_compose(self.ptr(), out.as_mut_ptr(), a.as_ptr(), b.as_ptr()) };
    }

    pub fn square(&self, out: &mut SanityForm, a: &SanityForm) {
        unsafe { sanity_qform_square(self.ptr(), out.as_mut_ptr(), a.as_ptr()) };
    }

    pub fn cube(&self, out: &mut SanityForm, a: &SanityForm) {
        unsafe { sanity_qform_cube(self.ptr(), out.as_mut_ptr(), a.as_ptr()) };
    }

    pub fn print(&self, form: &SanityForm) {
        unsafe { sanity_qform_print(self.ptr(), form.as_ptr()) };
    }

    pub fn is_primeform(&self, form: &mut SanityForm, p: i32) -> bool {
        unsafe { sanity_qform_is_primeform(self.ptr(), form.as_mut_ptr(), p) != 0 }
    }

    pub fn prime_form(&self, p: i32) -> Option<SanityForm> {
        let mut form = self.new_form()?;
        if self.is_primeform(&mut form, p) {
            Some(form)
        } else {
            None
        }
    }

    pub fn random_prime_form(&self) -> Option<SanityForm> {
        let mut form = self.new_form()?;
        unsafe { qform_random_primeform(self.qgroup_ptr(), form.as_mut_ptr().cast()) };
        Some(form)
    }

    pub fn next_prime_form(&self, prime_index: i32) -> Option<(i32, SanityForm)> {
        let prime_index = validate_prime_index(prime_index)?;
        let mut form = self.new_form()?;
        let out = unsafe {
            qform_next_primeform(self.qgroup_ptr(), form.as_mut_ptr().cast(), prime_index)
        };
        if out < 0 { None } else { Some((out, form)) }
    }

    pub fn pow_u32(&self, base: &SanityForm, exp: u32) -> Option<SanityForm> {
        let mut out = self.new_form()?;
        unsafe {
            with_group_pow(self.group_ptr(), |pow| {
                qform_pow_u32(pow, out.as_mut_ptr().cast(), base.as_ptr().cast(), exp);
            });
        }
        Some(out)
    }

    pub fn split_ambiguous_str(&self, form: &SanityForm, n: &str) -> Option<String> {
        split_ambiguous_str(n, |d, n_ptr| unsafe {
            sanity_qform_split_ambiguous(self.ptr(), d, n_ptr, form.as_ptr())
        })
    }

    #[cfg(feature = "gmp")]
    pub fn split_ambiguous_mpz(&self, form: &SanityForm, n: &Mpz) -> Option<Mpz> {
        let out = self.split_ambiguous_str(form, &n.to_string_radix(10))?;
        Mpz::from_str_radix(&out, 10).ok()
    }
}

impl Drop for SanityGroup {
    fn drop(&mut self) {
        unsafe { sanity_qform_group_clear(&mut self.raw) };
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "gmp")]
    use super::Mpz;
    use super::{
        GenBackend, GenGroup, MpzGroup, S64Form, S64Group, S128Form, S128Group, SanityGroup,
    };

    #[test]
    fn s64_identity_is_identity() {
        let group = S64Group::new(-23).expect("alloc group");
        let id = group.identity();
        assert!(group.is_id(&id));
    }

    #[test]
    fn s64_compose_with_identity_is_neutral() {
        let group = S64Group::new(-23).expect("alloc group");
        let id = group.identity();
        let f = S64Form { a: 2, b: 1, c: 3 };

        let left = group.compose(&id, &f);
        let right = group.compose(&f, &id);

        assert!(group.equal(&left, &f));
        assert!(group.equal(&right, &f));
    }

    #[test]
    fn s64_inverse_composes_to_identity() {
        let group = S64Group::new(-23).expect("alloc group");
        let f = S64Form { a: 2, b: 1, c: 3 };
        let mut inv = f;
        group.inverse(&mut inv);
        let prod = group.compose(&f, &inv);

        assert!(group.is_id(&prod));
    }

    #[test]
    fn s128_compose_with_identity_is_neutral() {
        let group = S128Group::new(-23).expect("alloc group");
        let id = group.identity();
        let f = S128Form { a: 2, b: 1, c: 3 };

        let left = group.compose(&id, &f);
        let right = group.compose(&f, &id);

        assert!(group.equal(&left, &f));
        assert!(group.equal(&right, &f));
    }

    #[test]
    fn s64_pow_u32_matches_cube_for_exp_three() {
        let group = S64Group::new(-23).expect("alloc group");
        let f = group.prime_form(2).expect("prime form");
        let pow = group.pow_u32(&f, 3);
        let cube = group.cube(&f);
        assert!(group.equal(&pow, &cube));
    }

    #[test]
    fn s128_pow_u32_matches_cube_for_exp_three() {
        let group = S128Group::new(-23).expect("alloc group");
        let f = group.prime_form(2).expect("prime form");
        let pow = group.pow_u32(&f, 3);
        let cube = group.cube(&f);
        assert!(group.equal(&pow, &cube));
    }

    #[test]
    fn next_prime_form_rejects_out_of_range_indices() {
        let s64 = S64Group::new(-23).expect("s64");
        assert!(s64.next_prime_form(-1).is_none());
        assert!(s64.next_prime_form(i32::MAX).is_none());

        let s128 = S128Group::new(-23).expect("s128");
        assert!(s128.next_prime_form(-1).is_none());
        assert!(s128.next_prime_form(i32::MAX).is_none());

        let mpz = MpzGroup::new_i64(-23).expect("mpz");
        assert!(mpz.next_prime_form(-1).is_none());
        assert!(mpz.next_prime_form(i32::MAX).is_none());

        let gen_group = GenGroup::new_i64(-23).expect("gen");
        assert!(gen_group.next_prime_form(-1).is_none());
        assert!(gen_group.next_prime_form(i32::MAX).is_none());

        let sanity = SanityGroup::new_i64(-23).expect("sanity");
        assert!(sanity.next_prime_form(-1).is_none());
        assert!(sanity.next_prime_form(i32::MAX).is_none());
    }

    #[test]
    fn s64_next_prime_form_indices_increase() {
        let group = S64Group::new(-23).expect("s64");
        let (i0, _) = group.next_prime_form(0).expect("first");
        let (i1, _) = group.next_prime_form(i0 + 1).expect("second");
        assert!(i1 > i0);
    }

    #[test]
    fn mpz_pow_u32_matches_cube_for_exp_three() {
        let group = MpzGroup::new_i64(-23).expect("mpz");
        let f = group.random_prime_form().expect("prime form");
        let pow = group.pow_u32(&f, 3).expect("pow");

        let mut cube = group.new_form().expect("cube");
        group.cube(&mut cube, &f);
        assert!(group.equal(&pow, &cube));
    }

    #[test]
    fn generic_pow_u32_matches_cube_for_exp_three() {
        let group = GenGroup::new_i64(-23).expect("gen");
        let f = group.random_prime_form().expect("prime form");
        let pow = group.pow_u32(&f, 3).expect("pow");

        let mut cube = group.new_form().expect("cube");
        group.cube(&mut cube, &f);
        assert!(group.equal(&pow, &cube));
    }

    #[test]
    fn sanity_pow_u32_matches_cube_for_exp_three() {
        let group = SanityGroup::new_i64(-23).expect("sanity");
        let f = group.random_prime_form().expect("prime form");
        let pow = group.pow_u32(&f, 3).expect("pow");

        let mut cube = group.new_form().expect("cube");
        group.cube(&mut cube, &f);
        assert!(group.equal(&pow, &cube));
    }

    #[test]
    fn mpz_group_identity_is_identity() {
        let group = MpzGroup::new_i64(-23).expect("alloc group");
        let id = group.identity().expect("id");
        assert!(group.is_id(&id));
    }

    #[test]
    fn sanity_group_identity_is_identity() {
        let group = SanityGroup::new_i64(-23).expect("alloc group");
        let id = group.identity().expect("id");
        assert!(group.is_id(&id));
    }

    #[test]
    fn clear_form_keeps_forms_usable() {
        let mpz = MpzGroup::new_i64(-23).expect("mpz");
        let mut mpz_form = mpz.identity().expect("id");
        mpz.clear_form(&mut mpz_form);
        mpz.set_id(&mut mpz_form);
        assert!(mpz.is_id(&mpz_form));

        let gen_group = GenGroup::new_i64(-23).expect("gen");
        let mut gen_form = gen_group.identity().expect("id");
        gen_group.clear_form(&mut gen_form);
        gen_group.set_id(&mut gen_form);
        assert!(gen_group.is_id(&gen_form));

        let sanity = SanityGroup::new_i64(-23).expect("sanity");
        let mut sanity_form = sanity.identity().expect("id");
        sanity.clear_form(&mut sanity_form);
        sanity.set_id(&mut sanity_form);
        assert!(sanity.is_id(&sanity_form));
    }

    #[test]
    fn generic_selects_backend_from_discriminant_size() {
        let g64 = GenGroup::new_i64(-23).expect("g64");
        assert_eq!(g64.backend(), GenBackend::S64);

        let g128 = GenGroup::new_i128(-(1_i128 << 80) + 1).expect("g128");
        assert_eq!(g128.backend(), GenBackend::S128);

        let gmp = GenGroup::new_str("-123456789012345678901234567890123456789").expect("gmp");
        assert_eq!(gmp.backend(), GenBackend::Mpz);
    }

    #[test]
    fn generic_s64_identity_and_compose() {
        let group = GenGroup::new_i64(-23).expect("group");

        let mut id = group.identity().expect("id");
        assert!(group.is_id(&id));

        let mut f = group.new_form().expect("form");
        assert!(group.set_from_s64(&mut f, S64Form { a: 2, b: 1, c: 3 }));

        let mut out = group.new_form().expect("out");
        group.compose(&mut out, &id, &f);
        assert!(group.equal(&out, &f));

        group.inverse(&mut f);
        group.compose(&mut out, &f, &id);
        assert!(group.equal(&out, &f));

        group.set_id(&mut id);
        assert!(group.is_id(&id));
    }

    #[test]
    fn generic_mpz_identity_has_string_coefficients() {
        let group = GenGroup::new_str("-123456789012345678901234567890123456789").expect("group");
        let id = group.identity().expect("id");
        assert!(group.is_id(&id));

        let (a, b, c) = group.mpz_coefficients(&id).expect("mpz coefficients");
        assert_eq!(a, "1");
        assert!(!b.is_empty());
        assert!(!c.is_empty());
    }

    #[cfg(feature = "gmp")]
    #[test]
    fn generic_mpz_roundtrip_with_gmp_feature() {
        let d = Mpz::from_str_radix("-123456789012345678901234567890123456789", 10).expect("disc");
        let group = GenGroup::new_mpz(&d).expect("group");
        let mut form = group.identity().expect("id");

        let a = Mpz::from_i64(1);
        let b = Mpz::from_i64(1);
        let c = Mpz::from_str_radix("30864197253086419725308641972530864197", 10).expect("c");
        assert!(group.set_from_mpz(&mut form, &a, &b, &c));

        let (out_a, out_b, out_c) = group.get_mpz(&form).expect("get mpz");
        assert_eq!(out_a.to_string_radix(10), "1");
        assert_eq!(out_b.to_string_radix(10), "1");
        assert_eq!(
            out_c.to_string_radix(10),
            "30864197253086419725308641972530864197"
        );
    }
}
