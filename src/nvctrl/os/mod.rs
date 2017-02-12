#[cfg(any(target_os="linux", target_os="freebsd"))]
pub mod unix;
#[cfg(any(target_os="linux", target_os="freebsd"))]
pub mod windows {}

#[cfg(target_os="windows")]
pub mod windows;
#[cfg(target_os="windows")]
pub mod unix {}
