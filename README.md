libqform
========

Ideal Class Group Arithmetic in Imaginary Quadratic Number Fields.

This requires liboptarith and it should be checked out in a sibling folder
otherwise you'll need to adjust the SConstruct include path.

You will also need scons to build these packages and a fairly recent version of gcc.

There are some tests in the tests/ folder.

Documentation will gradually be updated and hopefully a C++ class wrapper.

Rust workspace
==============

This repository now includes a Rust workspace with two crates:

- `qform-sys`: raw FFI + bundled C build for libqform
- `qform`: safe wrappers over `qform-sys`

The safe crate exposes wrappers for:

- `s64` qforms
- `s128` qforms
- generic qforms (`gen_qform`) with dynamic backend dispatch (`s64`/`s128`/`mpz`)

```sh
cargo test --workspace
```

Dependency model
----------------

`qform-sys` links against `optarith-sys` and consumes GMP from
`gmp-mpfr-sys` (not from system GMP).

`qform` depends on `qform-sys` and the Rust `optarith` crate.

In this workspace, `optarith`/`optarith-sys` are pulled from GitHub
(`https://github.com/mkorje/liboptarith`).

MPZ feature
-----------

The `gmp` feature is enabled by default on `qform`.

To build without GMP-specific Rust helpers:

```sh
cargo test -p qform --no-default-features
```

When enabled, `gmp` exposes `Mpz` utilities and MPZ-specific helpers on `GenGroup`.
