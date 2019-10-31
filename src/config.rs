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
    #[serde(rename = "gpu")]
    gpus: Vec<T>,
}

fn true_() -> bool { true }

#[derive(Debug, Deserialize)]
pub struct TomlConf {
    #[serde(default)]
    id: u32,
    #[serde(default = "true_")]
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
        Err(e) => {
            // Toml parsing failed; try legacy config instead
            if might_be_legacy_string(conf) {
                from_legacy_string(conf)
            } else {
                Err(format!("config parsing failed: {}", e))
            }
        }
    }
}

#[test]
fn test_valid_toml_from_string() {
    let cfg = from_string(&"[[gpu]]
                            id = 0
                            enabled = true
                            points = [[1, 2], [3, 4], [5, 6]]

                            [[gpu]]
                            id = 1
                            enabled = false
                            points = [[6, 7], [8, 9]]");

    assert!(cfg.is_ok());

    let cfg = cfg.unwrap();

    if let Config::Toml(cfg) = cfg {
        assert!(cfg.gpus.len() == 2);
        let g0 = &cfg.gpus[0];
        assert_eq!(g0.id, 0);
        assert_eq!(g0.enabled, true);
        assert_eq!(g0.points, vec![(1, 2), (3, 4), (5, 6)]);

        let g1 = &cfg.gpus[1];
        assert_eq!(g1.id, 1);
        assert_eq!(g1.enabled, false);
        assert_eq!(g1.points, vec![(6, 7), (8, 9)]);
    } else {
        assert!(false, "Not a Config::Toml(..) enum value");
    }
}

#[test]
fn test_defaults_with_toml_from_string() {
    let cfg = from_string(&"[[gpu]]\npoints = [[11, 22], [33, 44]]");

    assert!(cfg.is_ok());

    let cfg = cfg.unwrap();

    if let Config::Toml(cfg) = cfg {
        assert!(cfg.gpus.len() == 1);
        let g0 = &cfg.gpus[0];
        assert_eq!(g0.id, 0);
        assert_eq!(g0.enabled, true);
        assert_eq!(g0.points, vec![(11, 22), (33, 44)]);
    } else {
        assert!(false, "Not a Config::Toml(..) enum value");
    }
}


#[test]
fn test_invalid_toml_from_string() {
    let cfg = from_string(&"[[gpu]]\npoints = [[2, foobar]]");

    assert!(cfg.is_err());

    if let Err(msg) = cfg {
        assert!(msg.find("invalid number").is_some());
    } else {
        assert!(false, "parsing should have failed");
    }
}

#[test]
fn test_invalid_legacy_from_string() {
    let cfg = from_string(&"2 3 ]]");

    assert!(cfg.is_err());

    if let Err(msg) = cfg {
        assert!(msg == "At least two points are required for the curve");
    } else {
        assert!(false, "parsing should have failed");
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

fn might_be_legacy_string(conf: &str) -> bool {
    for line in conf.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed.find("[gpu]").is_some() || trimmed.starts_with("points") {
            return false;
        }
    }
    true
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
        Err("At least two points are required for the curve".to_string())
    } else {
        Ok(Config::Legacy(GpuConfig {
            gpus: vec![LegacyConf { points: curve }],
        }))
    }
}
