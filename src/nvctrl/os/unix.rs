use libc::{c_int, c_char};
use std::collections::HashMap;
use std::mem;
use std::ffi::CStr;
use ::{NVCtrlFanControlState, NvFanController, NvidiaControl};

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
    fn nv_get_adapter(adapter: *const *mut c_char) -> c_int;
    fn nv_set_ctrl_type(typ: c_int) -> c_int;
}

const XNV_OK: i32 = 1;

impl NvFanController for NvidiaControl {
    fn init() -> Result<(), String> {
        match unsafe { nv_init() } {
            XNV_OK => Ok(()),
            i => Err(format!("XNVCtrl init() failed; error: {}", i))
        }
    }

    fn deinit() -> Result<(), String> {
        match unsafe { nv_deinit() } {
            XNV_OK => Ok(()),
            i => Err(format!("XNVCtrl deinit() failed; error {}", i))
        }
    }

    fn get_temp(&self) -> Result<i32, String> {
        let mut tmp = -1 as i32;
        match unsafe { nv_get_temp(&mut tmp) } {
            XNV_OK => Ok(tmp),
            i => Err(format!("XNVCtrl get_temp() failed; error {}", i))
        }
    }


    fn get_ctrl_status(&self) -> Result<NVCtrlFanControlState, String> {
        let mut tmp = -1 as i32;
        match unsafe { nv_get_ctrl_status(&mut tmp) } {
            XNV_OK => {
                match tmp {
                    0 => Ok(NVCtrlFanControlState::Auto),
                    1 => Ok(NVCtrlFanControlState::Manual),
                    i => Err(format!("Unspecified control state: {}", i))
                }
            },
            i => Err(format!("XNVCtrl get_ctrl_status() failed; error {}", i))
        }
    }

    fn set_ctrl_type(&self, typ: NVCtrlFanControlState) -> Result<(), String> {
        match unsafe { nv_set_ctrl_type(typ as c_int) } {
            XNV_OK => Ok(()),
            i => Err(format!("XNVCtrl set_ctrl_type() failed; error {}", i))
        }
    }

    fn get_fanspeed(&self) -> Result<i32, String> {
        let mut tmp = -1 as i32;
        match unsafe { nv_get_fanspeed(&mut tmp) } {
            XNV_OK => Ok(tmp),
            i => Err(format!("XNVCtrl get_fanspeed() failed; error {}", i))
        }
    }

    fn get_fanspeed_rpm(&self) -> Result<i32, String> {
        let mut tmp = -1 as i32;
        match unsafe { nv_get_fanspeed_rpm(&mut tmp) } {
            XNV_OK => Ok(tmp),
            i => Err(format!("XNVCtrl get_fanspeed_rpm() failed; error {}", i))
        }
    }

    fn set_fanspeed(&self, speed: i32) -> Result<(), String> {
        let true_speed = self.true_speed(speed);
        match unsafe { nv_set_fanspeed(true_speed as c_int) } {
            XNV_OK => Ok(()),
            i => Err(format!("XNVCtrl set_fanspeed() failed; error {}", i))
        }
    }

    fn get_version(&self) -> Result<String, String> {
        let mut v: *mut c_char = unsafe { mem::uninitialized() };
        match unsafe { nv_get_version(&mut v) } {
            XNV_OK => {
                assert!(!v.is_null());
                Ok(unsafe { CStr::from_ptr(v as *const c_char).to_str().unwrap().to_owned() })
            },
            i => Err(format!("XNVCtrl get_version() failed; error {}", i))
        }
    }

    fn get_adapter(&self) -> Result<String, String> {
        let mut v: *mut c_char = unsafe { mem::uninitialized() };
        match unsafe { nv_get_adapter(&mut v) } {
            XNV_OK => {
                assert!(!v.is_null());
                Ok(unsafe { CStr::from_ptr(v as *const c_char).to_str().unwrap().to_owned() })
            },
            i => Err(format!("XNVCtrl get_version() failed; error {}", i))
        }
    }

    fn get_utilization(&self) -> Result<HashMap<&str, i32>, String> {
        let mut v: *mut c_char = unsafe { mem::uninitialized() };
        match unsafe { nv_get_utilization(&mut v) } {
            XNV_OK => {
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
                Ok(ret)
            },
            i => Err(format!("XNVCtrl get_utilization() failed; error {}", i))
        }
    }
}
