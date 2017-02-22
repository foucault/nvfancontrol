#![allow(non_upper_case_globals)]

use libloading::{Library, Symbol};

use std::ffi::CStr;
use std::mem;
use std::collections::HashMap;
use std::env;
use libc;
use ::{NVCtrlFanControlState, NvFanController};

const NVAPI_SHORT_STRING_MAX: usize = 64;
const NVAPI_MAX_PHYSICAL_GPUS: usize = 64;
const NVAPI_MAX_THERMAL_SENSORS_PER_GPU: usize = 3;
const NVAPI_MAX_COOLERS_PER_GPU: usize = 20;
const NVAPI_MAX_USAGES_PER_GPU: usize = 33;

#[cfg(target_arch="x86")] type QueryPtr = u32;
#[cfg(target_arch="x86")] const NVAPI_DLL: &'static str = "nvapi.dll";

#[cfg(target_arch="x86_64")] type QueryPtr = u64;
#[cfg(target_arch="x86_64")] const NVAPI_DLL: &'static str = "nvapi64.dll";

#[allow(dead_code)]
#[repr(u32)]
enum QueryCode {
    Initialize = 0x150E828,
    Unload = 0x0D22BDD7E,
    SetCoolerLevels = 0x891FA0AE,
    GetCoolerSettings = 0xDA141340,
    GetUsages = 0x189A1FDF
}

#[allow(non_snake_case)]
fn NVAPI_VERSION<T>(v: u32) -> u32 {
    let size: u32 = mem::size_of::<T>() as u32;
    (size | v<<16) as u32
}

lazy_static! {
    static ref NVAPI: Library = {
        let system_root = env::var("SystemRoot").unwrap_or(String::from("C:\\Windows"));
        let nvapi_path = format!("{}\\System32\\{}", system_root, NVAPI_DLL);
        Library::new(nvapi_path).unwrap()
    };
    static ref NvAPI_QueryInterface: Symbol<'static, unsafe extern "C" fn(QueryPtr) -> *const ()> =
        unsafe { NVAPI.get(b"nvapi_QueryInterface").unwrap() };
}

/*
 * Duplication here is necessary until #37406 [0] is resolved and
 * feature(link_cfg) makes it into stable
 * [0]: https://github.com/rust-lang/rust/issues/37406
 */
#[allow(dead_code)]
#[cfg(target_arch="x86_64")]
#[link(name="nvapi64", kind="static")]
extern {
    fn NvAPI_Initialize() -> libc::c_int;
    fn NvAPI_Unload() -> libc::c_int;
    fn NvAPI_GetInterfaceVersionString(ver: *mut NvAPI_ShortString) -> libc::c_int;
    fn NvAPI_EnumPhysicalGPUs(handles: *mut [NvPhysicalGpuHandle; NVAPI_MAX_PHYSICAL_GPUS], count: *mut u32) -> libc::c_int;
    fn NvAPI_GPU_GetFullName(handle: NvPhysicalGpuHandle, name: *mut NvAPI_ShortString) -> libc::c_int;
    fn NvAPI_GPU_GetTachReading(handles: NvPhysicalGpuHandle, value: *mut u32) -> libc::c_int;
    fn NvAPI_GPU_GetThermalSettings(handle: NvPhysicalGpuHandle, index: u32, settings: *mut NV_GPU_THERMAL_SETTINGS_V2) -> libc::c_int;
    fn NvAPI_SYS_GetDriverAndBranchVersion(driverVersion: *mut u32, branch: *mut NvAPI_ShortString) -> libc::c_int;
}

