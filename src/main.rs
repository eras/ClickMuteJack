mod background_sampler;
mod click_info;
mod click_mute;
mod click_mute_control;
mod clicky_events;
mod config;
mod cross_fader;
mod delay;
mod error;
mod fader;
mod gui;
mod level_event;
mod looper;
mod sampler;
mod save;

use crate::click_info::ClickInfo;
use crate::config::Config;
use crate::level_event::LevelEvent;
use clap::{App, Arg};
use directories::ProjectDirs;
use std::path::Path;
use std::process::exit;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn get_config_file(config_file_arg: Option<&str>) -> Result<String, error::Error> {
    let joined_pathbuf;
    let joined_path;
    // argument overrides all automation
    let config_file: &Path = if let Some(config_file) = config_file_arg {
        Path::new(config_file)
    } else {
        let config_file = Path::new(&config::FILENAME);
        // does the default config filename exist? if so, go with that
        let config_file: &Path = if config_file.exists() {
            config_file
        } else {
            // otherwise, choose the XDG directory if it can be created
            (if let Some(proj_dirs) = ProjectDirs::from("", "Erkki Seppälä", "click_mute") {
                let config_dir = proj_dirs.config_dir();
                if let Ok(()) = std::fs::create_dir_all(config_dir) {
                    // it's fine to set this to a non-existing file; it will be ignored, but
                    // the filename will still be used for saving
                    joined_pathbuf = config_dir.join("click_mute.ini");
                    joined_path = joined_pathbuf.as_path();
                    Some(&joined_path)
                } else {
                    None
                }
            } else {
                None
            })
            .unwrap_or(&config_file)
        };
        config_file
    };
    let config_file = if let Some(path) = config_file.to_str() {
        path
    } else {
        return Err(error::Error::UnsupportedPath(
            "Sorry, unsupported config file path (needs to be legal UTF8)".to_string(),
        ));
    };
    Ok(config_file.to_string())
}

fn main() -> Result<(), error::Error> {
    let args = App::new("click_mute")
        .version(option_env!("GIT_DESCRIBE").unwrap_or_else(|| env!("VERGEN_SEMVER")))
        .author("Erkki Seppälä <erkki.seppala@vincit.fi>")
        .about("Input-device-synchronized microphone muting for Jack/Pipewire")
        .arg(
            Arg::new("config")
                .long("config")
                .short('c')
                .takes_value(true)
                .about("Configuration file to load (and save, if the save function is used)"),
        )
        .get_matches();
    let config_file = get_config_file(args.value_of("config"))?;
    let (send_control, recv_control) = mpsc::channel();
    let config = match Config::load(&config_file) {
        Ok(config) => config,
        Err(config::Error::ParseError(error)) => {
            println!("{}", error);
            exit(1);
        }
        Err(err) => return Err(error::Error::ConfigError(err)),
    };
    let exit_flag = LevelEvent::new();
    let click_info = Arc::new(Mutex::new(ClickInfo::new()));
    let gui_join = {
        let mut exit_flag = exit_flag.clone();
        let click_info = click_info.clone();
        thread::spawn(move || {
            gui::main(
                exit_flag.clone(),
                click_info,
                config,
                config_file,
                send_control,
            );
            exit_flag.activate();
        })
    };
    let click_mute_join = {
        let mut exit_flag = exit_flag.clone();
        thread::spawn(move || {
            let result = click_mute::main(exit_flag.clone(), click_info, config, recv_control);
            exit_flag.activate();
            result
        })
    };
    exit_flag.wait();
    click_mute_join.join().unwrap()?;
    gui_join.join().unwrap();
    Ok(())
}
