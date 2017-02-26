#[cfg(unix)] use std::env;

#[cfg(any(target_os="linux", target_os="freebsd"))]
fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    match env::var("LIBRARY_PATH") {
        Ok(path) => {
            let parts = path.split(":");
            for s in parts {
                println!("cargo:rustc-link-search={}", s);
            }
        },
        Err(_) => {}
    }

    let lib_path = match env::var("NVFANCONTROL_MULTILIB") {
        Ok(val) => {
            match val.parse::<i32>() {
                Ok(1) => "lib32",
                _ => "lib"
            }
        },
        _ => "lib"
    };

    println!("cargo:rustc-link-search={}/build", out_dir);
    println!("cargo:rustc-link-search=/usr/{}", lib_path);
    println!("cargo:rustc-link-search=/usr/local/{}", lib_path);

    #[cfg(feature="dynamic-xnvctrl")] {
        println!("cargo:rustc-link-lib=dylib=XNVCtrl");
    }
    #[cfg(not(feature="dynamic-xnvctrl"))] {
        println!("cargo:rustc-link-lib=static=XNVCtrl");
    }
    println!("cargo:rustc-link-lib=dylib=X11");
    println!("cargo:rustc-link-lib=dylib=Xext");
}

#[cfg(all(target_os="windows", target_arch="x86_64"))]
fn main() {
    println!("cargo:rustc-flags=-L {} -L {} -l static={}", ".", "../../", "nvapi64");
}

#[cfg(all(target_os="windows", target_arch="x86"))]
fn main() {
    println!("cargo:rustc-flags=-L {} -L {} -l static={}", ".", "../../", "nvapi");
}
