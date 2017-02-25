use libc::{c_int, c_char, c_void, c_uint};
use std::collections::HashMap;
use std::mem;
use std::ptr;
use std::ffi::{CStr, CString};
use ::{NVCtrlFanControlState, NvFanController};

const XNV_OK: i32 = 1;

type Display = *mut c_void;

#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[repr(u32)]
enum CTRL_TARGET {
    X_SCREEN = 0,
    GPU = 1,
    FRAMELOCK = 2,
    VCSC = 3,
    GVI = 4,
    COOLER = 5,
    THERMAL_SENSOR = 6,
    _3D_VISION_PRO_TRANSCEIVER = 7,
    DISPLAY = 8,
}

#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[repr(u32)]
enum CTRL_ATTR {
    NVIDIA_DRIVER_VERSION = 3,
    PRODUCT_NAME = 0,
    UTILIZATION = 53,
    CORE_TEMPERATURE = 60,
    CORE_THRESHOLD = 61,
    DEFAULT_CORE_THRESHOLD = 62,
    MAX_CORE_THRESHOLD = 63,
    COOLER_MANUAL_CONTROL = 319,
    THERMAL_COOLER_LEVEL = 320,
    THERMAL_COOLER_SPEED = 405,
    THERMAL_COOLER_CURRENT_LEVEL = 417,
}

#[link(name="X11")]
#[link(name="Xext")]
#[link(name="XNVCtrl")]
extern {
    fn XOpenDisplay(name: *const c_char) -> *mut Display;
    fn XCloseDisplay(dpy: *const Display);
    fn XNVCTRLQueryAttribute(dpy: *const Display, id: c_int, mask: c_uint,
                             attribute: CTRL_ATTR, value: *mut c_int) -> c_int;
    fn XNVCTRLQueryStringAttribute(dpy: *const Display, id: c_int, mask: c_uint,
                                   attribute: CTRL_ATTR, value: *const *mut c_char) -> c_int;
    fn XNVCTRLQueryTargetAttribute(dpy: *const Display, target: CTRL_TARGET,
                                   id: c_int, mask: c_uint,
                                   attribute: CTRL_ATTR, value: *mut c_int) -> c_int;
    fn XNVCTRLQueryTargetStringAttribute(dpy: *const Display, target: CTRL_TARGET,
                                         id: c_int, mask: c_uint,
                                         attribute: CTRL_ATTR, value: *const *mut c_char) -> c_int;
    fn XNVCTRLSetTargetAttributeAndGetStatus(dpy: *const Display, target: CTRL_TARGET,
                                             id: c_int, mask: c_uint, attribute: CTRL_ATTR,
                                             value: c_int) -> c_int;
}

pub struct NvidiaControl {
    pub limits: (u16, u16),
    dpy: *mut Display,
    //screen: c_int
}

impl NvidiaControl {
    pub fn init(lim: (u16, u16)) -> Result<NvidiaControl, String> {
        let dpy = unsafe { XOpenDisplay(CString::new(":0").unwrap().as_ptr()) };
        if dpy == ptr::null_mut() {
            return Err(format!("XNVCtrl failed: Could open display :0"));
        }
        // let screen = unsafe { XDefaultScreen(dpy) };
        Ok(NvidiaControl{ limits: lim, dpy: dpy/*, screen: screen */})
    }
}

impl Drop for NvidiaControl {
    fn drop(&mut self) {
        unsafe { XCloseDisplay(self.dpy) };
    }
}

impl NvFanController for NvidiaControl {

    fn get_temp(&self) -> Result<i32, String> {
        let mut tmp = -1 as i32;
        match unsafe {
            XNVCTRLQueryAttribute(self.dpy, 0, 0, CTRL_ATTR::CORE_TEMPERATURE, &mut tmp)
        } {
            XNV_OK => Ok(tmp),
            i => Err(format!("XNVCtrl QueryAttr(CORE_TEMPERATURE) failed; error {}", i))
        }
    }


    fn get_ctrl_status(&self) -> Result<NVCtrlFanControlState, String> {
        let mut tmp = -1 as i32;
        match unsafe {
            XNVCTRLQueryTargetAttribute(self.dpy, CTRL_TARGET::GPU, 0, 0,
                                        CTRL_ATTR::COOLER_MANUAL_CONTROL, &mut tmp)
        } {
            XNV_OK => {
                match tmp {
                    0 => Ok(NVCtrlFanControlState::Auto),
                    1 => Ok(NVCtrlFanControlState::Manual),
                    i => Err(format!("Unspecified control state: {}", i))
                }
            },
            i => Err(format!("XNVCtrl QueryAttr(COOLER_MANUAL_CONTROL) failed; error {}", i))
        }
    }

