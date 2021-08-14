extern crate nvctrl;
use nvctrl::{NvFanController, NvidiaControl, NVCtrlFanControlState};

#[macro_use]extern crate log;
use log::{Log, Record, LevelFilter, Metadata};

extern crate getopts;
use getopts::Options;

#[cfg(windows)]extern crate ctrlc;
#[cfg(unix)]extern crate nix;
#[cfg(unix)]use nix::sys::signal;
#[cfg(unix)]use std::ffi::OsString;

extern crate time;
extern crate dirs;

#[macro_use]extern crate serde_derive;

use std::env;
use std::thread;
use std::process;
use std::time::Duration;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::Write;
use std::path::PathBuf;
use std::net::{TcpListener, TcpStream, Shutdown};

pub mod config;
use self::config::{Curve};

pub mod fanflicker;
use fanflicker::{FanFlickerFix, FanFlickerRange};

pub mod fanspeedcurve;
use fanspeedcurve::FanspeedCurve;

const CONF_FILE: &'static str = "nvfancontrol.conf";
const MIN_VERSION: f32 = 352.09;
const DEFAULT_PORT: u32 = 12125;
const DEFAULT_CONFIG: &'static str = r"
[[gpu]]
id = {}
enabled = true
points = [[41, 20], [49, 30], [57, 45], [66, 55], [75, 63], [78, 72], [80, 80]]
";

static RUNNING: AtomicBool = AtomicBool::new(false);
static SRVING: AtomicBool = AtomicBool::new(false);
static LOGGER: Logger = Logger;

struct Logger;

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            eprintln!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) { }
}

struct NVFanManager {
    gpu: u32,
    ctrl: NvidiaControl,
    curve: FanspeedCurve,
    on_time: Option<f64>,
    force: bool,
    monitor: bool,
    fanflicker: Option<FanFlickerFix>,
}

impl Drop for NVFanManager {

    fn drop(&mut self) {
        if !self.monitor {
            debug!("Resetting fan control");
            self.reset_fan().unwrap();
        }
    }

}

impl NVFanManager {
    fn new(
        gpu: u32,
        curve: FanspeedCurve,
        force: bool,
        monitor: bool,
        limits: Option<(u16, u16)>,
        fanflickerrange: Option<FanFlickerRange>,
    ) -> Result<NVFanManager, String> {

        let ctrl = NvidiaControl::new(limits)?;
        let gpu_count = ctrl.gpu_count()?;
        match ctrl.get_version() {
            Ok(v) => {
                validate_driver_version(v)?;
            }
            Err(e) => {
                return Err(format!("Could not get driver version: {}", e))
            }
        };

        if gpu > gpu_count-1 {
            return Err(format!("GPU id {} is not valid; min: 0 max: {}", gpu, gpu_count-1));
        }

        let ret = NVFanManager {
            gpu: gpu,
            curve: curve,
            on_time: None,
            force: force,
            monitor: monitor,
            fanflicker: match fanflickerrange {
                Some(range) => {
                    let prev = (range.fickering_starts as i32).max(ctrl.get_fanspeed(0, gpu)?);
                    Some(FanFlickerFix::new(range, prev))
                },
                None => None
            },
            ctrl: ctrl,
        };

        Ok(ret)
    }

    fn set_manual_fan_speed(&self, speed: i32) -> Result<(), String> {
        #[cfg(target_os="windows")]
        if self.ctrl.is_rtx(self.gpu)? {
            self.ctrl.set_fancontrol(self.gpu, speed, NVCtrlFanControlState::Manual)?;
            return Ok(())
        }

        self.ctrl.set_ctrl_type(self.gpu, NVCtrlFanControlState::Manual)?;
        let coolers = &*self.ctrl.gpu_coolers(self.gpu)?;
        for c in coolers {
            self.ctrl.set_fanspeed(self.gpu, *c, speed)?;
        }
        Ok(())
    }

