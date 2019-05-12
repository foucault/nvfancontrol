use libc::{c_int, c_char, c_uchar, c_void, c_uint};
use std::collections::HashMap;
use std::{mem, ptr, slice};
use std::ffi::CStr;
use ::{NVCtrlFanControlState, NvFanController};

const XNV_OK: i32 = 1;

type Display = *mut c_void;

/// XNVCtrl target
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

/// XNVCtrl Attribute (non exhaustive)
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

/// XNVCtrl Binary Attribute (non exchaustive)
#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[repr(u32)]
enum BIN_ATTR {
    EDID = 0,
    MODELINES = 1,
    METAMODES = 2,
    XSCREENS_USING_GPU = 3,
    GPUS_USED_BY_XSCREEN = 4,
    GPUS_USING_FRAMELOCK = 5,
    DISPLAY_VIEWPORT = 6,
    FRAMELOCKS_USED_BY_GPU = 7,
    GPUS_USING_VCSC = 8,
    VCSCS_USED_BY_GPU = 9,
    COOLERS_USED_BY_GPU = 10,
    GPUS_USED_BY_LOGICAL_XSCREEN = 11,
    THERMAL_SENSORS_USED_BY_GPU = 12,
    GLASSES_PAIRED_TO_3D_VISION_PRO_TRANSCEIVERS = 13,
    DISPLAY_TARGETS = 14,
    DISPLAYS_CONNECTED_TO_GPU = 15,
    METAMODES_VERSION_2 = 16,
    DISPLAYS_ENABLED_ON_XSCREEN = 17,
    DISPLAYS_ASSIGNED_TO_XSCREEN = 18,
    GPU_FLAGS = 19,
    DATA_DISPLAYS_ON_GPU = 20
}

/// All required foreign functions that are used in this library
#[allow(dead_code)]
#[link(name="X11")]
#[link(name="Xext")]
#[link(name="XNVCtrl")]
extern {
    /// Opens a new X11 display with the specified name
    ///
    /// **Arguments**
    ///
    /// * `name` - Name of the display to open
    fn XOpenDisplay(name: *const c_char) -> *mut Display;

    /// Closes the specified display
    ///
    /// ***Arguments**
    ///
    /// * `dpy` - The `Display` to close
    fn XCloseDisplay(dpy: *const Display);

    /// XNVCtrl generic int query
    ///
    /// **Arguments**
    ///
    /// * `dpy` - The current X11 `Display`
    /// * `screen` - Screen id
    /// * `mask` - Attribute mask
    /// * `attribute` - Target attribute to query (`CTRL_ATTR`)
    /// * `value` - The value of the attribute that will be populated upon function call
    fn XNVCTRLQueryAttribute(dpy: *const Display, screen: c_int, mask: c_uint,
                             attribute: CTRL_ATTR, value: *mut c_int) -> c_int;

    /// XNVCtrl string query
    ///
    /// **Arguments**
    ///
    /// * `dpy` - The current X11 `Display`
    /// * `screen` - Screen id
    /// * `mask` - Attribute mask
    /// * `attribute` - Target attribute to query (`CTRL_ATTR`)
    /// * `value` - The value of the attribute that will be populated upon function call
    fn XNVCTRLQueryStringAttribute(dpy: *const Display, screen: c_int, mask: c_uint,
                                   attribute: CTRL_ATTR, value: *const *mut c_char) -> c_int;

    /// XNVCtrl int query with target
    ///
    /// **Arguments**
    ///
    /// * `dpy` - The current X11 `Display`
    /// * `target` - Attribute query target (`CTRL_TARGET`)
    /// * `id` - GPU id
    /// * `mask` - Attribute mask
    /// * `attribute` - Attribute to query (`CTRL_ATTR`)
    /// * `value` - The value of the attribute that will be populated upon function call
    fn XNVCTRLQueryTargetAttribute(dpy: *const Display, target: CTRL_TARGET,
                                   id: c_int, mask: c_uint,
                                   attribute: CTRL_ATTR, value: *mut c_int) -> c_int;

    /// XNVCtrl string query with target
    ///
    /// **Arguments**
    ///
    /// * `dpy` - The current X11 `Display`
    /// * `target` - Attribute query target (`CTRL_TARGET`)
    /// * `id` - GPU id
    /// * `mask` - Attribute mask
    /// * `attribute` - Attribute to query (`CTRL_ATTR`)
    /// * `value` - The value of the attribute that will be populated upon function call
    fn XNVCTRLQueryTargetStringAttribute(dpy: *const Display, target: CTRL_TARGET,
                                         id: c_int, mask: c_uint,
                                         attribute: CTRL_ATTR, value: *const *mut c_char) -> c_int;

    /// XNVCtrl set target attribute
    ///
    /// **Arguments**
    ///
    /// * `dpy` - The current X11 `Display`
    /// * `target` - Attribute modification target (`CTRL_TARGET`)
    /// * `id` - GPU id
    /// * `mask` - Attribute mask
    /// * `attribute` - Attribute to set (`CTRL_ATTR`)
    /// * `value` - The value of the attribute to set
    fn XNVCTRLSetTargetAttributeAndGetStatus(dpy: *const Display, target: CTRL_TARGET,
                                             id: c_int, mask: c_uint, attribute: CTRL_ATTR,
                                             value: c_int) -> c_int;

    /// XNVCtrl get target count
    ///
    /// **Arguments**
    ///
    /// * `dpy` - The current X11 `Display`
    /// * `target` - Attribute to count (`CTRL_TARGET`)
    /// * `value` - The value of the attribute
    fn XNVCTRLQueryTargetCount(dpy: *const Display, target: CTRL_TARGET,
                               value: *mut c_int) -> c_int;

    /// XNVCtrl get target binary data
    ///
    /// **Arguments**
    ///
    /// * `dpy` - The current X11 `Display`
    /// * `target` - Attribute to count (`CTRL_TARGET`)
    /// * `id` - GPU id
    /// * `mask` - Attribute mask
    /// * `attribute` - Attribute to get (`CTRL_ATTR`)
    /// * `data` - The value of the attribute
    /// * `len` - The length of the data
    fn XNVCTRLQueryTargetBinaryData(dpy: *const Display, target: CTRL_TARGET,
                                    id: c_int, mask: c_uint, attribute: BIN_ATTR,
                                    data: *const *mut c_uchar, len: *mut c_int) -> c_int;
}

