#![allow(unsafe_op_in_unsafe_fn)]

use qform_sys as sys;
use std::ffi::CString;
use std::os::raw::c_int;

unsafe extern "C" {
    #[link_name = "__gmpz_init"]
    fn mpz_init(x: *mut sys::__mpz_struct);
    #[link_name = "__gmpz_clear"]
    fn mpz_clear(x: *mut sys::__mpz_struct);
    #[link_name = "__gmpz_set_str"]
    fn mpz_set_str(x: *mut sys::__mpz_struct, s: *const i8, base: c_int) -> c_int;
}

#[test]
fn generic_group_can_select_mpz_backend() {
    // 128-bit negative discriminant represented as decimal text.
    let d = CString::new("-170141183460469231731687303715884105731").expect("cstring");

    unsafe {
        let mut g: sys::gen_qform_group_t = std::mem::zeroed();
        let mut f: sys::gen_qform_t = std::mem::zeroed();
        let mut d_mpz: sys::mpz_t = std::mem::zeroed();

        sys::gen_qform_group_init(&mut g);
        sys::gen_qform_init(&mut g, &mut f);
        mpz_init(d_mpz.as_mut_ptr());
        assert_eq!(mpz_set_str(d_mpz.as_mut_ptr(), d.as_ptr(), 10), 0);

        sys::gen_qform_group_set_discriminant(&mut g, d_mpz.as_ptr());
        assert!(
            g.logD > sys::s128_qform_group_max_bits as c_int,
            "expected mpz backend"
        );

        sys::gen_qform_set_id(&mut g, &mut f);
        assert_ne!(sys::gen_qform_is_id(&mut g, &f), 0);
        assert_eq!(sys::gen_qform_is_ambiguous(&mut g, &f), 0);

        sys::gen_qform_inverse(&mut g, &mut f);
        sys::gen_qform_reduce(&mut g, &mut f);
        assert_ne!(sys::gen_qform_is_id(&mut g, &f), 0);

        mpz_clear(d_mpz.as_mut_ptr());
        sys::gen_qform_clear(&mut g, &mut f);
        sys::gen_qform_group_clear(&mut g);
    }
}
