#![allow(non_upper_case_globals)]

use libloading::{Library, Symbol};

use std::ffi::CStr;
use std::mem;
use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use libc;
use ::{NVCtrlFanControlState, NvFanController};

const NVAPI_SHORT_STRING_MAX: usize = 64;
const NVAPI_MAX_PHYSICAL_GPUS: usize = 64;
const NVAPI_MAX_THERMAL_SENSORS_PER_GPU: usize = 3;
const NVAPI_MAX_COOLERS_PER_GPU: usize = 3;
const NVAPI_MAX_USAGES_PER_GPU: usize = 33;
const NVAPI_COOLER_TARGET_ALL: usize = 7;

#[cfg(target_arch="x86")] type QueryPtr = u32;
#[cfg(target_arch="x86")] const NVAPI_DLL: &'static str = "nvapi.dll";

#[cfg(target_arch="x86_64")] type QueryPtr = u64;
#[cfg(target_arch="x86_64")] const NVAPI_DLL: &'static str = "nvapi64.dll";

/// Query codes for NvAPI functions hidden from the public API. This is used
/// in conjuction with `NvAPI_QueryInterface`. There are implementations for
/// all the variants defined in `QueryCode`.
#[allow(dead_code)]
#[repr(u32)]
enum QueryCode {
    Initialize = 0x150E828,
    Unload = 0x0D22BDD7E,
    SetCoolerLevels = 0x891FA0AE,
    GetCoolerSettings = 0xDA141340,
    GetUsages = 0x189A1FDF
}

/// Generates a NvAPI compatible version for a specified struct type
///
/// **Arguments**
///
/// * `v` - Version number
/// * `T` - The type of the struct that this version is generated for
#[allow(non_snake_case)]
fn NVAPI_VERSION<T>(v: u32) -> u32 {
    let size: u32 = mem::size_of::<T>() as u32;
    (size | v<<16) as u32
}

lazy_static! {
    /// Dynamic load of nvapi{64}.dll
    static ref NVAPI: Library = {
        let system_root = env::var("SystemRoot").unwrap_or(String::from("C:\\Windows"));
        let nvapi_path = format!("{}\\System32\\{}", system_root, NVAPI_DLL);
        Library::new(nvapi_path).unwrap()
    };
    /// Registration of `NvAPI_QueryInterface` function which provides pointer to all the
    /// function of NvAPI.
    static ref NvAPI_QueryInterface: Symbol<'static, unsafe extern "C" fn(QueryPtr) -> *const ()> =
        unsafe { NVAPI.get(b"nvapi_QueryInterface").unwrap() };
}

/*
 * Duplication here is necessary until #37406 [0] is resolved and
 * feature(link_cfg) makes it into stable
 * [0]: https://github.com/rust-lang/rust/issues/37406
 */
