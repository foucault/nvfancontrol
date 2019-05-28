use std::fs;
use std::io::prelude::*;
use std::path::PathBuf;

pub trait Curve {
    fn points(&self, id: usize) -> &Vec<(u16, u16)>;
    fn enabled(&self, id: usize) -> bool;
}

#[derive(Debug, Deserialize)]
pub enum Config {
    Toml(GpuConfig<TomlConf>),
    Legacy(GpuConfig<LegacyConf>),
}

#[derive(Debug, Deserialize)]
pub struct GpuConfig<T> {
    gpus: Vec<T>,
}

#[derive(Debug, Deserialize)]
pub struct TomlConf {
    id: u32,
    enabled: bool,
    points: Vec<(u16, u16)>,
}

#[derive(Debug, Deserialize)]
pub struct LegacyConf {
    points: Vec<(u16, u16)>,
}

impl Curve for Config {
    fn points(&self, id: usize) -> &Vec<(u16, u16)> {
        match self {
            Config::Toml(conf) => &conf.gpus[id].points,
            Config::Legacy(conf) => &conf.gpus[0].points,
        }
    }

    fn enabled(&self, id: usize) -> bool {
        match self {
            Config::Toml(conf) => conf.gpus[id].enabled,
            Config::Legacy(_) => true,
        }
    }
}

pub fn from_string(conf: &str) -> Result<Config, String> {
    match toml::from_str::<GpuConfig<TomlConf>>(conf) {
        Ok(c) => Ok(Config::Toml(c)),
        Err(_) => {
            // Toml parsing failed; try legacy config instead
            from_legacy_string(conf)
        }
    }
}

pub fn from_file(path: PathBuf) -> Result<Config, String> {
    match fs::File::open(path.to_str().unwrap()) {
        Ok(mut file) => {
            let mut contents = String::new();
            file.read_to_string(&mut contents).unwrap();
            from_string(&contents)
        }
        Err(e) => Err(format!("Could not open file: {}", e)),
    }
}

fn from_legacy_string(conf: &str) -> Result<Config, String> {
    let mut curve: Vec<(u16, u16)>;

    curve = Vec::new();

    for line in conf.lines() {
        //let line = raw_line.unwrap();
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }

        let parts = trimmed.split_whitespace().collect::<Vec<&str>>();

        if parts.len() < 2 {
            //println!("Invalid line \"{}\", ignoring", line);
            continue;
        }

        let x = match parts[0].parse::<u16>() {
            Ok(val) => val,
            Err(_) => {
                //println!("Could not parse value {}: {}, ignoring", parts[0], e);
                continue;
            }
        };

        let y = match parts[1].parse::<u16>() {
            Ok(val) => val,
            Err(_) => {
                //println!("Could not parse value {}: {}, ignoring", parts[1], e);
                continue;
            }
        };

        curve.push((x, y));
    }
    if curve.len() < 2 {
        Err(String::from(
            "At least two points are required for \
             the curve",
        ))
    } else {
        Ok(Config::Legacy(GpuConfig {
            gpus: vec![LegacyConf { points: curve }],
        }))
    }
}
