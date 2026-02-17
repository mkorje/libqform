#![allow(unsafe_op_in_unsafe_fn)]

use optarith_sys as opt;
use qform_sys as sys;
use std::ffi::c_void;
use std::os::raw::{c_int, c_uint, c_ulong};

unsafe extern "C" {
    fn srand(seed: c_uint);
    fn free(ptr: *mut c_void);
    #[link_name = "__gmp_randinit_default"]
    fn gmp_randinit_default(state: *mut opt::__gmp_randstate_struct);
    #[link_name = "__gmp_randseed_ui"]
    fn gmp_randseed_ui(state: *mut opt::__gmp_randstate_struct, seed: c_ulong);
    #[link_name = "__gmp_randclear"]
    fn gmp_randclear(state: *mut opt::__gmp_randstate_struct);
    #[link_name = "__gmpz_init"]
    fn mpz_init(x: *mut opt::__mpz_struct);
    #[link_name = "__gmpz_clear"]
    fn mpz_clear(x: *mut opt::__mpz_struct);
    #[link_name = "__gmpz_urandomb"]
    fn mpz_urandomb(
        rop: *mut opt::__mpz_struct,
        state: *mut opt::__gmp_randstate_struct,
        nbits: c_ulong,
    );
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|x| x.parse::<usize>().ok())
        .unwrap_or(default)
}

unsafe fn q_mpz_mut(x: &mut opt::mpz_t) -> *mut sys::__mpz_struct {
    x.as_mut_ptr().cast()
}

unsafe fn opt_group_mut(x: &mut sys::group_t) -> *mut opt::group_t {
    (x as *mut sys::group_t).cast()
}

unsafe fn costs_ptr(x: *const sys::group_cost_t) -> *const opt::group_cost_t {
    x.cast()
}

#[test]
fn qform_pow_matches_c_reference_paths() {
    let min_bits = env_usize("QFORM_TEST_MIN_BITS", 16);
    let max_bits = env_usize("QFORM_TEST_MAX_BITS", 140);
    let min_exp_bits = env_usize("QFORM_TEST_MIN_EXP_BITS", 256);
    let max_exp_bits = env_usize("QFORM_TEST_MAX_EXP_BITS", 1024);
    let exp_step = env_usize("QFORM_TEST_EXP_STEP_BITS", 64).max(1);
    let seed = env_usize("QFORM_TEST_SEED", 1) as c_uint;

    unsafe {
        let mut rands: opt::gmp_randstate_t = std::mem::zeroed();
        let mut discriminant: opt::mpz_t = std::mem::zeroed();
        let mut ex: opt::mpz_t = std::mem::zeroed();
        let mut sanity_group: sys::sanity_qform_group_t = std::mem::zeroed();
        let mut b: sys::sanity_qform_t = std::mem::zeroed();
        let mut p1: sys::sanity_qform_t = std::mem::zeroed();
        let mut p2: sys::sanity_qform_t = std::mem::zeroed();
        let mut pow: opt::group_pow_t = std::mem::zeroed();

        gmp_randinit_default(rands.as_mut_ptr());
        gmp_randseed_ui(rands.as_mut_ptr(), seed as c_ulong);
        mpz_init(discriminant.as_mut_ptr());
        mpz_init(ex.as_mut_ptr());
        srand(seed);

        sys::sanity_qform_group_init(&mut sanity_group);
        sys::sanity_qform_init(&mut sanity_group, &mut b);
        sys::sanity_qform_init(&mut sanity_group, &mut p1);
        sys::sanity_qform_init(&mut sanity_group, &mut p2);
        opt::group_pow_init(&mut pow, opt_group_mut(&mut sanity_group.desc.group));

        for bits in min_bits..=max_bits {
            let mut exp_bits = min_exp_bits;
            while exp_bits < max_exp_bits {
                opt::mpz_random_semiprime_discriminant(
                    discriminant.as_mut_ptr(),
                    rands.as_mut_ptr(),
                    bits as c_int,
                );
                sys::sanity_qform_group_set_discriminant(
                    &mut sanity_group,
                    q_mpz_mut(&mut discriminant),
                );

                mpz_urandomb(ex.as_mut_ptr(), rands.as_mut_ptr(), exp_bits as c_ulong);
                sys::qform_random_primeform(
                    &mut sanity_group.desc,
                    (&mut b as *mut sys::sanity_qform_t).cast(),
                );

                opt::group_pow_naf_r2l(
                    &mut pow,
                    (&mut p1 as *mut sys::sanity_qform_t).cast(),
                    (&b as *const sys::sanity_qform_t).cast(),
                    ex.as_ptr(),
                );

                let mut rep_count = 0_i32;
                let mut frep_count = 0_i32;
                let rep = opt::rep_prune_closest(
                    &mut rep_count,
                    ex.as_ptr(),
                    costs_ptr(std::ptr::addr_of!(sys::s64_qform_costs)),
                    512,
                );
                let frep = opt::factored_rep(&mut frep_count, rep, rep_count);

                opt::group_pow_factored23(
                    &mut pow,
                    (&mut p2 as *mut sys::sanity_qform_t).cast(),
                    (&b as *const sys::sanity_qform_t).cast(),
                    frep,
                    frep_count,
                );
                free(rep.cast());
                free(frep.cast());

                assert_ne!(
                    sys::sanity_qform_equal(&mut sanity_group, &p1, &p2),
                    0,
                    "pow mismatch for bits={bits}, exp_bits={exp_bits}"
                );

                exp_bits += exp_step;
            }
        }

        opt::group_pow_clear(&mut pow);
        sys::sanity_qform_clear(&mut sanity_group, &mut b);
        sys::sanity_qform_clear(&mut sanity_group, &mut p1);
        sys::sanity_qform_clear(&mut sanity_group, &mut p2);
        sys::sanity_qform_group_clear(&mut sanity_group);
        mpz_clear(ex.as_mut_ptr());
        mpz_clear(discriminant.as_mut_ptr());
        gmp_randclear(rands.as_mut_ptr());
    }
}