/// All these functions return a status code upon call. There are wrappers for all these function
/// through the `NvFanController` trait and are part of the documented NVAPI spec.
#[allow(dead_code)]
#[cfg(target_arch="x86_64")]
#[link(name="nvapi64", kind="static")]
extern {
    /// Initialises NvAPI.
    fn NvAPI_Initialize() -> libc::c_int;

    /// Unloads nvapi{64}.dll from memory.
    fn NvAPI_Unload() -> libc::c_int;

    /// Returns the version of the NVAPI library
    ///
    /// ***Arguments***
    ///
    /// * `ver` - An `NvAPI_ShortString` that will be populated upon function call with the NVAPI
    /// version string
    fn NvAPI_GetInterfaceVersionString(ver: *mut NvAPI_ShortString) -> libc::c_int;

    /// Lists all available physical GPUS into the specified array.
    ///
    /// **Arguments**
    ///
    /// * `handles` - An array of unitialized `NvPhysicalGpuHandle`s. The size of this function is
    /// specified by the static variable `NVAPI_MAX_PHYSICAL_GPUS`. The array will be populated
    /// upon function call.
    ///
    /// * `count` - The number of available physical GPUs. These variable will be populated upon
    /// function call.
    fn NvAPI_EnumPhysicalGPUs(handles: *mut [NvPhysicalGpuHandle; NVAPI_MAX_PHYSICAL_GPUS], count: *mut u32) -> libc::c_int;

    /// Get the name of the specified GPU
    ///
    /// **Arguments**
    ///
    /// * `handle` - The GPU for which the name is requested
    /// * `name` - A pointer to an `NvAPI_ShortString` that will be populated with the adapter name
    /// upon function call.
    fn NvAPI_GPU_GetFullName(handle: NvPhysicalGpuHandle, name: *mut NvAPI_ShortString) -> libc::c_int;

    /// Returns the fan speed in RPM
    ///
    /// **Arguments**
    ///
    /// * `handle` - The GPU for which the fan speed is requested
    /// * `value` - The fan speed in RPM; it will be populated upon function call
    fn NvAPI_GPU_GetTachReading(handle: NvPhysicalGpuHandle, value: *mut u32) -> libc::c_int;

    /// Returns the thermal status of the specified GPU
    ///
    /// **Arguments**
    ///
    /// * `handle` - The GPU for which the thermal settings are requested.
    /// * `index` - The sensor index
    /// * `settings` - The thermal settings struct; it will be populated upon function call
    fn NvAPI_GPU_GetThermalSettings(handle: NvPhysicalGpuHandle, index: u32, settings: *mut NV_GPU_THERMAL_SETTINGS_V2) -> libc::c_int;

    /// Returns the NVidia driver version
    ///
    /// **Arguments**
    ///
    /// * `driverVersion` - The driver version number; it will be populated upon function call
    /// * `branch` - The driver version branch; it will be populated upon function call
    fn NvAPI_SYS_GetDriverAndBranchVersion(driverVersion: *mut u32, branch: *mut NvAPI_ShortString) -> libc::c_int;
}

/// All these functions return a status code upon call. There are wrappers for all these function
/// through the `NvFanController` trait and are part of the documented NVAPI spec.
#[allow(dead_code)]
#[cfg(target_arch="x86")]
#[link(name="nvapi", kind="static")]
extern {
    /// Initialises NvAPI.
    fn NvAPI_Initialize() -> libc::c_int;

    /// Unloads nvapi{64}.dll from memory.
    fn NvAPI_Unload() -> libc::c_int;

    /// Returns the version of the NVAPI library
    ///
    /// ***Arguments***
    ///
    /// * `ver` - An `NvAPI_ShortString` that will be populated upon function call with the NVAPI
    /// version string
    fn NvAPI_GetInterfaceVersionString(ver: *mut NvAPI_ShortString) -> libc::c_int;

    /// Lists all available physical GPUS into the specified array.
    ///
    /// **Arguments**
    ///
    /// * `handles` - An array of unitialized `NvPhysicalGpuHandle`s. The size of this function is
    /// specified by the static variable `NVAPI_MAX_PHYSICAL_GPUS`. The array will be populated
    /// upon function call.
    ///
    /// * `count` - The number of available physical GPUs. These variable will be populated upon
    /// function call.
    fn NvAPI_EnumPhysicalGPUs(handles: *mut [NvPhysicalGpuHandle; NVAPI_MAX_PHYSICAL_GPUS], count: *mut u32) -> libc::c_int;

    /// Get the name of the specified GPU
    ///
    /// **Arguments**
    ///
    /// * `handle` - The GPU for which the name is requested
    /// * `name` - A pointer to an `NvAPI_ShortString` that will be populated with the adapter name
    /// upon function call.
    fn NvAPI_GPU_GetFullName(handle: NvPhysicalGpuHandle, name: *mut NvAPI_ShortString) -> libc::c_int;

    /// Returns the fan speed in RPM
    ///
    /// **Arguments**
    ///
    /// * `handle` - The GPU for which the fan speed is requested
    /// * `value` - The fan speed in RPM; it will be populated upon function call
    fn NvAPI_GPU_GetTachReading(handle: NvPhysicalGpuHandle, value: *mut u32) -> libc::c_int;

    /// Returns the thermal status of the specified GPU
    ///
    /// **Arguments**
    ///
    /// * `handle` - The GPU for which the thermal settings are requested.
    /// * `index` - The sensor index
    /// * `settings` - The thermal settings struct; it will be populated upon function call
    fn NvAPI_GPU_GetThermalSettings(handle: NvPhysicalGpuHandle, index: u32, settings: *mut NV_GPU_THERMAL_SETTINGS_V2) -> libc::c_int;

    /// Returns the NVidia driver version
    ///
    /// **Arguments**
    ///
    /// * `driverVersion` - The driver version number; it will be populated upon function call
    /// * `branch` - The driver version branch; it will be populated upon function call
    fn NvAPI_SYS_GetDriverAndBranchVersion(driverVersion: *mut u32, branch: *mut NvAPI_ShortString) -> libc::c_int;
}

