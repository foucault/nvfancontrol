
#[cfg(unix)] use std::process::{Command, Stdio};
#[cfg(unix)] use std::env;

#[cfg(any(target_os="linux", target_os="freebsd"))]
fn main() {
    let ret = Command::new("make")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .unwrap();
    assert!(ret.success());
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-flags=-L {}/build -l static={} -l {} -l {}",
             out_dir, "nvctrl_c", "X11", "Xext");
}

#[cfg(all(target_os="windows", target_arch="x86_64"))]
fn main() {
    println!("cargo:rustc-flags=-L {} -L {} -l static={}", ".", "../../", "nvapi64");
}

#[cfg(all(target_os="windows", target_arch="x86"))]
fn main() {
    println!("cargo:rustc-flags=-L {} -L {} -l static={}", ".", "../../", "nvapi");
}
