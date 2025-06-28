use std::path::PathBuf;

fn main() {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();

    println!("cargo::rerun-if-changed=bpf/uclogic.bpf.c");

    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c")
        .arg(r#"exec -- ${BPF_CC:-clang -target bpfel} "$@""#)
        .arg("sh")
        .args(["-Os", "-g", "-c", "-o"])
        .arg(PathBuf::from(out_dir).join("uclogic.bpf.o"))
        .arg("bpf/uclogic.bpf.c");

    eprintln!("Running: {:?}", cmd.get_args());
    let result = cmd.spawn().unwrap().wait().unwrap();
    if !result.success() {
        if result.code() == Some(127) {
            eprintln!("Compiler command not found, clang or $BPF_CC required");
        }
        panic!("Compiling uclogic.bpf.o failed");
    }
}