    fn reset_fan(&self) -> Result<(), String> {
        self.ctrl.set_ctrl_type(self.gpu, NVCtrlFanControlState::Auto)?;
        Ok(())
    }

    fn update(&mut self) -> Result<(), String> {

        if self.monitor {
            return Ok(())
        }

        let temp = self.ctrl.get_temp(self.gpu)? as u16;
        let ctrl_status = self.ctrl.get_ctrl_status(self.gpu)?;
        let coolers = &*self.ctrl.gpu_coolers(self.gpu)?;

        if coolers.len() == 0 {
            return Err("No coolers available to adjust".to_string());
        }

        let rpm = self.ctrl.get_fanspeed_rpm(self.gpu, coolers[0])?;

        let utilization = self.ctrl.get_utilization(self.gpu)?;
        let gutil = utilization.get("graphics");

        if rpm > 0 && !self.force {
            if let NVCtrlFanControlState::Auto = ctrl_status {
                debug!("Fan is enabled on auto control; doing nothing");
                return Ok(());
            };
        }

        let speed = self.curve.speed_y(temp);

        match (speed, self.on_time, &mut self.fanflicker) {
            (Some(speed), _, None) => {
                let since_epoch: time::Duration =
                    time::OffsetDateTime::now_utc() - time::OffsetDateTime::unix_epoch();
                self.on_time = Some(since_epoch.as_seconds_f64());
                self.set_manual_fan_speed(speed)
            },
            (None, Some(t), None) => {
                let since_epoch: time::Duration = time::OffsetDateTime::now_utc() - time::OffsetDateTime::unix_epoch();
                let now = since_epoch.as_seconds_f64();
                let diff = now - t;

                debug!("{} seconds elapsed since fan was last on", diff as u64);

                // if utilization can't be retrieved the utilization leg is
                // always false and ignored
                if diff < 240.0 || gutil.unwrap_or(&-1) > &25 {
                    self.set_manual_fan_speed(self.curve.minspeed())
                } else {
                    debug!("Grace period expired; turning fan off");
                    self.on_time = None;
                    self.reset_fan()
                }
            },
            (None, None, None) => {
                // If no point is found then fan should be off
                self.on_time = None;
                self.reset_fan()
            },
            (Some(speed), _, Some(fff)) => {
                let speed = fff.fix_speed(rpm, speed);
                self.set_manual_fan_speed(speed)
            },
            (None, _, Some(fff)) => {
                // The jump from 0 to some RPM (presumably in the flicker range) will
                // cause flickering, which will then raise the RPM too much. So keep
                // it at the lowest speed.
                debug!("FanFlickerFix: preventing fan-off");
                let new_speed = fff.fix_speed(rpm, fff.minimum());
                self.set_manual_fan_speed(new_speed)
            },
        }
    }
}

#[cfg(unix)]
extern fn sigint(_: i32) {
    debug!("Interrupt signal");
    RUNNING.store(false, Ordering::Relaxed);
}

#[cfg(windows)]
fn sigint() {
    debug!("Interrupt signal");
    RUNNING.store(false, Ordering::Relaxed);
}

#[cfg(unix)]
fn register_signal_handlers() -> Result<(), String> {
    let sigaction = signal::SigAction::new(signal::SigHandler::Handler(sigint),
                                           signal::SaFlags::empty(),
                                           signal::SigSet::empty());
    for &sig in &[signal::SIGINT, signal::SIGTERM, signal::SIGQUIT] {
        match unsafe { signal::sigaction(sig, &sigaction) } {
            Ok(_) => {} ,
            Err(err) => {
                return Err(format!("Could not register SIG #{:?} handler: {:?}",
                                   sig ,err));
            }
        };
    }
    Ok(())
}

#[cfg(windows)]
fn register_signal_handlers() -> Result<(), String> {
    match ctrlc::set_handler(sigint) {
        Ok(_) => { Ok(()) } ,
        Err(err) => {
            Err(format!("Could not register signal handler: {:?}", err))
        }
    }
}

