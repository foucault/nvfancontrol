extern crate nvctrl;
use nvctrl::{NvidiaControl, NVCtrlFanControlState};

#[macro_use]
extern crate log;
use log::{Log, LogRecord, LogLevelFilter, LogMetadata, SetLoggerError};

extern crate getopts;
use getopts::Options;

extern crate nix;
use nix::sys::signal;

extern crate time;

extern crate xdg;
use xdg::BaseDirectories;

use std::io::{BufReader, BufRead};
use std::fs::File;
use std::env;
use std::thread;
use std::process;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering, ATOMIC_BOOL_INIT};
use std::io::Write;

// http://stackoverflow.com/questions/27588416/how-to-send-output-to-stderr
macro_rules! errln(
    ($($arg:tt)*) => (
        match writeln!(&mut ::std::io::stderr(), $($arg)* ) {
            Ok(_) => {},
            Err(x) => panic!("Unable to write to stderr: {}", x),
        }
    )
);

const CONF_FILE: &'static str = "nvfancontrol.conf";
const MIN_VERSION: f32 = 352.09;

static RUNNING: AtomicBool = ATOMIC_BOOL_INIT;

struct Logger;

impl Log for Logger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= log::max_log_level()
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            errln!("{} - {}", record.level(), record.args());
        }
    }
}

impl Logger {
    pub fn new(level: LogLevelFilter) -> Result<(), SetLoggerError> {
        log::set_logger(|max_level| {
            max_level.set(level);
            Box::new(Logger)
        })
    }
}

struct NVFanManager {
    ctrl: NvidiaControl,
    points: Vec<(u16, u16)>,
    on_time: Option<f64>,
    force: bool
}

impl Drop for NVFanManager {

    fn drop(&mut self) {
        debug!("Resetting fan control");
        self.reset_fan();
    }

}

impl NVFanManager {
    fn new(
            points: Vec<(u16, u16)>, force: bool, limits: Option<(u16, u16)>
        ) -> Result<NVFanManager, String> {

        let ctrl = NvidiaControl::new(limits);
        let version: f32 = ctrl.get_version().parse::<f32>().unwrap();

        if version < MIN_VERSION {
            let err = format!("Unsupported driver version; need >= {:.2}",
                              MIN_VERSION);
            return Err(err);
        }

        if points.len() < 2 {
            return Err(format!("Need at least two points for the curve"));
        };

        debug!("Curve points: {:?}", points);

        let ret = NVFanManager {
            ctrl: ctrl,
            points: points,
            on_time: None,
            force: force
        };

        Ok(ret)
    }

    fn set_fan(&self, speed: i32) {
        self.ctrl.set_ctrl_type(NVCtrlFanControlState::Manual);
        self.ctrl.set_fanspeed(speed);
    }

    fn reset_fan(&self) {
        self.ctrl.set_ctrl_type(NVCtrlFanControlState::Auto);
    }

    fn update(&mut self) {

        let temp = self.ctrl.get_temp() as u16;
        let ctrl_status = self.ctrl.get_ctrl_status().unwrap();
        let rpm = self.ctrl.get_fanspeed_rpm();

        let utilization = self.ctrl.get_utilization();
        let gutil = utilization.get("graphics");

        let pfirst = self.points.first().unwrap();
        let plast = self.points.last().unwrap();

        if rpm > 0 && !self.force {
            match ctrl_status {
                NVCtrlFanControlState::Auto => {
                    debug!("Fan is enabled on auto control; doing nothing");
                    return;
                },
                _ => {}
            };
        }

        if temp < pfirst.0 && self.on_time.is_some() {
            let now = time::precise_time_s();
            let dif = now - self.on_time.unwrap();

            debug!("{} seconds elapsed since fan was last on", dif as u64);

            // if utilization can't be retrieved the utilization leg is
            // always false and ignored
            if dif < 240.0 || gutil.unwrap_or(&-1) > &25 {
                self.set_fan(pfirst.1 as i32);
            } else {
                debug!("Grace period expired; turning fan off");
                self.on_time = None;
            }
            return;
        }

        if temp > plast.0 {
            debug!("Temperature outside curve; setting to max");
            self.set_fan(plast.1 as i32);
            return;
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
                self.set_fan(y as i32);

                return;
            }
        }

        // If no point is found then fan should be off
        self.on_time = None;
        self.reset_fan();

    }
}

extern fn sigint(_: i32) {
    debug!("Interrupt signal");
    RUNNING.store(false, Ordering::Relaxed);
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    println!("{}", opts.usage(&brief));
}

