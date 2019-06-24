extern crate nvctrl;
use nvctrl::{NvFanController, NvidiaControl, NVCtrlFanControlState};

#[macro_use]
extern crate log;
use log::{Log, Record, LevelFilter, Metadata};

extern crate getopts;
use getopts::Options;

#[cfg(windows)] extern crate ctrlc;
#[cfg(unix)] extern crate nix;
#[cfg(unix)] use nix::sys::signal;
#[cfg(unix)] use std::ffi::OsString;

extern crate time;
extern crate dirs;

#[macro_use] extern crate serde_derive;
extern crate serde_json;

use std::io::{BufReader, BufRead};
use std::fs::File;
use std::env;
use std::thread;
use std::process;
use std::time::Duration;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::Write;
use std::path::PathBuf;
use std::net::{TcpListener, TcpStream, Shutdown};

const CONF_FILE: &'static str = "nvfancontrol.conf";
const MIN_VERSION: f32 = 352.09;
const DEFAULT_PORT: u32 = 12125;
const DEFAULT_CURVE: [(u16, u16); 7] = [(41, 20), (49, 30), (57, 45), (66, 55),
                                        (75, 63), (78, 72), (80, 80)];

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
    points: Vec<(u16, u16)>,
    on_time: Option<f64>,
    force: bool
}

impl Drop for NVFanManager {

    fn drop(&mut self) {
        debug!("Resetting fan control");
        self.reset_fan().unwrap();
    }

}

impl NVFanManager {
    fn new(
            gpu: u32, points: Vec<(u16, u16)>, force: bool, limits: Option<(u16, u16)>
        ) -> Result<NVFanManager, String> {

        let ctrl = NvidiaControl::new(limits)?;
        let gpu_count = ctrl.gpu_count()?;
        let version: f32 = match ctrl.get_version() {
            Ok(v) => {
                v.parse::<f32>().unwrap()
            }
            Err(e) => {
                return Err(format!("Could not get driver version: {}", e))
            }
        };

        if version < MIN_VERSION {
            let err = format!("Unsupported driver version; need >= {:.2}",
                              MIN_VERSION);
            return Err(err);
        }

        if gpu > gpu_count-1 {
            return Err(format!("GPU id {} is not valid; min: 0 max: {}", gpu, gpu_count-1));
        }

        debug!("Curve points: {:?}", points);

        let ret = NVFanManager {
            gpu: gpu,
            ctrl: ctrl,
            points: points,
            on_time: None,
            force: force
        };

        Ok(ret)
    }

    fn set_fan(&self, id: u32, speed: i32) -> Result<(), String> {
        self.ctrl.set_ctrl_type(self.gpu, NVCtrlFanControlState::Manual)?;
        self.ctrl.set_fanspeed(self.gpu, id, speed)?;
        Ok(())
    }

