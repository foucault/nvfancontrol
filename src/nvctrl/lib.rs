extern crate libc;
use libc::{c_int, c_char};

use std::ffi::CStr;
use std::mem;
use std::collections::HashMap;

#[link(name="nvctrl_c")]
extern {
    fn nv_init() -> c_int;
    fn nv_deinit() -> c_int;
    fn nv_get_temp(temp: *mut c_int) -> c_int;
    fn nv_get_ctrl_status(status: *mut c_int) -> c_int;
    fn nv_get_fanspeed(speed: *mut c_int) -> c_int;
    fn nv_set_fanspeed(speed: c_int) -> c_int;
    fn nv_get_fanspeed_rpm(speed_rpm: *mut c_int) -> c_int;
    fn nv_get_version(ver: *const *mut c_char) -> c_int;
    fn nv_get_utilization(util: *const *mut c_char) -> c_int;
    fn nv_set_ctrl_type(typ: c_int) -> c_int;
}

#[derive(Debug)]
pub enum NVCtrlFanControlState {
    Auto = 0,
    Manual
}

pub struct NvidiaControl;

impl Drop for NvidiaControl {
    fn drop(&mut self) {
        NvidiaControl::deinit();
    }
}

impl NvidiaControl {
    pub fn new() -> NvidiaControl {
        let ret = NvidiaControl;
        NvidiaControl::init();
        ret
    }

    fn init() -> i32 {
        let ret = unsafe { nv_init() };
        ret as i32
    }

    fn deinit() -> i32 {
        let ret = unsafe { nv_deinit() };
        ret as i32
    }

    pub fn get_temp(&self) -> i32 {
        let mut tmp = -1 as i32;
        unsafe { nv_get_temp(&mut tmp); }
        tmp
    }


    pub fn get_ctrl_status(&self) -> Result<NVCtrlFanControlState, &str> {
        let mut tmp = -1 as i32;
        unsafe { nv_get_ctrl_status(&mut tmp); }
        match tmp {
            0 => Ok(NVCtrlFanControlState::Auto),
            1 => Ok(NVCtrlFanControlState::Manual),
            _ => Err("Unspecified control state")
        }
    }

    pub fn set_ctrl_type(&self, typ: NVCtrlFanControlState) {
        unsafe {
            nv_set_ctrl_type(typ as c_int);
        }
    }

    pub fn get_fanspeed(&self) -> i32 {
        let mut tmp = -1 as i32;
        unsafe { nv_get_fanspeed(&mut tmp); }
        tmp
    }

    pub fn set_fanspeed(&self, speed: i32) {
        unsafe { nv_set_fanspeed(speed as c_int); }
    }

    pub fn get_fanspeed_rpm(&self) -> i32 {
        let mut tmp = -1 as i32;
        unsafe { nv_get_fanspeed_rpm(&mut tmp); }
        tmp
    }

    pub fn get_version(&self) -> String {
        let mut v: *mut c_char = unsafe { mem::uninitialized() };
        unsafe { nv_get_version(&mut v) };
        assert!(!v.is_null());
        unsafe { CStr::from_ptr(v as *const c_char).to_str().unwrap().to_owned() }
    }

    pub fn get_utilization(&self) -> HashMap<&str, i32> {
        let mut v: *mut c_char = unsafe { mem::uninitialized() };
        unsafe { nv_get_utilization(&mut v) };
        assert!(!v.is_null());
        let res = unsafe { CStr::from_ptr(v as *const c_char).to_str().unwrap() };
        let mut ret: HashMap<&str, i32> = HashMap::with_capacity(4);
        let parts = res.split(", ");
        for s in parts {
            let mut split_parts = s.split("=");
            let key = split_parts.next().unwrap();
            let val = split_parts.next().unwrap();
            ret.insert(key, val.parse::<i32>().unwrap());
        }
        ret
    }
}