fn parse_ascending_arg_pair(nm: &str, res: &str) -> Result<Option<(u16,u16)>, String> {
    let parts: Vec<&str> = res.split(',').map(|s| s.trim()).collect();
    let invalidopt = format!("Invalid option for \"-{}\"", nm);
    if parts.len() == 1 {
        if parts[0] != "0" {
            Err(format!("{}: {}", invalidopt, parts[0]))
        } else {
            Ok(None)
        }
    } else if parts.len() == 2 {
        match (parts[0].parse::<u16>(), parts[1].parse::<u16>()) {
            (Err(e), _) =>
                Err(format!("{}: could not parse {} as lower limit: {}", invalidopt, parts[0], e)),
            (_, Err(e)) =>
                Err(format!("{}: could not parse {} as upper limit: {}", invalidopt, parts[1], e)),
            (Ok(lower), Ok(upper)) if lower > upper =>
                Err(format!("{}: lower limit {} is greater than upper limit {}", invalidopt, lower, upper)),
            (Ok(lower), Ok(upper)) if upper > 100 => {
                debug!("Upper limit {} is > 100; clipping to 100", upper);
                Ok(Some((lower, 100)))
            },
            (Ok(lower), Ok(upper)) =>
                Ok(Some((lower, upper))),
        }
    } else {
        Err(format!("Invalid argument for \"-{}\": {:?}", nm, parts))
    }
}

#[cfg(unix)]
fn find_global_config_dirs() -> Vec<PathBuf> {

    // This is essentially the same as in xdg-rs
    fn paths(paths: OsString) -> Option<Vec<PathBuf>> {
        let p = env::split_paths(&paths)
                    .map(PathBuf::from)
                    .filter(|ref path| path.is_absolute())
                    .collect::<Vec<_>>();
        if p.is_empty() {
            None
        } else {
            Some(p)
        }
    }

    env::var_os("XDG_CONFIG_DIRS")
        .and_then(paths)
        .unwrap_or(vec![PathBuf::from("/etc/xdg")])

}

fn find_config_file() -> Option<PathBuf> {

    match dirs::config_dir() {
        Some(path) => {
            let mut conf_path = PathBuf::from(path.to_str().unwrap());
            conf_path.push(CONF_FILE);

            // The "local" configuration file was found and supersedes all
            // others. We are done.
            if conf_path.as_path().exists() {
                return Some(conf_path);
            }
        },
        _ => {}
    };

    // If no "local" configuration file was found check the global ones
    // unix-only
    #[cfg(unix)] {
        let config_dirs = find_global_config_dirs();

        for dir in config_dirs {
            let mut conf_path = PathBuf::from(dir.to_str().unwrap());
            conf_path.push(CONF_FILE);
            if conf_path.as_path().exists() {
                return Some(conf_path);
            }
        }
    }

    None
}

