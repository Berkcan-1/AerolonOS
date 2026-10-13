extern crate alloc;

use alloc::string::String;
use nc_app::Icon;

#[derive(Clone, Debug)]
pub struct PackageManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub icon: Icon,
    pub width_units: i32,
    pub height_units: i32,
}

impl PackageManifest {
    pub fn parse(text: &str) -> Option<Self> {
        let mut id = String::new();
        let mut name = String::new();
        let mut version = String::from("1.0.0");
        let mut icon = Icon::Generic;
        let mut width_units = 32;
        let mut height_units = 20;

        for line in text.lines() {
            let line = line.trim();
            if let Some((key, val)) = line.split_once('=') {
                let k = key.trim();
                let v = val.trim().trim_matches('"');
                match k {
                    "id" => id = String::from(v),
                    "name" => name = String::from(v),
                    "version" => version = String::from(v),
                    "icon" => {
                        icon = match v {
                            "sliders" => Icon::Sliders,
                            "monitor" => Icon::Monitor,
                            "stopwatch" => Icon::Stopwatch,
                            _ => Icon::Generic,
                        };
                    }
                    "width_units" => {
                        if let Ok(w) = v.parse::<i32>() {
                            width_units = w;
                        }
                    }
                    "height_units" => {
                        if let Ok(h) = v.parse::<i32>() {
                            height_units = h;
                        }
                    }
                    _ => {}
                }
            }
        }

        if id.is_empty() || name.is_empty() {
            return None;
        }

        Some(PackageManifest {
            id,
            name,
            version,
            icon,
            width_units,
            height_units,
        })
    }
}