    fn set_ctrl_type(&self, typ: NVCtrlFanControlState) -> Result<(), String> {
        match unsafe {
            XNVCTRLSetTargetAttributeAndGetStatus(self.dpy, CTRL_TARGET::GPU, 0, 0,
                                                  CTRL_ATTR::COOLER_MANUAL_CONTROL,
                                                  typ as c_int)
        } {
            XNV_OK => Ok(()),
            i => Err(format!("XNVCtrl SetAttr(COOLER_MANUAL_CONTROL) failed; error {}", i))
        }
    }

    fn get_fanspeed(&self) -> Result<i32, String> {
        let mut tmp = -1 as i32;
        match unsafe {
            XNVCTRLQueryTargetAttribute(self.dpy, CTRL_TARGET::COOLER, 0, 0,
                                        CTRL_ATTR::THERMAL_COOLER_CURRENT_LEVEL, &mut tmp)} {
            XNV_OK => Ok(tmp),
            i => Err(format!("XNVCtrl QueryAttr(COOLER_CURRENT_LEVEL) failed; error {}", i))
        }
    }

    fn get_fanspeed_rpm(&self) -> Result<i32, String> {
        let mut tmp = -1 as i32;
        match unsafe {
            XNVCTRLQueryTargetAttribute(self.dpy, CTRL_TARGET::COOLER, 0, 0,
                                        CTRL_ATTR::THERMAL_COOLER_SPEED, &mut tmp)} {
            XNV_OK => Ok(tmp),
            i => Err(format!("XNVCtrl QueryAttr(COOLER_SPEED) failed; error {}", i))
        }
    }

    fn set_fanspeed(&self, speed: i32) -> Result<(), String> {
        let true_speed = self.true_speed(speed);
        match unsafe {
            XNVCTRLSetTargetAttributeAndGetStatus(self.dpy, CTRL_TARGET::COOLER, 0, 0,
                                                  CTRL_ATTR::THERMAL_COOLER_LEVEL,
                                                  true_speed as c_int)
        } {
            XNV_OK => Ok(()),
            i => Err(format!("XNVCtrl SetAttr(THERMAL_COOLER_LEVEL) failed; error {}", i))
        }
    }

    fn get_version(&self) -> Result<String, String> {
        let v: *mut c_char = unsafe { mem::uninitialized() };
        match unsafe {
            XNVCTRLQueryStringAttribute(self.dpy, 0, 0, CTRL_ATTR::NVIDIA_DRIVER_VERSION, &v)
        } {
            XNV_OK => {
                assert!(!v.is_null());
                Ok(unsafe { CStr::from_ptr(v as *const c_char).to_str().unwrap().to_owned() })
            },
            i => Err(format!("XNVCtrl QueryAttr(NVIDIA_DRIVER_VERSION) failed; error {}", i))
        }
    }

    fn get_adapter(&self) -> Result<String, String> {
        let v: *mut c_char = unsafe { mem::uninitialized() };
        match unsafe {
            XNVCTRLQueryTargetStringAttribute(self.dpy, CTRL_TARGET::GPU, 0, 0,
                                              CTRL_ATTR::PRODUCT_NAME, &v)
        } {
            XNV_OK => {
                assert!(!v.is_null());
                Ok(unsafe { CStr::from_ptr(v as *const c_char).to_str().unwrap().to_owned() })
            },
            i => Err(format!("XNVCtrl QueryAttr(PRODUCT_NAME) failed; error {}", i))
        }
    }

    fn get_utilization(&self) -> Result<HashMap<&str, i32>, String> {
        let v: *mut c_char = unsafe { mem::uninitialized() };
        match unsafe {
            XNVCTRLQueryTargetStringAttribute(self.dpy, CTRL_TARGET::GPU, 0, 0,
                                              CTRL_ATTR::UTILIZATION, &v)
        } {
            XNV_OK => {
                assert!(!v.is_null());
                let res = unsafe { CStr::from_ptr(v as *const c_char).to_str().unwrap() };
                let mut ret: HashMap<&str, i32> = HashMap::with_capacity(4);
                let parts = res.split(", ");
                for s in parts {
                    let mut split_parts = s.split('=');
                    let key = split_parts.next().unwrap();
                    let val = split_parts.next().unwrap();
                    ret.insert(key, val.parse::<i32>().unwrap());
                }
                Ok(ret)
            },
            i => Err(format!("XNVCtrl QueryAttr(UTILIZATION) failed; error {}", i))
        }
    }
}