    fn set_fans(&self, speed: i32) -> Result<(), String> {
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

        let temp = self.ctrl.get_temp(self.gpu)? as u16;
        let ctrl_status = self.ctrl.get_ctrl_status(self.gpu)?;
        let coolers = &*self.ctrl.gpu_coolers(self.gpu)?;

        if coolers.len() == 0 {
            return Err("No coolers available to adjust".to_string());
        }

        let rpm = self.ctrl.get_fanspeed_rpm(self.gpu, coolers[0])?;

        let utilization = self.ctrl.get_utilization(self.gpu)?;
        let gutil = utilization.get("graphics");

        let pfirst = self.points.first().unwrap();
        let plast = self.points.last().unwrap();


        if rpm > 0 && !self.force {
            if let NVCtrlFanControlState::Auto = ctrl_status {
                debug!("Fan is enabled on auto control; doing nothing");
                return Ok(());
            };
        }

        if temp < pfirst.0 && self.on_time.is_some() {
            let now = time::precise_time_s();
            let dif = now - self.on_time.unwrap();

            debug!("{} seconds elapsed since fan was last on", dif as u64);

            // if utilization can't be retrieved the utilization leg is
            // always false and ignored
            if dif < 240.0 || gutil.unwrap_or(&-1) > &25 {
                if let Err(e) = self.set_fans(pfirst.1 as i32) { return Err(e); }
            } else {
                debug!("Grace period expired; turning fan off");
                self.on_time = None;
            }
            return Ok(());
        }

        if temp > plast.0 {
            debug!("Temperature outside curve; setting to max");

            for c in coolers {
                if let Err(e) = self.set_fan(*c, plast.1 as i32) { return Err(e); }
            }

            return Ok(());
        }

        for i in 0..(self.points.len()-1) {
            let p1 = self.points[i];
            let p2 = self.points[i+1];

            if temp >= p1.0 && temp < p2.0 {
                let dx = p2.0 - p1.0;
                let dy = p2.1 - p1.1;

                let slope = (dy as f32) / (dx as f32);

                let y = (p1.1 as f32) + (((temp - p1.0) as f32) * slope);

                self.on_time = Some(time::precise_time_s());

                if let Err(e) = self.set_fans(y as i32) { return Err(e); }

                return Ok(());
            }
        }

        // If no point is found then fan should be off
        self.on_time = None;
        if let Err(e) = self.reset_fan() { return Err(e); }

        Ok(())

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

fn make_limits(res: String) -> Result<Option<(u16,u16)>, String> {
    let parts: Vec<&str> = res.split(',').collect();
    if parts.len() == 1 {
        if parts[0] != "0" {
            Err(format!("Invalid option for \"-l\": {}", parts[0]))
        } else {
            Ok(None)
        }
    } else if parts.len() == 2 {
        let lower = match parts[0].parse::<u16>() {
            Ok(num) => num,
            Err(e) => {
                return Err(format!("Could not parse {} as lower limit: {}", parts[0], e));
            }
        };
        let upper = match parts[1].parse::<u16>() {
            Ok(num) => num,
            Err(e) => {
                return Err(format!("Could not parse {} as upper limit: {}", parts[1], e));
            }
        };

        if upper < lower {
            return Err(format!("Lower limit {} is greater than the upper {}", lower, upper));
        }

        if upper > 100 {
            debug!("Upper limit {} is > 100; clipping to 100", upper);
            Ok(Some((lower, 100)))
        } else {
            Ok(Some((lower, upper)))
        }
    } else {
        Err(format!("Invalid argument for \"-l\": {:?}", parts))
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

fn curve_from_conf(path: PathBuf) -> Result<Vec<(u16,u16)>, String> {

    let mut curve: Vec<(u16, u16)>;

    match File::open(path.to_str().unwrap()) {
        Ok(file) => {
            curve = Vec::new();

            for raw_line in BufReader::new(file).lines() {
                let line = raw_line.unwrap();
                let trimmed = line.trim();
                if trimmed.starts_with('#') {
                    continue;
                }

                let parts = trimmed.split_whitespace()
                                   .collect::<Vec<&str>>();

                if parts.len() < 2 {
                    warn!("Invalid line \"{}\", ignoring", line);
                    continue
                }

                let x = match parts[0].parse::<u16>() {
                    Ok(val) => val,
                    Err(e) => {
                        warn!("Could not parse value {}: {}, ignoring",
                               parts[0], e);
                        continue;
                    }
                };

                let y = match parts[1].parse::<u16>() {
                    Ok(val) => val,
                    Err(e) => {
                        warn!("Could not parse value {}: {}, ignoring",
                               parts[1], e);
                        continue;
                    }
                };

                curve.push((x, y));
            }
            if curve.len() < 2 {
                Err(String::from("At least two points are required for \
                                 the curve"))
            } else {
                Ok(curve)
            }
        },
        Err(e) => Err(format!("Could not read configuration file {:?}: {}",
                      path, e))
    }

}

fn make_options() -> Options {
    let mut opts = Options::new();

    opts.optflag("d", "debug", "Enable debug messages");
    opts.optopt("l", "limits",
        "Comma separated lower and upper limits, use 0 to disable,
        default: 20,80", "LOWER,UPPER");
    opts.optopt("g", "gpu", "GPU to adjust; must be > 0", "GPU");
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

pub fn main() {

    let args: Vec<String> = env::args().collect();
    let opts = make_options();

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => panic!("Could not parse command line: {:?}", e)
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

    let limits: Option<(u16, u16)>;
    if matches.opt_present("l") {
        match matches.opt_str("l") {
            Some(res) => {
                match make_limits(res) {
                    Ok(lims) => { limits = lims },
                    Err(e) => {
                        error!("{}", e);
                        process::exit(1);
                    }
                }
            },
            None => {
                error!("Option \"-l\" present but no argument provided");
                process::exit(1);
            }
        }
    } else {
        // Default limits
        limits = Some((20, 80));
    }

    let gpu: u32;

    if matches.opt_present("g") {
        gpu = match matches.opt_str("g") {
            Some(g) => {
                match g.parse::<u32>() {
                    Ok(v) => v,
                    Err(e) => {
                        error!("Option \"-g\" present but non-valid: \"{}\": {}", e, g);
                        process::exit(1);
                    }
                }
            },
            None => { 0 }
        }
    } else {
        gpu = 0;
    }

    match register_signal_handlers() {
        Ok(_) => {},
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    }

    let curve: Vec<(u16, u16)> = match find_config_file() {
        Some(path) => {
            match curve_from_conf(path) {
                Ok(c) => c,
                Err(e) => {
                    warn!("{}; using default curve", e);
                    DEFAULT_CURVE.to_vec()
                }
            }
        },
        None => {
            warn!("No config file found; using default curve");
            DEFAULT_CURVE.to_vec()
        }
    };

    let mut mgr = match NVFanManager::new(gpu, curve, force_update, limits) {
        Ok(m) => m,
        Err(s) => {
            error!("{}", s);
            process::exit(1);
        }
    };

    info!("NVIDIA driver version: {:.2}",
          mgr.ctrl.get_version().unwrap().parse::<f32>().unwrap());
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

    let monitor_only = matches.opt_present("m");
    if monitor_only {
        info!("Option \"-m\" is present; curve will have no actual effect");
    }

    let json_output = matches.opt_present("j");

    let data = Arc::new(RwLock::new(GPUData::new(&mgr, 0).unwrap()));

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

        if !monitor_only {
            if let Err(e) = mgr.update() {
                error!("Could not update fan speed: {}", e)
            };
        }

        let mut raw_data = data.write().unwrap();
        (*raw_data).update_from_mgr(time::now().to_timespec().sec, &mgr, 0);
        drop(raw_data);

        let raw_data = data.read().unwrap();
        debug!("Temp: {}; Speed: {:?} RPM ({:?}%); Load: {}%; Mode: {}",
            raw_data.temp, raw_data.rpm, raw_data.speed, raw_data.load,
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
