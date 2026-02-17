use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
struct BindgenCallbacks;

impl bindgen::callbacks::ParseCallbacks for BindgenCallbacks {
    fn read_env_var(&self, key: &str) {
        println!("cargo:rerun-if-env-changed={key}");
    }
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let source_root = manifest_dir
        .join("../..")
        .canonicalize()
        .expect("failed to locate source root");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let include_root = out_dir.join("include");
    let include_libqform = include_root.join("libqform");

    copy_qform_headers(&source_root, &include_libqform);

    let gmp_include_dir =
        PathBuf::from(env::var("DEP_GMP_INCLUDE_DIR").expect("DEP_GMP_INCLUDE_DIR not set"));
    let gmp_lib_dir = PathBuf::from(env::var("DEP_GMP_LIB_DIR").expect("DEP_GMP_LIB_DIR not set"));
    let optarith_include_root = discover_dep_include_root(&out_dir, "optarith-sys")
        .expect("could not locate optarith-sys generated include directory");

    let includes = vec![
        source_root.clone(),
        include_root.clone(),
        optarith_include_root,
        gmp_include_dir,
    ];

    let cc = env::var("CC").unwrap_or_else(|_| "cc".to_owned());
    let ar = env::var("AR").unwrap_or_else(|_| "ar".to_owned());

    let bindgen_wrapper = out_dir.join("libqform_bindgen_wrapper.h");
    let bindgen_static_c = out_dir.join("libqform_bindgen_static.c");
    let bindings_rs = out_dir.join("bindings.rs");
    let headers = discover_public_headers(&source_root);

    write_bindgen_wrapper(&bindgen_wrapper, &headers);
    generate_bindings(
        &bindgen_wrapper,
        &bindings_rs,
        &bindgen_static_c,
        &includes,
        &include_root,
    );

    let mut c_files = discover_c_files(&source_root);
    if bindgen_static_c.exists() {
        c_files.push(bindgen_static_c.clone());
    }

    let mut objects = Vec::with_capacity(c_files.len());
    for source in &c_files {
        let stem = source
            .file_stem()
            .expect("file stem")
            .to_string_lossy()
            .to_string();
        let obj = out_dir.join(format!("{stem}.o"));
        compile_object(&cc, source, &obj, &includes);
        objects.push(obj);
    }

    let archive = out_dir.join("libqform_c.a");
    archive_objects(&ar, &archive, &objects);

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=qform_c");
    println!("cargo:rustc-link-lib=static=optarith_c");
    println!("cargo:rustc-link-search=native={}", gmp_lib_dir.display());
    println!("cargo:rustc-link-lib=gmp");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=DEP_GMP_INCLUDE_DIR");
    println!("cargo:rerun-if-env-changed=DEP_GMP_LIB_DIR");
    for path in tracked_paths(&source_root) {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

fn discover_dep_include_root(out_dir: &Path, dep_prefix: &str) -> Option<PathBuf> {
    let build_root = out_dir.parent()?.parent()?;
    let mut matches = Vec::new();

    for entry in fs::read_dir(build_root).ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with(dep_prefix) {
            continue;
        }

        let include_root = entry.path().join("out").join("include");
        if include_root.join("liboptarith").is_dir() {
            matches.push(include_root);
        }
    }

    matches.sort();
    matches.pop()
}

fn copy_qform_headers(source_root: &Path, include_libqform: &Path) {
    fs::create_dir_all(include_libqform.join("dbreps")).expect("create dbreps include dirs");
    fs::create_dir_all(include_libqform.join("tests")).expect("create tests include dirs");

    for entry in fs::read_dir(source_root).expect("read source root") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.extension() == Some(OsStr::new("h")) {
            let dst = include_libqform.join(entry.file_name());
            fs::copy(&path, &dst).unwrap_or_else(|e| {
                panic!("copy {} -> {} failed: {e}", path.display(), dst.display())
            });
        }
    }

    let dbreps = source_root.join("dbreps");
    for entry in fs::read_dir(&dbreps).expect("read dbreps") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.extension() == Some(OsStr::new("h")) {
            let dst = include_libqform.join("dbreps").join(entry.file_name());
            fs::copy(&path, &dst).unwrap_or_else(|e| {
                panic!("copy {} -> {} failed: {e}", path.display(), dst.display())
            });
        }
    }

    let sanity_h = source_root.join("tests").join("sanity_qform.h");
    let sanity_dst = include_libqform.join("tests").join("sanity_qform.h");
    fs::copy(&sanity_h, &sanity_dst).unwrap_or_else(|e| {
        panic!(
            "copy {} -> {} failed: {e}",
            sanity_h.display(),
            sanity_dst.display()
        )
    });
}