#[allow(dead_code)]
#[cfg(target_arch="x86")]
#[link(name="nvapi", kind="static")]
extern {
    fn NvAPI_Initialize() -> libc::c_int;
    fn NvAPI_Unload() -> libc::c_int;
    fn NvAPI_GetInterfaceVersionString(ver: *mut NvAPI_ShortString) -> libc::c_int;
    fn NvAPI_EnumPhysicalGPUs(handles: *mut [NvPhysicalGpuHandle; NVAPI_MAX_PHYSICAL_GPUS], count: *mut u32) -> libc::c_int;
    fn NvAPI_GPU_GetFullName(handle: NvPhysicalGpuHandle, name: *mut NvAPI_ShortString) -> libc::c_int;
    fn NvAPI_GPU_GetTachReading(handles: NvPhysicalGpuHandle, value: *mut u32) -> libc::c_int;
    fn NvAPI_GPU_GetThermalSettings(handle: NvPhysicalGpuHandle, index: u32, settings: *mut NV_GPU_THERMAL_SETTINGS_V2) -> libc::c_int;
    fn NvAPI_SYS_GetDriverAndBranchVersion(driverVersion: *mut u32, branch: *mut NvAPI_ShortString) -> libc::c_int;
}

#[allow(non_snake_case)]
unsafe fn NvAPI_GPU_SetCoolerLevels(handle: NvPhysicalGpuHandle, index: u32, levels: *const NvGpuCoolerLevels) -> libc::c_int {
    let func = mem::transmute::<
        *const (), fn(NvPhysicalGpuHandle, u32, *const NvGpuCoolerLevels) -> libc::c_int
    >(NvAPI_QueryInterface(QueryCode::SetCoolerLevels as QueryPtr));
    func(handle, index, levels)
}

#[allow(non_snake_case)]
unsafe fn NvAPI_GPU_GetCoolerSettings(handle: NvPhysicalGpuHandle, index: u32, settings: *mut NvGpuCoolerSettings) -> libc::c_int {
    let func = mem::transmute::<
        *const (), fn(NvPhysicalGpuHandle, u32, *mut NvGpuCoolerSettings) -> libc::c_int
    >(NvAPI_QueryInterface(QueryCode::GetCoolerSettings as QueryPtr));
    func(handle, index, settings)
}

#[allow(non_snake_case)]
unsafe fn NvAPI_GPU_GetUsages(handle: NvPhysicalGpuHandle, usages: *mut NvGpuUsages) -> libc::c_int {
    let func = mem::transmute::<
        *const (), fn(NvPhysicalGpuHandle, *mut NvGpuUsages) -> libc::c_int
    >(NvAPI_QueryInterface(QueryCode::GetUsages as QueryPtr));
    func(handle, usages)
}

#[repr(C)]
struct NvAPI_ShortString {
    inner: [libc::c_char; NVAPI_SHORT_STRING_MAX]
}

impl NvAPI_ShortString {
    fn new() -> NvAPI_ShortString {
        NvAPI_ShortString { inner: [0 as libc::c_char; NVAPI_SHORT_STRING_MAX] }
    }