/// Sets the cooler level for the specified GPU. This is an undocumented function.
///
/// **Arguments**
///
/// * `handle` - The GPU for which the cooler levels will be set
/// * `index` - The cooler index
/// * `levels` - The cooler levels for the specified GPU
#[allow(non_snake_case)]
unsafe fn NvAPI_GPU_SetCoolerLevels(handle: NvPhysicalGpuHandle, index: u32, levels: *const NvGpuCoolerLevels) -> libc::c_int {
    let func = mem::transmute::<
        *const (), fn(NvPhysicalGpuHandle, u32, *const NvGpuCoolerLevels) -> libc::c_int
    >(NvAPI_QueryInterface(QueryCode::SetCoolerLevels as QueryPtr));
    func(handle, index, levels)
}

/// Returns the active cooler settings for the specified GPU and cooler. This is an undocumented
/// function.
///
/// **Arguments**
///
/// * `handle` - The GPU for which the cooler settings are requested
/// * `index` - The cooler index
/// * `settings` - The `NvGpuCoolerSettings` containing the requested information; it will be
/// populated upon function call
#[allow(non_snake_case)]
unsafe fn NvAPI_GPU_GetCoolerSettings(handle: NvPhysicalGpuHandle, index: u32, settings: *mut NvGpuCoolerSettings) -> libc::c_int {
    let func = mem::transmute::<
        *const (), fn(NvPhysicalGpuHandle, u32, *mut NvGpuCoolerSettings) -> libc::c_int
    >(NvAPI_QueryInterface(QueryCode::GetCoolerSettings as QueryPtr));
    func(handle, index, settings)
}

/// Returns the GPU utilisation. This is an undocumented function.
///
/// **Arguments**
///
/// * `handle` - The GPU for which the utilisation is requested
/// * `usages` - The `NvGpuUsages` containing the requested information; it will be populated upon
/// function call
#[allow(non_snake_case)]
unsafe fn NvAPI_GPU_GetUsages(handle: NvPhysicalGpuHandle, usages: *mut NvGpuUsages) -> libc::c_int {
    let func = mem::transmute::<
        *const (), fn(NvPhysicalGpuHandle, *mut NvGpuUsages) -> libc::c_int
    >(NvAPI_QueryInterface(QueryCode::GetUsages as QueryPtr));
    func(handle, usages)
}

/// A representation of the NvAPI_ShortString. It is an array of `c_char` with a predefined length.
#[repr(C)]
struct NvAPI_ShortString {
    inner: [libc::c_char; NVAPI_SHORT_STRING_MAX]
}

impl NvAPI_ShortString {
    /// Create an empty `NvAPI_ShortString` consisting entirely of \0
    fn new() -> NvAPI_ShortString {
        NvAPI_ShortString { inner: [0 as libc::c_char; NVAPI_SHORT_STRING_MAX] }
    }

    /// Returns a `String` representation of this `NvAPI_ShortString`. This copied data
    /// in order to be useful.
    fn to_string(&self) -> String {
        unsafe { CStr::from_ptr(self.inner.as_ptr()).to_str().unwrap().to_owned() }
    }
}

/// A GPU handle.
#[repr(C)]
#[derive(Copy, Clone)]
struct NvPhysicalGpuHandle { unused: i32 }

