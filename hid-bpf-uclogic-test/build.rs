fn main() {
    cc::Build::new()
        .file("bpf/uclogic.bpf.c")
        .flag("-DTEST")
        .compile("uclogic-test");
    println!("cargo:rerun-if-changed=bpf/");
}
