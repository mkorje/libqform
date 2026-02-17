#![allow(unsafe_op_in_unsafe_fn)]

use optarith_sys as opt;
use qform_sys as sys;
use std::os::raw::{c_int, c_uint, c_ulong};

unsafe extern "C" {
    fn srand(seed: c_uint);
    fn rand() -> c_int;
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

#[test]
fn basic_ops_match_c_sanity_driver() {
    let iterations = env_usize("QFORM_TEST_ITERATIONS", 10_000);
    let groups = env_usize("QFORM_TEST_GROUPS", 100);
    let min_bits = env_usize("QFORM_TEST_MIN_BITS", 16);
    let max_bits = env_usize("QFORM_TEST_MAX_BITS", 140);
    let seed = env_usize("QFORM_TEST_SEED", 1) as c_uint;

    unsafe {
        let mut rands: opt::gmp_randstate_t = std::mem::zeroed();
        let mut discriminant: opt::mpz_t = std::mem::zeroed();
        let mut sanity_group: sys::sanity_qform_group_t = std::mem::zeroed();
        let mut a: sys::sanity_qform_t = std::mem::zeroed();
        let mut b: sys::sanity_qform_t = std::mem::zeroed();
        let mut c: sys::sanity_qform_t = std::mem::zeroed();

        gmp_randinit_default(rands.as_mut_ptr());
        gmp_randseed_ui(rands.as_mut_ptr(), seed as c_ulong);
        mpz_init(discriminant.as_mut_ptr());
        srand(seed);

        sys::sanity_qform_group_init(&mut sanity_group);
        sys::sanity_qform_init(&mut sanity_group, &mut a);
        sys::sanity_qform_init(&mut sanity_group, &mut b);
        sys::sanity_qform_init(&mut sanity_group, &mut c);

        for bits in min_bits..=max_bits {
            for _ in 0..groups {
                opt::mpz_random_semiprime_discriminant(
                    discriminant.as_mut_ptr(),
                    rands.as_mut_ptr(),
                    bits as c_int,
                );
                sys::sanity_qform_group_set_discriminant(
                    &mut sanity_group,
                    q_mpz_mut(&mut discriminant),
                );

                sys::qform_random_primeform(
                    &mut sanity_group.desc,
                    (&mut a as *mut sys::sanity_qform_t).cast(),
                );
                sys::qform_random_primeform(
                    &mut sanity_group.desc,
                    (&mut b as *mut sys::sanity_qform_t).cast(),
                );
                sys::qform_random_primeform(
                    &mut sanity_group.desc,
                    (&mut c as *mut sys::sanity_qform_t).cast(),
                );

                for _ in 0..iterations {
                    let action = rand() % 3;
                    let dest = rand() % 3;
                    match (action, dest) {
                        (0, 0) => sys::sanity_qform_compose(&mut sanity_group, &mut a, &b, &c),
                        (0, 1) => sys::sanity_qform_compose(&mut sanity_group, &mut b, &c, &a),
                        (0, _) => sys::sanity_qform_compose(&mut sanity_group, &mut c, &a, &b),
                        (1, 0) => sys::sanity_qform_square(&mut sanity_group, &mut a, &a),
                        (1, 1) => sys::sanity_qform_square(&mut sanity_group, &mut b, &b),
                        (1, _) => sys::sanity_qform_square(&mut sanity_group, &mut c, &c),
                        (2, 0) => sys::sanity_qform_cube(&mut sanity_group, &mut a, &a),
                        (2, 1) => sys::sanity_qform_cube(&mut sanity_group, &mut b, &b),
                        _ => sys::sanity_qform_cube(&mut sanity_group, &mut c, &c),
                    }
                }
            }
        }

        sys::sanity_qform_clear(&mut sanity_group, &mut a);
        sys::sanity_qform_clear(&mut sanity_group, &mut b);
        sys::sanity_qform_clear(&mut sanity_group, &mut c);
        sys::sanity_qform_group_clear(&mut sanity_group);
        mpz_clear(discriminant.as_mut_ptr());
        gmp_randclear(rands.as_mut_ptr());
    }
}