impl NvPhysicalGpuHandle {
    /// Returns a new empty GPU handle.
    fn new() -> NvPhysicalGpuHandle {
        NvPhysicalGpuHandle { unused: 0 }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(dead_code, non_camel_case_types)]
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
#[allow(dead_code, non_camel_case_types)]
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

/// A representation of a thermal sensor
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
    /// Instantiates a new empty thermal sensors. The field are further populated when a function
    /// call occurs.
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

/// A thermal settings struct
#[repr(C)]
#[allow(non_snake_case)]
struct NV_GPU_THERMAL_SETTINGS_V2 {
    /// Struct version; created with `NVAPI_VERSION<T>()`
    version: u32,
    /// Number of available sensors
    count: u32,
    /// A list of all the available sensors
    sensors: [NvThermalSensor; NVAPI_MAX_THERMAL_SENSORS_PER_GPU]
}

impl NV_GPU_THERMAL_SETTINGS_V2 {
    /// Returns a new `NV_GPU_THERMAL_SETTINGS_V2`; its fields are further populated when a
    /// function call occurs.
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

/// A cooler policy enum
#[repr(C)]
#[derive(Clone, Copy, Debug)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
enum NV_COOLER_POLICY {
    /// When a cooler is not available the policy is always `NONE` (0)
    NONE = 0,
    /// Explicitly set cooler speed
    MANUAL = 1,
    /// Performance profile
    PERF = 2,
    /// Discrete temperature steps
    DISCRETE = 4,
    /// Continuous temperature curve (hardware controlled)
    CONTINUOUS_HW = 8,
    /// Continuous temperature curver (software controlled)
    CONTINUOUS_SW = 16,
    /// The default policy; this will always change to the default policy for the GPU
    DEFAULT = 32,
}

/// The level (in %) for a GPU cooler
#[repr(C)]
#[derive(Clone, Copy)]
#[allow(non_snake_case)]
struct NvLevel {
    /// The level value
    level: i32,
    /// The associated policy
    policy: NV_COOLER_POLICY
}

/// Cooler levels for each cooler in the GPU
#[repr(C)]
#[allow(non_snake_case)]
struct NvGpuCoolerLevels {
    /// Struct version
    version: u32,
    coolers: [NvLevel; NVAPI_MAX_COOLERS_PER_GPU]
}

impl NvGpuCoolerLevels {
    /// Returns a new `NvGpuCoolerLevels`; it is usually populated upon function call
    fn new() -> NvGpuCoolerLevels {
        NvGpuCoolerLevels {
            version: NVAPI_VERSION::<NvGpuCoolerLevels>(1u32),
            coolers: [NvLevel { level: -1, policy: NV_COOLER_POLICY::NONE };
                        NVAPI_MAX_COOLERS_PER_GPU]
        }
    }

    /// Set the level of the cooler fan (in %)
    ///
    /// **Arguments**
    ///
    /// * `index` - The index of the cooler
    /// * `level` - The cooler level (in %)
    fn set_level(&mut self, index: u32, level: i32) {
        self.coolers[index as usize].level = level;
    }

