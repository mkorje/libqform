#![allow(unsafe_op_in_unsafe_fn)]

use optarith_sys as opt;
use qform_sys as sys;
use std::fs::File;
use std::io::Write;
use std::os::raw::{c_int, c_uint, c_ulong};
use std::time::Instant;

const S64_MAX_BITS: i32 = 59;
const S128_MAX_BITS: i32 = 118;

unsafe extern "C" {
    fn srand(seed: c_uint);
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

fn usage(prog: &str) {
    eprintln!(
        "Usage: {prog} <seed> <groups> <ops> [min_bits max_bits] [-d output.dat] [--s64] [--s128] [--mpz]"
    );
}

unsafe fn q_mpz_const(x: &opt::mpz_t) -> *const sys::__mpz_struct {
    x.as_ptr().cast()
}

unsafe fn time_s64(bits: i32, groups: usize, ops: usize, seed: u32) -> Option<(u128, u128, u128)> {
    let mut group: sys::s64_qform_group_t = std::mem::zeroed();
    let mut rands: opt::gmp_randstate_t = std::mem::zeroed();
    let mut d: opt::mpz_t = std::mem::zeroed();
    let mut a: sys::s64_qform_t = std::mem::zeroed();
    let mut c: sys::s64_qform_t = std::mem::zeroed();

    sys::s64_qform_group_init(&mut group);
    gmp_randinit_default(rands.as_mut_ptr());
    mpz_init(d.as_mut_ptr());

    let mut compose_ns = 0_u128;
    let mut square_ns = 0_u128;
    let mut cube_ns = 0_u128;

    gmp_randseed_ui(rands.as_mut_ptr(), seed as c_ulong);
    srand(seed as c_uint);
    for _ in 0..groups {
        opt::mpz_random_semiprime_discriminant(d.as_mut_ptr(), rands.as_mut_ptr(), bits as c_int);
        sys::s64_qform_group_set_discriminant(&mut group, q_mpz_const(&d));
        sys::qform_random_primeform(&mut group.desc, (&mut a as *mut sys::s64_qform_t).cast());
        let mut b = a;
        let start = Instant::now();
        for _ in 0..ops {
            sys::s64_qform_compose(&mut group, &mut c, &b, &a);
            a = b;
            b = c;
        }
        compose_ns += start.elapsed().as_nanos();
    }

    gmp_randseed_ui(rands.as_mut_ptr(), seed as c_ulong);
    srand(seed as c_uint);
    for _ in 0..groups {
        opt::mpz_random_semiprime_discriminant(d.as_mut_ptr(), rands.as_mut_ptr(), bits as c_int);
        sys::s64_qform_group_set_discriminant(&mut group, q_mpz_const(&d));
        sys::qform_random_primeform(&mut group.desc, (&mut a as *mut sys::s64_qform_t).cast());
        let start = Instant::now();
        for _ in 0..ops {
            sys::s64_qform_square(&mut group, &mut a, &a);
        }
        square_ns += start.elapsed().as_nanos();
    }

    gmp_randseed_ui(rands.as_mut_ptr(), seed as c_ulong);
    srand(seed as c_uint);
    for _ in 0..groups {
        opt::mpz_random_semiprime_discriminant(d.as_mut_ptr(), rands.as_mut_ptr(), bits as c_int);
        sys::s64_qform_group_set_discriminant(&mut group, q_mpz_const(&d));
        sys::qform_random_primeform(&mut group.desc, (&mut a as *mut sys::s64_qform_t).cast());
        let start = Instant::now();
        for _ in 0..ops {
            sys::s64_qform_cube(&mut group, &mut a, &a);
        }
        cube_ns += start.elapsed().as_nanos();
    }

    mpz_clear(d.as_mut_ptr());
    gmp_randclear(rands.as_mut_ptr());
    sys::s64_qform_group_clear(&mut group);
    Some((compose_ns, square_ns, cube_ns))
}

unsafe fn time_s128(bits: i32, groups: usize, ops: usize, seed: u32) -> Option<(u128, u128, u128)> {
    let mut group: sys::s128_qform_group_t = std::mem::zeroed();
    let mut rands: opt::gmp_randstate_t = std::mem::zeroed();
    let mut d: opt::mpz_t = std::mem::zeroed();
    let mut a: sys::s128_qform_t = std::mem::zeroed();
    let mut c: sys::s128_qform_t = std::mem::zeroed();

    sys::s128_qform_group_init(&mut group);
    gmp_randinit_default(rands.as_mut_ptr());
    mpz_init(d.as_mut_ptr());

    let mut compose_ns = 0_u128;
    let mut square_ns = 0_u128;
    let mut cube_ns = 0_u128;

    gmp_randseed_ui(rands.as_mut_ptr(), seed as c_ulong);
    srand(seed as c_uint);
    for _ in 0..groups {
        opt::mpz_random_semiprime_discriminant(d.as_mut_ptr(), rands.as_mut_ptr(), bits as c_int);
        sys::s128_qform_group_set_discriminant(&mut group, q_mpz_const(&d));
        sys::qform_random_primeform(&mut group.desc, (&mut a as *mut sys::s128_qform_t).cast());
        let mut b = a;
        let start = Instant::now();
        for _ in 0..ops {
            sys::s128_qform_compose(&mut group, &mut c, &b, &a);
            a = b;
            b = c;
        }
        compose_ns += start.elapsed().as_nanos();
    }

    gmp_randseed_ui(rands.as_mut_ptr(), seed as c_ulong);
    srand(seed as c_uint);
    for _ in 0..groups {
        opt::mpz_random_semiprime_discriminant(d.as_mut_ptr(), rands.as_mut_ptr(), bits as c_int);
        sys::s128_qform_group_set_discriminant(&mut group, q_mpz_const(&d));
        sys::qform_random_primeform(&mut group.desc, (&mut a as *mut sys::s128_qform_t).cast());
        let start = Instant::now();
        for _ in 0..ops {
            sys::s128_qform_square(&mut group, &mut a, &a);
        }
        square_ns += start.elapsed().as_nanos();
    }

    gmp_randseed_ui(rands.as_mut_ptr(), seed as c_ulong);
    srand(seed as c_uint);
    for _ in 0..groups {
        opt::mpz_random_semiprime_discriminant(d.as_mut_ptr(), rands.as_mut_ptr(), bits as c_int);
        sys::s128_qform_group_set_discriminant(&mut group, q_mpz_const(&d));
        sys::qform_random_primeform(&mut group.desc, (&mut a as *mut sys::s128_qform_t).cast());
        let start = Instant::now();
        for _ in 0..ops {
            sys::s128_qform_cube(&mut group, &mut a, &a);
        }
        cube_ns += start.elapsed().as_nanos();
    }

    mpz_clear(d.as_mut_ptr());
    gmp_randclear(rands.as_mut_ptr());
    sys::s128_qform_group_clear(&mut group);
    Some((compose_ns, square_ns, cube_ns))
}

unsafe fn time_mpz(bits: i32, groups: usize, ops: usize, seed: u32) -> Option<(u128, u128, u128)> {
    let mut group: sys::mpz_qform_group_t = std::mem::zeroed();
    let mut rands: opt::gmp_randstate_t = std::mem::zeroed();
    let mut d: opt::mpz_t = std::mem::zeroed();
    let mut a: sys::mpz_qform_t = std::mem::zeroed();
    let mut b: sys::mpz_qform_t = std::mem::zeroed();
    let mut c: sys::mpz_qform_t = std::mem::zeroed();

    sys::mpz_qform_group_init(&mut group);
    sys::mpz_qform_init(&mut group, &mut a);
    sys::mpz_qform_init(&mut group, &mut b);
    sys::mpz_qform_init(&mut group, &mut c);
    gmp_randinit_default(rands.as_mut_ptr());
    mpz_init(d.as_mut_ptr());

    let mut compose_ns = 0_u128;
    let mut square_ns = 0_u128;
    let mut cube_ns = 0_u128;

    gmp_randseed_ui(rands.as_mut_ptr(), seed as c_ulong);
    srand(seed as c_uint);
    for _ in 0..groups {
        opt::mpz_random_semiprime_discriminant(d.as_mut_ptr(), rands.as_mut_ptr(), bits as c_int);
        sys::mpz_qform_group_set_discriminant(&mut group, q_mpz_const(&d));
        sys::qform_random_primeform(&mut group.desc, (&mut a as *mut sys::mpz_qform_t).cast());
        sys::mpz_qform_set(&mut group, &mut b, &a);
        let start = Instant::now();
        for _ in 0..ops {
            sys::mpz_qform_compose(&mut group, &mut c, &b, &a);
            sys::mpz_qform_set(&mut group, &mut a, &b);
            sys::mpz_qform_set(&mut group, &mut b, &c);
        }
        compose_ns += start.elapsed().as_nanos();
    }

    gmp_randseed_ui(rands.as_mut_ptr(), seed as c_ulong);
    srand(seed as c_uint);
    for _ in 0..groups {
        opt::mpz_random_semiprime_discriminant(d.as_mut_ptr(), rands.as_mut_ptr(), bits as c_int);
        sys::mpz_qform_group_set_discriminant(&mut group, q_mpz_const(&d));
        sys::qform_random_primeform(&mut group.desc, (&mut a as *mut sys::mpz_qform_t).cast());
        let start = Instant::now();
        for _ in 0..ops {
            sys::mpz_qform_square(&mut group, &mut a, &a);
        }
        square_ns += start.elapsed().as_nanos();
    }

    gmp_randseed_ui(rands.as_mut_ptr(), seed as c_ulong);
    srand(seed as c_uint);
    for _ in 0..groups {
        opt::mpz_random_semiprime_discriminant(d.as_mut_ptr(), rands.as_mut_ptr(), bits as c_int);
        sys::mpz_qform_group_set_discriminant(&mut group, q_mpz_const(&d));
        sys::qform_random_primeform(&mut group.desc, (&mut a as *mut sys::mpz_qform_t).cast());
        let start = Instant::now();
        for _ in 0..ops {
            sys::mpz_qform_cube(&mut group, &mut a, &a);
        }
        cube_ns += start.elapsed().as_nanos();
    }

    mpz_clear(d.as_mut_ptr());
    gmp_randclear(rands.as_mut_ptr());
    sys::mpz_qform_clear(&mut group, &mut a);
    sys::mpz_qform_clear(&mut group, &mut b);
    sys::mpz_qform_clear(&mut group, &mut c);
    sys::mpz_qform_group_clear(&mut group);
    Some((compose_ns, square_ns, cube_ns))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        usage(&args[0]);
        std::process::exit(1);
    }

    let seed = args[1]
        .parse::<u32>()
        .expect("seed should be an unsigned integer");
    let groups = args[2]
        .parse::<usize>()
        .expect("groups should be a positive integer");
    let ops = args[3]
        .parse::<usize>()
        .expect("ops should be a positive integer");
    if groups == 0 || ops == 0 {
        eprintln!("groups and ops must both be non-zero");
        std::process::exit(1);
    }

    let mut min_bits = 16_i32;
    let mut max_bits = 80_i32;
    let mut dump_file = None;

    let mut do_s64 = false;
    let mut do_s128 = false;
    let mut do_mpz = false;

    let mut idx = 4;
    if args.len() >= 6
        && !args[4].starts_with('-')
        && !args[5].starts_with('-')
        && args[4].chars().all(|c| c.is_ascii_digit())
        && args[5].chars().all(|c| c.is_ascii_digit())
    {
        min_bits = args[4].parse::<i32>().expect("invalid min_bits");
        max_bits = args[5].parse::<i32>().expect("invalid max_bits");
        idx = 6;
    }

    while idx < args.len() {
        match args[idx].as_str() {
            "-d" => {
                if idx + 1 >= args.len() {
                    usage(&args[0]);
                    std::process::exit(1);
                }
                dump_file =
                    Some(File::create(&args[idx + 1]).expect("failed to create output file"));
                idx += 2;
            }
            "--s64" => {
                do_s64 = true;
                idx += 1;
            }
            "--s128" => {
                do_s128 = true;
                idx += 1;
            }
            "--mpz" => {
                do_mpz = true;
                idx += 1;
            }
            _ => {
                usage(&args[0]);
                std::process::exit(1);
            }
        }
    }

    if !do_s64 && !do_s128 && !do_mpz {
        do_s64 = true;
        do_s128 = true;
        do_mpz = true;
    }

    if min_bits > max_bits {
        std::mem::swap(&mut min_bits, &mut max_bits);
    }
    min_bits = min_bits.max(2);

    println!("# bits\tbackend\tcompose(ns/op)\tsquare(ns/op)\tcube(ns/op)");
    for bits in min_bits..=max_bits {
        if do_s64 && bits <= S64_MAX_BITS {
            let Some((compose, square, cube)) = (unsafe { time_s64(bits, groups, ops, seed) })
            else {
                continue;
            };
            let denom = (groups * ops) as f64;
            let c = compose as f64 / denom;
            let s = square as f64 / denom;
            let u = cube as f64 / denom;
            println!("{bits}\ts64\t{c:.3}\t{s:.3}\t{u:.3}");
            if let Some(f) = dump_file.as_mut() {
                writeln!(f, "{bits}\ts64\t{c:.3}\t{s:.3}\t{u:.3}").expect("write failed");
            }
        }

        if do_s128 && bits <= S128_MAX_BITS {
            let Some((compose, square, cube)) = (unsafe { time_s128(bits, groups, ops, seed) })
            else {
                continue;
            };
            let denom = (groups * ops) as f64;
            let c = compose as f64 / denom;
            let s = square as f64 / denom;
            let u = cube as f64 / denom;
            println!("{bits}\ts128\t{c:.3}\t{s:.3}\t{u:.3}");
            if let Some(f) = dump_file.as_mut() {
                writeln!(f, "{bits}\ts128\t{c:.3}\t{s:.3}\t{u:.3}").expect("write failed");
            }
        }

        if do_mpz {
            let Some((compose, square, cube)) = (unsafe { time_mpz(bits, groups, ops, seed) })
            else {
                continue;
            };
            let denom = (groups * ops) as f64;
            let c = compose as f64 / denom;
            let s = square as f64 / denom;
            let u = cube as f64 / denom;
            println!("{bits}\tmpz\t{c:.3}\t{s:.3}\t{u:.3}");
            if let Some(f) = dump_file.as_mut() {
                writeln!(f, "{bits}\tmpz\t{c:.3}\t{s:.3}\t{u:.3}").expect("write failed");
            }
        }
    }
}