#[allow(dead_code)]
struct UnixGPU {
    id: u32,
    coolers: Vec<u32>
}

/// NvidiaControl is the main struct that monitors and controls the
/// GPU fan state in addition with thermal and general information.
pub struct NvidiaControl {
    /// Current lower and upper limits
    pub limits: (u16, u16),
    dpy: *mut Display,
    _gpus: Vec<UnixGPU>
}

impl NvidiaControl {

    /// Initialises the native library corresponding to the current OS.
    /// `init()` should be called when calling `NvidiaControl::new()` so
    /// there is no need to call it directly.
    pub fn init(lim: (u16, u16)) -> Result<NvidiaControl, String> {
        let dpy = unsafe { XOpenDisplay(ptr::null()) };
        let mut gpu_count = -1 as i32;
        let mut gpus: Vec<UnixGPU>;


        if dpy.is_null() {
            return Err(format!("XNVCtrl failed: XOpenDisplay failed; is $DISPLAY set?"));
        } else {
            match unsafe {
                XNVCTRLQueryTargetCount(dpy, CTRL_TARGET::GPU, &mut gpu_count)
            } {
                XNV_OK => {}
                i => { return Err(format!("XNVCtrl QueryCount(GPU) failed; error {}", i)); }
            }

            gpus = Vec::with_capacity(gpu_count as usize);

            for i in 0..gpu_count {
                let mut len = -1 as i32;
                let v: *mut c_uchar = unsafe { mem::uninitialized() };

                match unsafe {
                    XNVCTRLQueryTargetBinaryData(dpy, CTRL_TARGET::GPU, i, 0,
                                                 BIN_ATTR::COOLERS_USED_BY_GPU, &v , &mut len)
                } {
                    XNV_OK => {
                        // NVCtrl stores the number of coolers in the first int of the response
                        // array rather than the `len` variable; I know, it's unintuitive. So we
                        // need to actually read the first int from the `raw` array to find out how
                        // many coolers the GPU has. The `raw` array always has a length of
                        // NUM_OF_GPU_COOLERS + 1 and it is populated with the indices of said
                        // coolers. Black pointer magic.
                        let raw = unsafe { mem::transmute::<*mut c_uchar, *mut c_int>(v) };
                        let num_coolers = unsafe { ptr::read(raw) } as usize;
                        let mut coolers: Vec<u32> = Vec::with_capacity(num_coolers);
                        let array: &[c_int] = unsafe {
                            slice::from_raw_parts(raw, 1usize+num_coolers)
                        };

                        for cooler in 0..(num_coolers) {
                            coolers.push(array[cooler+1] as u32);
                        }

                        gpus.push(UnixGPU { id: i as u32, coolers: coolers });

                    }
                    i => {
                        return Err(format!("XNVCtrl BinaryData(COOLERS_USED_BY_GPU failed; {}", i));
                    }
                }

            }
        }

        Ok(NvidiaControl{ limits: lim,
                          dpy: dpy,
                          _gpus: gpus })
    }
}

impl Drop for NvidiaControl {
    fn drop(&mut self) {
        unsafe { XCloseDisplay(self.dpy) };
    }
}

impl NvidiaControl {

    /// Check if the supplied GPU id corresponds to a physical GPU. This
    /// function will return an `Err` if the specified id is outside the
    /// defined boundaries or `()` otherwise.
    ///
    /// **Arguments**
    ///
    /// * `gpu` - The GPU id to check
    fn check_gpu_id(&self, gpu: u32) -> Result<(), String> {
        if gpu as usize > (self._gpus.len() - 1) {
            Err(format!("check_gpu_id() failed; id {} > {}",
                        gpu, self._gpus.len() - 1))
        } else {
            Ok(())
        }
    }