    /// Set the policy governing the specified cooler fan
    ///
    /// **Arguments**
    ///
    /// * `index` - The index of the cooler
    /// * `poliy` - The `NV_COOLER_POLICY` for the cooler
    fn set_policy(&mut self, index: u32, policy: NV_COOLER_POLICY) {
        self.coolers[index as usize].policy = policy;
    }
}

/// A GPU cooler
#[repr(C)]
#[derive(Clone, Copy)]
#[allow(non_snake_case)]
struct NvCooler {
    cooler_type: i32,
    /// Controller from `NV_THERMAL_CONTROLLER`
    controller: i32,
    /// Default minimum speed in (%)
    default_min: i32,
    /// Default maximum speed in (%)
    default_max: i32,
    /// Current minimum speed in (%)
    current_min: i32,
    /// Current maximum speed in (%)
    current_max: i32,
    /// Current level of the GPU cooler (%)
    current_level: i32,
    /// Default cooler policy from `NV_COOLER_POLICY`
    default_policy: NV_COOLER_POLICY,
    /// Current cooler policy from `NV_COOLER_POLICY`
    current_policy: NV_COOLER_POLICY,
    /// Cooling target from `NV_THERMAL_TARGET`
    target: i32,
    control_type: i32,
    /// Cooler activity
    active: i32,
}

/// Cooler settings
#[repr(C)]
struct NvGpuCoolerSettings {
    /// Struct version
    version: u32,
    /// Number of available coolers
    count: u32,
    /// All `NvCooler`s
    coolers: [NvCooler; NVAPI_MAX_COOLERS_PER_GPU]
}

impl NvGpuCoolerSettings {
    /// Creates a new `NvGpuCoolerSettings` with sane defaults
    fn new() -> NvGpuCoolerSettings {
        NvGpuCoolerSettings {
            version: NVAPI_VERSION::<NvGpuCoolerSettings>(1u32),
            count: 0,
            coolers: [NvCooler {
                cooler_type: -1,
                controller: -1,
                default_min: -1,
                default_max: -1,
                current_min: -1,
                current_max: -1,
                current_level: -1,
                default_policy: NV_COOLER_POLICY::NONE,
                current_policy: NV_COOLER_POLICY::NONE,
                target: -1,
                control_type: -1,
                active: -1
            }; NVAPI_MAX_COOLERS_PER_GPU]
        }
    }
}

/// GPU utilisation
#[repr(C)]
#[allow(non_snake_case)]
struct NvGpuUsages {
    /// Struct version
    version: u32,
    /// GPU utilisation for all GPUs
    usage: [u32; NVAPI_MAX_USAGES_PER_GPU]
}

impl NvGpuUsages {
    /// Creates a new `NvGpuUsages` with sane defaults
    fn new() -> NvGpuUsages {
        NvGpuUsages {
            version: NVAPI_VERSION::<NvGpuUsages>(1),
            usage: [0u32; NVAPI_MAX_USAGES_PER_GPU]
        }
    }
}

/// Helper to convert `NVCtrlFanControlState` to `NV_COOLER_POLICY`
///
/// **Arguments**
///
/// * `state` - The `NVCtrlFanControlState` to convert
fn mode_to_policy(state: NVCtrlFanControlState) -> NV_COOLER_POLICY {
    match state {
        NVCtrlFanControlState::Auto => NV_COOLER_POLICY::DEFAULT,
        NVCtrlFanControlState::Manual => NV_COOLER_POLICY::MANUAL,
    }
}

/// NvidiaControl is the main struct that monitors and controls the
/// GPU fan state in addition with thermal and general information.
pub struct NvidiaControl {
    /// Current lower and upper limits
    pub limits: (u16, u16),
    /// All GPU handles
    handles: [NvPhysicalGpuHandle; NVAPI_MAX_PHYSICAL_GPUS],
    /// Number of available GPUs in the system
    _gpu_count: u32
}

impl NvidiaControl {

    /// Initialises the native library corresponding to the current OS.
    /// `init()` should be called when calling `NvidiaControl::new()` so
    /// there is no need to call it directly.
    pub fn init(lim: (u16, u16)) -> Result<NvidiaControl, String> {
        match unsafe { NvAPI_Initialize() } {
            0 => {
                let mut handle = [NvPhysicalGpuHandle::new(); NVAPI_MAX_PHYSICAL_GPUS];
                let mut count = 0 as u32;
                match unsafe { NvAPI_EnumPhysicalGPUs(&mut handle, &mut count) } {
                    0 => Ok(NvidiaControl{ limits: lim,
                        handles: handle, _gpu_count: count }),
                    i => Err(format!("NvAPI_EnumPhysicalGPUs() failed; error: {}", i))
                }
            },
            i => Err(format!("NvAPI_Initialize() failed; error: {}; No driver?", i))
        }
    }

}

impl Drop for NvidiaControl {
    fn drop(&mut self) {
        unsafe { NvAPI_Unload() };
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
        if gpu > (self._gpu_count - 1) {
            Err(format!("check_gpu_id() failed; id {} > {}",
                        gpu, self._gpu_count - 1))
        } else {
            Ok(())
        }
    }

}

impl NvFanController for NvidiaControl {