pub fn main() {

    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut opts = Options::new();

    opts.optflag("d", "debug", "Enable debug messages");
    opts.optopt("l", "limits",
        "Comma separated lower and upper limits, use 0 to disable,
        default: 20,80", "LOWER,UPPER");
    opts.optflag("f", "force", "Always use the custom curve even if the fan is
                 already spinning in auto mode");
    opts.optflag("h", "help", "Print this help message");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => panic!("Could not parse command line: {:?}", e)
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let log_level: LogLevelFilter;
    if matches.opt_present("d") {
        log_level = LogLevelFilter::Debug;
    } else {
        log_level = LogLevelFilter::Info;
    }

    let force_update: bool;
    if matches.opt_present("f") {
        force_update = true
    } else {
        force_update = false
    }

    let limits: Option<(u16, u16)>;
    if matches.opt_present("l") {
        match matches.opt_str("l") {
            Some(res) => {
                let parts: Vec<&str> = res.split(',').collect();
                if parts.len() == 1 {
                    if parts[0] != "0" {
                        errln!("Invalid option for \"-l\": {}", parts[0]);
                        process::exit(1);
                    }
                    else {
                        limits = None;
                    }
                } else if parts.len() == 2 {
                    let lower = match parts[0].parse::<u16>() {
                        Ok(num) => num,
                        Err(e) => {
                            errln!("Could not parse {} as lower limit: {}",
                                   parts[0], e);
                            process::exit(1);
                        }
                    };
                    let upper = match parts[1].parse::<u16>() {
                        Ok(num) => num,
                        Err(e) => {
                            errln!("Could not parse {} as upper limit: {}",
                                   parts[1], e);
                            process::exit(1);
                        }
                    };
                    if upper < lower {
                        errln!("Lower limit {} is greater than the upper {}",
                               lower, upper);
                        process::exit(1);
                    }
                    if upper > 100 {
                        debug!("Upper limit {} is > 100; clipping to 100", upper);
                        limits = Some((lower, 100));
                    } else {
                        limits = Some((lower, upper));
                    }
                } else {
                    errln!("Invalid argument for \"-l\": {:?}", parts);
                    process::exit(1);
                }
            },
            None => {
                errln!("Option \"-l\" present but no argument provided");
                process::exit(1);
            }
        }
    } else {
        // Default limits
        limits = Some((20, 80));
    }

    match Logger::new(log_level) {
        Ok(v) => v,
        Err(err) => panic!("Could not start logger: {:?}", err)
    };

    let sigaction = signal::SigAction::new(signal::SigHandler::Handler(sigint),
                                           signal::SaFlags::empty(),
                                           signal::SigSet::empty());

    unsafe {
        match signal::sigaction(signal::SIGINT, &sigaction) {
            Ok(_) => {} ,
            Err(err) => {
                error!("Could not register SIGINT handler: {:?}", err);
                process::exit(1);
            }
        };

        match signal::sigaction(signal::SIGTERM, &sigaction) {
            Ok(_) => {} ,
            Err(err) => {
                error!("Could not register SIGTERM handler: {:?}", err);
                process::exit(1);
            }
        };

        match signal::sigaction(signal::SIGQUIT, &sigaction) {
            Ok(_) => {} ,
            Err(err) => {
                error!("Could not register SIGQUIT handler: {:?}", err);
                process::exit(1);
            }
        };
    }

    let default_curve = vec![(41, 20), (49, 30), (57, 45), (66, 55),
                             (75, 63), (78, 72), (80, 80)];

    let mut curve: Vec<(u16, u16)>;

    let conf_file = match BaseDirectories::new() {
        Ok(x) => {
            x.find_config_file(CONF_FILE)
        },
        Err(e) => {
            error!("Could not find xdg conformant dirs: {}", e);
            None
        }
    };

    match conf_file {
        Some(path) => {

            match File::open(path.to_str().unwrap()) {
                Ok(file) => {
                    curve = Vec::new();

                    for raw_line in BufReader::new(file).lines() {
                        let line = raw_line.unwrap();
                        let trimmed = line.trim();
                        if trimmed.starts_with("#") {
                            continue;
                        }

                        let parts = trimmed.split_whitespace()
                                           .collect::<Vec<&str>>();

                        if parts.len() < 2 {
                            debug!("Invalid line, continuing");
                            continue
                        }

                        let x = match parts[0].parse::<u16>() {
                            Ok(val) => val,
                            Err(e) => {
                                debug!("Could not parse value {}: {}, ignoring",
                                       parts[0], e);
                                continue;
                            }
                        };

                        let y = match parts[1].parse::<u16>() {
                            Ok(val) => val,
                            Err(e) => {
                                debug!("Could not parse value {}: {}, ignoring",
                                       parts[1], e);
                                continue;
                            }
                        };

                        curve.push((x, y));
                    }
                },
                Err(e) => {
                    error!("Could not read configuration file {:?}: {}",
                           path, e);
                    curve = default_curve;
                }
            };

        },
        None => {
            curve = default_curve;
        }
    };

    let mut mgr = match NVFanManager::new(curve, force_update, limits) {
        Ok(m) => m,
        Err(s) => {
            error!("{}", s);
            process::exit(1);
        }
    };

    println!("Using NVIDIA driver version {:.2}",
             mgr.ctrl.get_version().parse::<f32>().unwrap());

    let timeout = Duration::new(2, 0);
    RUNNING.store(true, Ordering::Relaxed);

    // Main loop
    loop {
        if RUNNING.load(Ordering::Relaxed) == false {
            debug!("Exiting");
            break;
        }

        mgr.update();

        let graphics_util = match mgr.ctrl.get_utilization().get("graphics") {
            Some(v) => *v,
            None => -1
        };

        debug!("Temp: {}; Speed: {} RPM ({}%); Load: {}%; Mode: {}",
            mgr.ctrl.get_temp(), mgr.ctrl.get_fanspeed_rpm(),
            mgr.ctrl.get_fanspeed(), graphics_util,
            match mgr.ctrl.get_ctrl_status() {
                Ok(NVCtrlFanControlState::Auto) => "Auto",
                Ok(NVCtrlFanControlState::Manual) => "Manual",
                Err(_) => "ERR"});

        thread::sleep(timeout);
    }

}