    fn check_fan_id(&self, id: u32) -> Result<(), String> {

        for gpu in &self._gpus {
            match gpu.coolers.iter().find(|x| x == &&id ) {
                Some(_) => { return Ok(()); },
                None => {}
            }
        }

        Err(format!("check_fan_id() failed; Cooler {} not found", id))
    }

}

impl NvFanController for NvidiaControl {

    fn get_temp(&self, id: u32) -> Result<i32, String> {

        self.check_gpu_id(id)?;

        let mut tmp = -1 as i32;
        match unsafe {
            XNVCTRLQueryTargetAttribute(self.dpy, CTRL_TARGET::GPU, id as i32, 0,
                                        CTRL_ATTR::CORE_TEMPERATURE, &mut tmp)
        } {
            XNV_OK => Ok(tmp),
            i => Err(format!("XNVCtrl QueryAttr(CORE_TEMPERATURE) failed; error {}", i))
        }
    }

    fn gpu_count(&self) -> Result<u32, String> {
        Ok(self._gpus.len() as u32)
    }

    fn gpu_coolers(&self, gpu: u32) -> Result<&Vec<u32>, String> {

        self.check_gpu_id(gpu)?;

        Ok(&self._gpus[gpu as usize].coolers)

    }

    fn get_ctrl_status(&self, gpu: u32) -> Result<NVCtrlFanControlState, String> {

        self.check_gpu_id(gpu)?;

        let mut tmp = -1 as i32;
        match unsafe {
            XNVCTRLQueryTargetAttribute(self.dpy, CTRL_TARGET::GPU, gpu as i32, 0,
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

    fn set_ctrl_type(&self, gpu: u32, typ: NVCtrlFanControlState) -> Result<(), String> {

        self.check_gpu_id(gpu)?;

        match unsafe {
            XNVCTRLSetTargetAttributeAndGetStatus(self.dpy, CTRL_TARGET::GPU, gpu as i32, 0,
                                                  CTRL_ATTR::COOLER_MANUAL_CONTROL,
                                                  typ as c_int)
        } {
            XNV_OK => Ok(()),
            i => Err(format!("XNVCtrl SetAttr(COOLER_MANUAL_CONTROL) failed; error {}", i))
        }
    }

    fn get_fanspeed(&self, _: u32, id: u32) -> Result<i32, String> {

        self.check_fan_id(id)?;

        let mut tmp = -1 as i32;
        match unsafe {
            XNVCTRLQueryTargetAttribute(self.dpy, CTRL_TARGET::COOLER, id as i32, 0,
                                        CTRL_ATTR::THERMAL_COOLER_CURRENT_LEVEL, &mut tmp)} {
            XNV_OK => Ok(tmp),
            i => Err(format!("XNVCtrl QueryAttr(COOLER_CURRENT_LEVEL) failed; error {}", i))
        }
    }

    fn get_fanspeed_rpm(&self, _: u32, id: u32) -> Result<i32, String> {

        self.check_fan_id(id)?;

        let mut tmp = -1 as i32;
        match unsafe {
            XNVCTRLQueryTargetAttribute(self.dpy, CTRL_TARGET::COOLER, id as i32, 0,
                                        CTRL_ATTR::THERMAL_COOLER_SPEED, &mut tmp)} {
            XNV_OK => Ok(tmp),
            i => Err(format!("XNVCtrl QueryAttr(COOLER_SPEED) failed; error {}", i))
        }
    }

    fn set_fanspeed(&self, _: u32, id: u32, speed: i32) -> Result<(), String> {

        self.check_fan_id(id)?;

        let true_speed = self.true_speed(speed);
        match unsafe {
            XNVCTRLSetTargetAttributeAndGetStatus(self.dpy, CTRL_TARGET::COOLER, id as i32,
                                                  0, CTRL_ATTR::THERMAL_COOLER_LEVEL,
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

    fn get_adapter(&self, id: u32) -> Result<String, String> {

        self.check_gpu_id(id)?;

        let v: *mut c_char = unsafe { mem::uninitialized() };
        match unsafe {
            XNVCTRLQueryTargetStringAttribute(self.dpy, CTRL_TARGET::GPU, id as i32,
                                              0, CTRL_ATTR::PRODUCT_NAME, &v)
        } {
            XNV_OK => {
                assert!(!v.is_null());
                Ok(unsafe { CStr::from_ptr(v as *const c_char).to_str().unwrap().to_owned() })
            },
            i => Err(format!("XNVCtrl QueryAttr(PRODUCT_NAME) failed; error {}", i))
        }
    }

    fn get_utilization(&self, id: u32) -> Result<HashMap<&str, i32>, String> {

        self.check_gpu_id(id)?;

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