    fn get_temp(&self, gpu: u32) -> Result<i32, String> {

        self.check_gpu_id(gpu)?;

        let mut thermal = NV_GPU_THERMAL_SETTINGS_V2::new();
        match unsafe { NvAPI_GPU_GetThermalSettings(self.handles[gpu as usize],
                                                    0, &mut thermal) }
        {
            0 => Ok(thermal.temp(0)),
            i => Err(format!("NvAPI_GPU_GetThermalSettings() failed; error {}", i))
        }
    }

    fn gpu_count(&self) -> Result<u32, String> {
        Ok(self._gpu_count)
    }

    fn gpu_coolers(&self, gpu: u32) -> Result<Cow<Vec<u32>>, String> {

        self.check_gpu_id(gpu)?;

        let mut cooler_settings = NvGpuCoolerSettings::new();
        match unsafe { NvAPI_GPU_GetCoolerSettings(self.handles[gpu as usize],
                                                   NVAPI_COOLER_TARGET_ALL as _,
                                                   &mut cooler_settings) }
        {
            0 => {
                Ok(Cow::Owned(
                    (0..(cooler_settings.count as u32)).collect::<Vec<u32>>()))
            },
            i => Err(format!("NvAPI_GPU_GetCoolerSettings() failed; error {}", i))
        }
    }

    fn get_ctrl_status(&self, gpu: u32) -> Result<NVCtrlFanControlState, String> {

        self.check_gpu_id(gpu)?;

        let mut cooler_settings = NvGpuCoolerSettings::new();
        match unsafe { NvAPI_GPU_GetCoolerSettings(self.handles[gpu as usize],
                                                   NVAPI_COOLER_TARGET_ALL as _,
                                                   &mut cooler_settings) }
        {
            0 => {
                // Technically each cooler can have different policy; however for our
                // purpose all coolers should ideally have the same policy. So,
                // unless the policy was not set by nvfancontrol (which should not
                // be the case) coolers[0]...coolers[n] should follow the same policy.
                // Hence return only the status of coolers[0].
                // I'm wondering if it would make better sense to check all coolers and
                // return an error if policies differ.
                match cooler_settings.coolers[0].current_policy {
                    NV_COOLER_POLICY::MANUAL => { Ok(NVCtrlFanControlState::Manual) },
                    NV_COOLER_POLICY::PERF          | NV_COOLER_POLICY::CONTINUOUS_SW |
                    NV_COOLER_POLICY::CONTINUOUS_HW | NV_COOLER_POLICY::DEFAULT |
                    NV_COOLER_POLICY::DISCRETE => {
                            Ok(NVCtrlFanControlState::Auto)
                    },
                    i => {
                        Err(format!("NvAPI_GPU_GetCoolerSettings() unknown policy: {:?}", i))
                    }
                }

            },
            i => Err(format!("NvAPI_GPU_GetCoolerSettings() failed; error {}", i))
        }
    }

    fn set_ctrl_type(&self, gpu: u32, typ: NVCtrlFanControlState) -> Result<(), String> {

        self.check_gpu_id(gpu)?;

        let coolers = &*self.gpu_coolers(gpu)?;
        let mut levels = NvGpuCoolerLevels::new();
        let policy = mode_to_policy(typ);

        for c in coolers {
            // Retain existing fanspeed for cooler c
            let fanspeed = self.get_fanspeed(gpu, *c)?;

            levels.set_policy(*c, policy);
            levels.set_level(*c, fanspeed);
        }

        match unsafe { NvAPI_GPU_SetCoolerLevels(self.handles[gpu as usize],
                                                 NVAPI_COOLER_TARGET_ALL as _, &levels) }
        {
            0 => { Ok(()) },
            i => { return Err(format!("NvAPI_GPU_SetCoolerLevels() failed; error {}", i)) }
        }

    }

