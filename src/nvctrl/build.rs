use std::process::{Command, Stdio};
use std::env;

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