    fn to_string(&self) -> String {
        unsafe { CStr::from_ptr(self.inner.as_ptr()).to_str().unwrap().to_owned() }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
struct NvPhysicalGpuHandle { unused: i32 }

impl NvPhysicalGpuHandle {
    fn new() -> NvPhysicalGpuHandle {
        NvPhysicalGpuHandle { unused: 0 }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(dead_code)]
enum NV_THERMAL_CONTROLLER {
    NONE = 0,
    GPU_INTERNAL = 1,
    ADM1032,
    MAX6649,
    MAX1617,
    LM99,
    LM89,
    LM64,
    ADT7473,
    SBMAX6649,
    VBIOSEVT,
    OS,
    UNKNOWN = -1
}

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(dead_code)]
enum NV_THERMAL_TARGET {
    NONE          = 0,
    GPU           = 1,
    MEMORY        = 2,
    POWER_SUPPLY  = 4,
    BOARD         = 8,
    VCD_BOARD     = 9,
    VCD_INLET     = 10,
    VCD_OUTLET    = 11,
    ALL           = 15,
    UNKNOWN       = -1,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct NvThermalSensor {
    controller: NV_THERMAL_CONTROLLER,
    default_min_temp: i32,
    default_max_temp: i32,
    current_temp: i32,
    target: NV_THERMAL_TARGET
}

impl NvThermalSensor {
    fn new() -> NvThermalSensor {
        NvThermalSensor {
            controller: NV_THERMAL_CONTROLLER::UNKNOWN,
            default_min_temp: -1,
            default_max_temp: -1,
            current_temp: -1,
            target: NV_THERMAL_TARGET::UNKNOWN
        }
    }
}

#[repr(C)]
#[allow(non_snake_case)]
struct NV_GPU_THERMAL_SETTINGS_V2 {
    version: u32,
    count: u32,
    sensors: [NvThermalSensor; NVAPI_MAX_THERMAL_SENSORS_PER_GPU]
}

impl NV_GPU_THERMAL_SETTINGS_V2 {
    fn new() -> NV_GPU_THERMAL_SETTINGS_V2 {
        NV_GPU_THERMAL_SETTINGS_V2 {
            version: NVAPI_VERSION::<NV_GPU_THERMAL_SETTINGS_V2>(2u32),
            count: 0,
            sensors: [NvThermalSensor::new(); NVAPI_MAX_THERMAL_SENSORS_PER_GPU]
        }
    }

    fn temp(&self, index: u32) -> i32 {
        self.sensors[index as usize].current_temp
    }

    /*fn target(&self, index: u32) -> NV_THERMAL_TARGET {
        self.sensors[index as usize].target
    }*/
}

#[repr(C)]
#[derive(Clone, Copy)]
#[allow(non_snake_case)]
struct NvLevel {
    level: i32,
    policy: i32
}

#[repr(C)]
#[allow(non_snake_case)]
struct NvGpuCoolerLevels {
    version: u32,
    coolers: [NvLevel; NVAPI_MAX_COOLERS_PER_GPU]
}

impl NvGpuCoolerLevels {
    fn new() -> NvGpuCoolerLevels {
        NvGpuCoolerLevels {
            version: NVAPI_VERSION::<NvGpuCoolerLevels>(1u32),
            coolers: [NvLevel { level: -1, policy: -1 }; NVAPI_MAX_COOLERS_PER_GPU]
        }
    }

    fn set_level(&mut self, index: u32, level: i32) {
        self.coolers[index as usize].level = level;
    }

    fn set_policy(&mut self, index: u32, policy: i32) {
        self.coolers[index as usize].policy = policy;
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
#[allow(non_snake_case)]
struct NvCooler {
    cooler_type: i32,
    controller: i32,
    default_min: i32,
    default_max: i32,
    current_min: i32,
    current_max: i32,
    current_level: i32,
    default_policy: i32,
    current_policy: i32,
    target: i32,
    control_type: i32,
    active: i32,
}

#[repr(C)]
struct NvGpuCoolerSettings {
    version: u32,
    count: u32,
    coolers: [NvCooler; NVAPI_MAX_COOLERS_PER_GPU]
}

impl NvGpuCoolerSettings {
    fn new() -> NvGpuCoolerSettings {
        NvGpuCoolerSettings {
            version: NVAPI_VERSION::<NvGpuCoolerSettings>(2u32),
            count: 0,
            coolers: [NvCooler {
                cooler_type: -1,
                controller: -1,
                default_min: -1,
                default_max: -1,
                current_min: -1,
                current_max: -1,
                current_level: -1,
                default_policy: -1,
                current_policy: -1,
                target: -1,
                control_type: -1,
                active: -1
            }; NVAPI_MAX_COOLERS_PER_GPU]
        }
    }
}

#[repr(C)]
#[allow(non_snake_case)]
struct NvGpuUsages {
    version: u32,
    usage: [u32; NVAPI_MAX_USAGES_PER_GPU]
}

impl NvGpuUsages {
    fn new() -> NvGpuUsages {
        NvGpuUsages {
            version: NVAPI_VERSION::<NvGpuUsages>(1),
            usage: [0u32; NVAPI_MAX_USAGES_PER_GPU]
        }
    }
}

fn mode_to_policy(state: NVCtrlFanControlState) -> u32 {
    match state {
        NVCtrlFanControlState::Auto => 0x20,
        NVCtrlFanControlState::Manual => 0x1,
    }
}

pub struct NvidiaControl {
    pub limits: (u16, u16)
}

impl NvidiaControl {
    pub fn init(lim: (u16, u16)) -> Result<NvidiaControl, String> {
        match unsafe { NvAPI_Initialize() } {
            0 => Ok(
                NvidiaControl {
                    limits: lim
                }
            ),
            i => Err(format!("NvAPI_Initialize() failed; error: {}", i))
        }
    }

}

impl Drop for NvidiaControl {
    fn drop(&mut self) {
        unsafe { NvAPI_Unload() };
    }
}

impl NvFanController for NvidiaControl {

    fn get_temp(&self) -> Result<i32, String> {
        let mut handle = [NvPhysicalGpuHandle::new(); NVAPI_MAX_PHYSICAL_GPUS];
        let mut count = 0 as u32;

        match unsafe { NvAPI_EnumPhysicalGPUs(&mut handle, &mut count) } {
            0 => {
                let mut thermal = NV_GPU_THERMAL_SETTINGS_V2::new();
                match unsafe { NvAPI_GPU_GetThermalSettings(handle[0], 0, &mut thermal) } {
                    0 => Ok(thermal.temp(0)),
                    i => Err(format!("NvAPI_GPU_GetThermalSettings() failed; error {}", i))
                }
            },
            i => Err(format!("NvAPI_EnumPhysicalGPUs() failed; error {}", i))
        }
    }


    fn get_ctrl_status(&self) -> Result<NVCtrlFanControlState, String> {
        let mut handle = [NvPhysicalGpuHandle::new(); NVAPI_MAX_PHYSICAL_GPUS];
        let mut count = 0 as u32;

        match unsafe { NvAPI_EnumPhysicalGPUs(&mut handle, &mut count) } {
            0 => {
                let mut cooler_settings = NvGpuCoolerSettings::new();
                match unsafe { NvAPI_GPU_GetCoolerSettings(handle[0], 0, &mut cooler_settings) } {
                    0 => {
                        match cooler_settings.coolers[0].current_policy {
                            0x20 | 0x10 => { Ok(NVCtrlFanControlState::Auto) },
                            0x01 => { Ok(NVCtrlFanControlState::Manual) },
                            i => { Err(format!("NvAPI_GPU_GetCoolerSettings() unknown policy: {}", i)) }
                        }

                    },
                    i => Err(format!("NvAPI_GPU_GetCoolerSettings() failed; error {}", i))
                }
            },
            i => Err(format!("NvAPI_EnumPhysicalGPUs() failed; error {}", i))
        }
    }

    fn set_ctrl_type(&self, typ: NVCtrlFanControlState) -> Result<(), String> {

        // Retain existing fanspeed
        let fanspeed = try!(self.get_fanspeed());
        let policy = mode_to_policy(typ);

        let mut handle = [NvPhysicalGpuHandle::new(); NVAPI_MAX_PHYSICAL_GPUS];
        let mut count = 0 as u32;

        match unsafe { NvAPI_EnumPhysicalGPUs(&mut handle, &mut count) } {
            0 => {
                let mut levels = NvGpuCoolerLevels::new();
                levels.set_policy(0, policy as i32);
                levels.set_level(0, fanspeed);
                match unsafe { NvAPI_GPU_SetCoolerLevels(handle[0], 0, &levels) } {
                    0 => { Ok(()) },
                    i => { Err(format!("NvAPI_GPU_SetCoolerLevels() failed; error {}", i)) }
                }
            },
            i => Err(format!("NvAPI_EnumPhysicalGPUs() failed; error {}", i))
        }
    }

    fn get_fanspeed(&self) -> Result<i32, String> {
        let mut handle = [NvPhysicalGpuHandle::new(); NVAPI_MAX_PHYSICAL_GPUS];
        let mut count = 0 as u32;

        match unsafe { NvAPI_EnumPhysicalGPUs(&mut handle, &mut count) } {
            0 => {
                let mut cooler_settings = NvGpuCoolerSettings::new();
                match unsafe { NvAPI_GPU_GetCoolerSettings(handle[0], 0, &mut cooler_settings) } {
                    0 => Ok(cooler_settings.coolers[0].current_level),
                    i => Err(format!("NvAPI_GPU_GetCoolerSettings() failed; error {}", i))
                }
            },
            i => Err(format!("NvAPI_EnumPhysicalGPUs() failed; error {}", i))
        }
    }

    fn get_fanspeed_rpm(&self) -> Result<i32, String> {
        let mut handle = [NvPhysicalGpuHandle::new(); NVAPI_MAX_PHYSICAL_GPUS];
        let mut count = 0 as u32;

        match unsafe { NvAPI_EnumPhysicalGPUs(&mut handle, &mut count) } {
            0 => {
                let mut speed = 0 as libc::c_uint;
                match unsafe { NvAPI_GPU_GetTachReading(handle[0], &mut speed) } {
                    0 => Ok(speed as i32),
                    i => Err(format!("NvAPI_GPU_GetTachReading() failed; error {}", i))
                }
            },
            i => Err(format!("NvAPI_EnumPhysicalGPUs() failed; error {}", i))
        }
    }

    fn set_fanspeed(&self, speed: i32) -> Result<(), String> {
        let true_speed = self.true_speed(speed);

        // Retain the existing policy
        let policy = match self.get_ctrl_status() {
            Ok(mode) => mode_to_policy(mode),
            Err(e) => { return Err(e); }
        } as i32;

        let mut handle = [NvPhysicalGpuHandle::new(); NVAPI_MAX_PHYSICAL_GPUS];
        let mut count = 0 as u32;

        match unsafe { NvAPI_EnumPhysicalGPUs(&mut handle, &mut count) } {
            0 => {
                let mut levels = NvGpuCoolerLevels::new();
                levels.set_policy(0, policy);
                levels.set_level(0, true_speed as i32);
                match unsafe { NvAPI_GPU_SetCoolerLevels(handle[0], 0, &levels) } {
                    0 => {},
                    i => { return Err(format!("NvAPI_GPU_SetCoolerLevels() failed; error {}", i)); }
                }
            },
            i => { return Err(format!("NvAPI_EnumPhysicalGPUs() failed; error {}", i)); }
        }

        Ok(())
    }

    fn get_version(&self) -> Result<String, String> {
        let mut b = NvAPI_ShortString::new();
        let mut v: libc::c_uint = 0;

        match unsafe { NvAPI_SYS_GetDriverAndBranchVersion(&mut v, &mut b) } {
            0 => Ok(format!("{:.2}", (v as f32)/100.0)),
            i => Err(format!("NvAPI_SYS_GetDriverAndBranchVersion() failed; error {:?}", i))
        }
    }

    fn get_adapter(&self) -> Result<String, String> {
        let mut handle = [NvPhysicalGpuHandle::new(); NVAPI_MAX_PHYSICAL_GPUS];
        let mut count = 0 as u32;

        match unsafe { NvAPI_EnumPhysicalGPUs(&mut handle, &mut count) } {
            0 => {
                let mut adapter = NvAPI_ShortString::new();
                match unsafe { NvAPI_GPU_GetFullName(handle[0], &mut adapter) } {
                    0 => Ok(adapter.to_string()),
                    i => Err(format!("NvAPI_GPU_GetFullName() failed; error {:?}", i))
                }
            },
            i => Err(format!("NvAPI_EnumPhysicalGPUs() failed; error {}", i))
        }
    }

    fn get_utilization(&self) -> Result<HashMap<&str, i32>, String> {
        let mut handle = [NvPhysicalGpuHandle::new(); NVAPI_MAX_PHYSICAL_GPUS];
        let mut count = 0 as u32;

        match unsafe { NvAPI_EnumPhysicalGPUs(&mut handle, &mut count) } {
            0 => {
                let mut gpu_usages = NvGpuUsages::new();
                match unsafe { NvAPI_GPU_GetUsages(handle[0], &mut gpu_usages) } {
                    0 => {
                        let mut ret: HashMap<&str, i32> = HashMap::with_capacity(3);
                        ret.insert("graphics", gpu_usages.usage[2] as i32);
                        ret.insert("memory", gpu_usages.usage[6] as i32);
                        ret.insert("video", gpu_usages.usage[10] as i32);
                        Ok(ret)
                    },
                    i => Err(format!("NvAPI_GPU_GetUsages() failed; error {}", i))
                }
            },
            i => Err(format!("NvAPI_EnumPhysicalGPUs() failed; error {}", i))
        }
    }
}
