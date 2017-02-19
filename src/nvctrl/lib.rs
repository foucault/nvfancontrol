extern crate libc;

#[cfg(target_os="windows")]
#[macro_use] extern crate lazy_static;

#[cfg(target_os="windows")]
extern crate libloading;

use std::collections::HashMap;

#[cfg(any(target_os="linux", target_os="freebsd"))]
pub use self::os::unix::*;

#[cfg(target_os="windows")]
pub use self::os::windows::*;

pub mod os;

pub trait NvFanController {
    fn init() -> Result<(), String>;
    fn deinit() -> Result<(), String>;
    fn get_temp(&self) -> Result<i32, String>;
    fn get_ctrl_status(&self) -> Result<NVCtrlFanControlState, String>;
    fn set_ctrl_type(&self, NVCtrlFanControlState) -> Result<(), String>;
    fn get_fanspeed(&self) -> Result<i32, String>;
    fn get_fanspeed_rpm(&self) -> Result<i32, String>;
    fn set_fanspeed(&self, speed: i32) -> Result<(), String>;
    fn get_version(&self) -> Result<String, String>;
    fn get_adapter(&self) -> Result<String, String>;
    fn get_utilization(&self) -> Result<HashMap<&str, i32>, String>;
}

#[derive(Debug)]
pub enum NVCtrlFanControlState {
    Auto = 0,
    Manual
}

pub struct NvidiaControl {
    limits: (u16, u16)
}

impl Drop for NvidiaControl {
    fn drop(&mut self) {
        match NvidiaControl::deinit() {
            Ok(_) | Err(_) => {},
        }
    }
}

impl NvidiaControl {
    pub fn new(lim: Option<(u16, u16)>) -> Result<NvidiaControl, String> {
        match NvidiaControl::init() {
            Ok(()) => Ok(NvidiaControl {
                limits: match lim {
                    Some((low, high)) => {
                        if high > 100 {
                            (low, 100)
                        } else {
                            (low, high)
                        }
                    },
                    None => (0, 100)
                }
            }),
            Err(e) => Err(e)
        }
    }

    fn true_speed(&self, speed: i32) -> u16 {
        let true_speed: u16;
        let (low, high) = self.limits;
        if speed < low as i32 {
            true_speed = low;
        } else if speed > high as i32 {
            true_speed = high;
        } else {
            true_speed = speed as u16;
        }

        true_speed
    }

}