fn make_options() -> Options {
    let mut opts = Options::new();

    opts.optflag("d", "debug", "Enable debug messages");
    opts.optopt("l", "limits",
        "Comma separated lower and upper limits, use 0 to disable,
        default: 20,80", "LOWER,UPPER");
    opts.optopt("g", "gpu", "GPU to adjust; must be >= 0", "GPU");
    opts.optflag("p", "print-coolers", "Print available GPUs and coolers");
    opts.optflag("f", "force", "Always use the custom curve even if the fan is
                 already spinning in auto mode");
    opts.optflag("m", "monitor-only", "Do not update the fan speed and control
                 mode; just log temperatures and fan speeds");
    opts.optflag("j", "json-output", "Print a json representation of the data
                 to stdout (useful for parsing)");
    opts.optflagopt("t", "tcp-server", "Print a json representation of the data
                    over a tcp port. Can be optionally followed by the port
                    number over which the server will listen for incoming
                    connections", "PORT");
    opts.optopt("r", "fanflicker", "Range in which fan flicker is prevented,
                     specify as with \"-l\". Also makes fan spin with at
                     least the specified lower limit which must not be zero.",
                     "LOWER,UPPER");
    opts.optflag("h", "help", "Print this help message");

    opts
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    println!("{}", opts.usage(&brief));
}

#[derive(Serialize, Deserialize, Debug)]
struct GPUData {
    timespec: i64,
    temp: i32,
    speed: Vec<i32>,
    rpm: Vec<i32>,
    load: i32,
    mode: Option<NVCtrlFanControlState>
}

impl GPUData {
    fn new(mgr: &NVFanManager, gpu: u32) -> Result<GPUData, String> {

        let coolers = mgr.ctrl.gpu_coolers(gpu)?;
        let temp = mgr.ctrl.get_temp(gpu)?;
        let mut speed: Vec<i32> = Vec::with_capacity(coolers.len());
        let mut rpm: Vec<i32> = Vec::with_capacity(coolers.len());

        for i in 0..coolers.len() {
            let current_speed = mgr.ctrl.get_fanspeed(gpu, coolers[i])?;
            let current_rpm = mgr.ctrl.get_fanspeed_rpm(gpu, coolers[i])?;
            speed.push(current_speed);
            rpm.push(current_rpm);
        }


        Ok(GPUData {
            timespec: -1,
            temp: temp,
            speed: speed,
            rpm: rpm,
            load: -1,
            mode: None
        })
    }

    fn update_from_mgr(&mut self, timespec: i64, mgr: &NVFanManager, gpu: u32) {
        self.timespec = timespec;
        self.temp = mgr.ctrl.get_temp(gpu).unwrap();
        self.load = match mgr.ctrl.get_utilization(gpu).unwrap().get("graphics") {
            Some(v) => *v,
            None => -1
        };
        self.mode = mgr.ctrl.get_ctrl_status(gpu).ok();
        let coolers_ref = mgr.ctrl.gpu_coolers(gpu).unwrap();
        for i in 0..coolers_ref.len() {
            self.rpm[i] = mgr.ctrl.get_fanspeed_rpm(gpu, coolers_ref[i]).unwrap();
            self.speed[i] = mgr.ctrl.get_fanspeed(gpu, coolers_ref[i]).unwrap();
        }

    }
}

fn serve_tcp(data: Arc<RwLock<GPUData>>, port: u32) {
    let l = TcpListener::bind(format!(":::{}", port).as_str()).unwrap();
    SRVING.store(true, Ordering::Relaxed);
    info!("Spinning up TCP server at {:?}", l.local_addr().unwrap());
    'server: loop {
        match l.accept() {
            Ok((mut s, client)) => {
                if RUNNING.load(Ordering::Relaxed) {
                    debug!("Incoming TCP connection: {:?}", client);
                    let raw_data = data.read().unwrap();
                    let json = format!("{}\n",
                                       serde_json::to_string(&*raw_data).unwrap());
                    s.write_all(json.as_bytes()).ok();
                } else {
                    SRVING.store(false, Ordering::Relaxed);
                    break 'server;
                }
                s.shutdown(Shutdown::Both).ok();
            }
            Err(e) => {
                error!("TCP server error: {:?}", e);
            }
        }
    };
    debug!("TCP server terminated")
}

fn list_gpus_and_coolers() -> Result<(), String> {
    let ctrl = NvidiaControl::new(None)?;
    let gpu_count = ctrl.gpu_count()?;

    println!("Found {} available GPU(s)", gpu_count);

    for gpu in 0..gpu_count {
        let name = ctrl.get_adapter(gpu)?;
        println!("GPU #{}: {} ", gpu, name);
        let coolers = &*ctrl.gpu_coolers(gpu)?;
        for c in coolers {
            println!(" COOLER-{}", c)
        }
    }

    Ok(())

}

fn make_default_curve(gpu: u32) -> Vec<(u16, u16)> {
    let mut conf = String::from("");
    for i in 0..gpu+1 {
        conf = conf + &(DEFAULT_CONFIG.replace("{}", &i.to_string()));
    }
    let c = config::from_string(&conf).unwrap();
    debug!("Default configuration loaded");
    debug!("{}", conf.trim());
    c.points(gpu as usize).to_vec()
}

fn validate_gpu_id(gpu: u32) -> Result<(), String> {
    let ctrl = NvidiaControl::new(None)?;
    let count = ctrl.gpu_count()?;

    if gpu > (count - 1) {
        Err(format!("Invalid GPU id: {}; max: {}", gpu, count-1))
    } else {
        Ok(())
    }
}

fn validate_driver_version(version: String) -> Result<(), String> {
    let parts: Vec<&str> = version.split(".").collect();

    let major = parts[0];
    let minor: &str;
    if parts.len() < 2 {
        minor = "00";
    } else {
        minor = parts[1];
    }

    let version_str = format!("{}.{}", major, minor);
    let version_num = version_str.parse::<f32>();

    if version_num.is_err() {
        return Err("Could not parse driver version".to_string());
    }

    if version_num.unwrap() < MIN_VERSION {
        let err = format!("Unsupported driver version; need >= {:.2}",
                          MIN_VERSION);
        return Err(err);
    }

    Ok(())

}

trait ProcessOrDefault<T> {
    fn opt_process_or_default<F>(&self, nm: &str, on_arg: F, default: T) -> T
        where F: Fn(&str) -> T;
}

impl<T> ProcessOrDefault<T> for getopts::Matches {
    fn opt_process_or_default<F>(&self, nm: &str, on_arg: F, default: T) -> T
        where F: Fn(&str) -> T
    {
        if self.opt_present(nm) {
            match self.opt_str(nm) {
                Some(arg) => on_arg(&arg),
                None => panic!("{} was not a getopts::Options::optopt() option", nm)
            }
        } else {
            default
        }
    }
}

pub fn main() {

    let args: Vec<String> = env::args().collect();
    let opts = make_options();

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Could not parse command line: {:?}", e);
            process::exit(1);
       }
    };

    if matches.opt_present("h") {
        print_usage(&args[0].clone(), opts);
        return;
    }

    if matches.opt_present("p") {
        if let Err(e) = list_gpus_and_coolers() { error!("Failed to list adapters: {}", e); }
        return;
    }

    let log_level = if matches.opt_present("d") {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(log_level);

    let force_update = matches.opt_present("f");

    let limits = matches.opt_process_or_default(
        "l",
        |arg: &str| {
            match parse_ascending_arg_pair("l", arg) {
                Ok(lims) => lims,
                Err(e) => {
                    error!("{}", e);
                    process::exit(1);
                }
            }
        },
        // Default limits
        Some((20, 80))
    );


    let gpu = matches.opt_process_or_default(
        "g",
        |arg: &str| {
            match arg.parse::<u32>() {
                Ok(v) => {
                    validate_gpu_id(v).unwrap_or_else(|e| {
                        error!("{}", e);
                        process::exit(1);
                    });
                    v
                },
                Err(e) => {
                    error!("Option \"-g\" present but non-valid: \"{}\": {}", e, arg);
                    process::exit(1);
                }
            }
        },
        0
    );

    match register_signal_handlers() {
        Ok(_) => {},
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    }

    let mut fanflicker = None;

    let points: Vec<(u16, u16)> = match find_config_file() {
        Some(path) => {
            info!("Loading configuration file: {:?}", path);
            match config::from_file(path) {
                Ok(c) => {
                    fanflicker = c.fanflicker(gpu as usize);
                    c.points(gpu as usize).to_vec()
                }
                Err(e) => {
                    warn!("{}; using default curve", e);
                    make_default_curve(gpu)
                }
            }
        },
        None => {
            warn!("No config file found; using default curve");
            make_default_curve(gpu)
        }
    };

    debug!("Curve points: {:?}", points);

    let curve = match FanspeedCurve::new(points) {
        Ok(curve) => curve,
        Err(msg) => {
            error!("{}", msg.to_string());
            process::exit(1);
        }
    };

    let fanflicker = matches.opt_process_or_default(
        "r",
        |arg: &str| {
            match parse_ascending_arg_pair("r", arg) {
                Ok(fanflicker) => fanflicker,
                Err(e) => {
                    error!("{}", e);
                    process::exit(1);
                }
            }
        },
        // from the config file, overridden by the commandline if present
        fanflicker
    );

    let fanflickerrange = match fanflicker {
        Some(range) => match FanFlickerRange::new(range, &curve, &limits) {
            Ok(range) => Some(range),
            Err(e) => {
                error!("{}", e);
                process::exit(1);
            },
        }
        None => None,
    };

    let monitor_only = matches.opt_present("m");

    let mut mgr = match NVFanManager::new(gpu, curve, force_update, monitor_only, limits, fanflickerrange) {
        Ok(m) => m,
        Err(s) => {
            error!("{}", s);
            process::exit(1);
        }
    };

    info!("NVIDIA driver version: {}",
          mgr.ctrl.get_version().unwrap());
    let gpu_count = mgr.ctrl.gpu_count().unwrap();
    for i in 0u32..gpu_count {
        info!("NVIDIA graphics adapter #{}: {}", i,
              mgr.ctrl.get_adapter(i).unwrap());
        match mgr.ctrl.gpu_coolers(i) {
            Ok(array) => {
                info!("  GPU #{} coolers: {}", i,
                      array.iter()
                           .map(|x| format!("COOLER-{}", x))
                           .collect::<Vec<String>>().join(", "));
            },
            Err(_) => { warn!("Could not get GPU cooler indices or unsupported OS") }
        };
    }

    let timeout = Duration::new(2, 0);
    RUNNING.store(true, Ordering::Relaxed);

    if monitor_only {
        info!("Option \"-m\" is present; curve will have no actual effect");
    }

    let json_output = matches.opt_present("j");

    let data = Arc::new(RwLock::new(GPUData::new(&mgr, gpu).unwrap()));

    let server_port = if matches.opt_present("t") {
        let srv_data = data.clone();
        let strport = format!("{}", DEFAULT_PORT);
        let port: u32 = match matches.opt_default("t", strport.as_str()) {
            Some(s) => {
                match s.parse::<u32>() {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("Could not parse port number: {:?}", e);
                        DEFAULT_PORT
                    }
                }
            }
            None => {
                warn!("No port provided for server, using default");
                DEFAULT_PORT
            }
        };
        thread::spawn(move || { serve_tcp(srv_data, port) });
        port
    } else {
        DEFAULT_PORT
    };

    // Main loop
    loop {
        if !RUNNING.load(Ordering::Relaxed) {
            debug!("Exiting");
            break;
        }

        if let Err(e) = mgr.update() {
            error!("Could not update fan speed: {}", e)
        };

        let mut raw_data = data.write().unwrap();
        let since_epoch: time::Duration =
                time::OffsetDateTime::now_utc() - time::OffsetDateTime::unix_epoch();
        (*raw_data).update_from_mgr(since_epoch.whole_seconds(), &mgr, gpu);
        drop(raw_data);

        let raw_data = data.read().unwrap();
        debug!("GPU #{} Temp: {}; Speed: {:?} RPM ({:?}%); Load: {}%; Mode: {}",
            gpu, raw_data.temp, raw_data.rpm, raw_data.speed, raw_data.load,
            match raw_data.mode {
                Some(NVCtrlFanControlState::Auto) => "Auto",
                Some(NVCtrlFanControlState::Manual) => "Manual",
                None => "ERR"
            });

        if json_output {
            println!("{}", serde_json::to_string(&*raw_data).unwrap());
        }

        thread::sleep(timeout);
    }

    if SRVING.load(Ordering::Relaxed) {
        // Flush the server
        let _ = TcpStream::connect(format!(":::{}", server_port).as_str());
    }

}