    fn get_fanspeed(&self, gpu: u32, id: u32) -> Result<i32, String> {

        self.check_gpu_id(gpu)?;

        let mut cooler_settings = NvGpuCoolerSettings::new();
        match unsafe { NvAPI_GPU_GetCoolerSettings(self.handles[gpu as usize], id,
                                                   &mut cooler_settings) }
        {
            0 => Ok(cooler_settings.coolers[id as usize].current_level),
            i => Err(format!("NvAPI_GPU_GetCoolerSettings() failed; error {}", i))
        }
    }

     // There is a bug here but it's not of nvfancontrol. If the GPU has more than
     // one cooler it is impossible to get its RPM reading since there is no function
     // for that in NVAPI; NvAPI_GPU_GetTachReading does not allow indexing on the
     // coolers. Unfortunately this RPM reading is probably meaningless on GPUs with
     // multiple coolers. It might be the RPM of the first coolers or who knows? There
     // is no documentation anywhere on the public NVAPI. In any case the GPU coolers
     // API is butchered anyway because reasons.
    fn get_fanspeed_rpm(&self, gpu: u32, _id: u32) -> Result<i32, String> {

        self.check_gpu_id(gpu)?;

        let mut speed = 0 as libc::c_uint;
        match unsafe { NvAPI_GPU_GetTachReading(self.handles[gpu as usize], &mut speed) } {
            0 => Ok(speed as i32),
            i => Err(format!("NvAPI_GPU_GetTachReading() failed; error {}", i))
        }
    }

    fn set_fanspeed(&self, gpu: u32, id: u32, speed: i32) -> Result<(), String> {

        self.check_gpu_id(gpu)?;

        let true_speed = self.true_speed(speed);

        // Retain the existing (global) policy for cooler
        let policy = match self.get_ctrl_status(gpu) {
            Ok(mode) => mode_to_policy(mode),
            Err(e) => { return Err(e); }
        };

        let mut levels = NvGpuCoolerLevels::new();
        levels.set_policy(id, policy);
        levels.set_level(id, true_speed as i32);
        match unsafe { NvAPI_GPU_SetCoolerLevels(self.handles[gpu as usize],
                                                 id, &levels) }
        {
            0 => { Ok(()) },
            i => { Err(format!("NvAPI_GPU_SetCoolerLevels() failed; error {}", i)) }
        }
    }

    fn get_version(&self) -> Result<String, String> {
        let mut b = NvAPI_ShortString::new();
        let mut v: libc::c_uint = 0;

        match unsafe { NvAPI_SYS_GetDriverAndBranchVersion(&mut v, &mut b) } {
            0 => Ok(format!("{:.2}", (v as f32)/100.0)),
            i => Err(format!("NvAPI_SYS_GetDriverAndBranchVersion() failed; error {:?}", i))
        }
    }

    fn get_adapter(&self, gpu: u32) -> Result<String, String> {

        self.check_gpu_id(gpu)?;

        let mut adapter = NvAPI_ShortString::new();
        match unsafe { NvAPI_GPU_GetFullName(self.handles[gpu as usize], &mut adapter) } {
            0 => Ok(adapter.to_string()),
            i => Err(format!("NvAPI_GPU_GetFullName() failed; error {:?}", i))
        }
    }

    fn get_utilization(&self, gpu: u32) -> Result<HashMap<&str, i32>, String> {

        self.check_gpu_id(gpu)?;

        let mut gpu_usages = NvGpuUsages::new();
        match unsafe { NvAPI_GPU_GetUsages(self.handles[gpu as usize],
                                           &mut gpu_usages) }
        {
            0 => {
                let mut ret: HashMap<&str, i32> = HashMap::with_capacity(3);
                ret.insert("graphics", gpu_usages.usage[2] as i32);
                ret.insert("memory", gpu_usages.usage[6] as i32);
                ret.insert("video", gpu_usages.usage[10] as i32);
                Ok(ret)
            },
            i => Err(format!("NvAPI_GPU_GetUsages() failed; error {}", i))
        }
    }
}
