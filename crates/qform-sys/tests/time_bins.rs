use std::process::Command;

#[test]
fn time_qforms_smoke() {
    let bin = env!("CARGO_BIN_EXE_time_qforms");
    let out = Command::new(bin)
        .args(["123", "2", "12", "16", "20"])
        .output()
        .expect("failed to run time_qforms");
    assert!(
        out.status.success(),
        "time_qforms failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