fn discover_c_files(source_root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for entry in fs::read_dir(source_root).expect("read source root") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.extension() == Some(OsStr::new("c")) {
            files.push(path);
        }
    }

    let dbreps = source_root.join("dbreps");
    for entry in fs::read_dir(&dbreps).expect("read dbreps") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.extension() == Some(OsStr::new("c")) {
            files.push(path);
        }
    }

    files.push(source_root.join("tests").join("sanity_qform.c"));

    files.sort();
    files
}

fn discover_public_headers(source_root: &Path) -> Vec<String> {
    let mut headers = Vec::new();
    for entry in fs::read_dir(source_root).expect("read source root") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.extension() != Some(OsStr::new("h")) {
            continue;
        }
        headers.push(format!("libqform/{}", entry.file_name().to_string_lossy()));
    }

    let dbreps = source_root.join("dbreps");
    for entry in fs::read_dir(&dbreps).expect("read dbreps") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.extension() != Some(OsStr::new("h")) {
            continue;
        }
        headers.push(format!(
            "libqform/dbreps/{}",
            entry.file_name().to_string_lossy()
        ));
    }

    headers.push("libqform/tests/sanity_qform.h".to_owned());

    headers.sort();
    headers.dedup();
    headers
}

fn write_bindgen_wrapper(wrapper_path: &Path, headers: &[String]) {
    let mut f = fs::File::create(wrapper_path).expect("create bindgen wrapper");
    writeln!(f, "#pragma once").expect("write wrapper");
    for header in headers {
        writeln!(f, "#include \"{header}\"").expect("write include");
    }
}

fn generate_bindings(
    wrapper_path: &Path,
    bindings_path: &Path,
    static_wrappers_c: &Path,
    includes: &[PathBuf],
    include_root: &Path,
) {
    let mut builder = bindgen::Builder::default()
        .header(wrapper_path.display().to_string())
        .parse_callbacks(Box::new(BindgenCallbacks))
        .layout_tests(false)
        .generate_comments(false)
        .allowlist_file(format!("{}/libqform/.*", include_root.display()))
        .clang_arg("-x")
        .clang_arg("c")
        .wrap_static_fns(true)
        .wrap_static_fns_path(static_wrappers_c);

    for include in includes {
        builder = builder.clang_arg(format!("-I{}", include.display()));
    }

    let bindings = builder.generate().expect("generate bindings");
    fs::write(bindings_path, bindings.to_string()).expect("write bindings");
}

fn compile_object(cc: &str, source: &Path, object: &Path, includes: &[PathBuf]) {
    let mut cmd = Command::new(cc);
    cmd.arg("-c")
        .arg(source)
        .arg("-o")
        .arg(object)
        .arg("-std=gnu99")
        .arg("-O3")
        .arg("-DNDEBUG");

    for include in includes {
        cmd.arg("-I").arg(include);
    }

    if let Ok(cflags) = env::var("CFLAGS") {
        for flag in cflags.split_whitespace() {
            cmd.arg(flag);
        }
    }

    let status = cmd.status().unwrap_or_else(|e| {
        panic!("failed to spawn compiler for {}: {e}", source.display());
    });
    assert!(
        status.success(),
        "compile failed for {} with status {status}",
        source.display()
    );
}

fn archive_objects(ar: &str, archive: &Path, objects: &[PathBuf]) {
    let mut cmd = Command::new(ar);
    cmd.arg("rcs").arg(archive);
    for object in objects {
        cmd.arg(object);
    }
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn archiver: {e}"));
    assert!(status.success(), "archiving failed with status {status}");
}

fn tracked_paths(source_root: &Path) -> Vec<PathBuf> {
    let mut paths = vec![
        source_root.join("dbreps"),
        source_root.join("tests").join("sanity_qform.c"),
        source_root.join("tests").join("sanity_qform.h"),
    ];

    for entry in fs::read_dir(source_root).expect("read source root") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.extension() == Some(OsStr::new("h")) || path.extension() == Some(OsStr::new("c")) {
            paths.push(path);
        }
    }

    let dbreps = source_root.join("dbreps");
    for entry in fs::read_dir(dbreps).expect("read dbreps") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.extension() == Some(OsStr::new("h")) || path.extension() == Some(OsStr::new("c")) {
            paths.push(path);
        }
    }

    paths
}
