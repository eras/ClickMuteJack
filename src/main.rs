mod click_info;
mod click_mute;
mod clicky_events;
mod delay;
mod fader;
mod gui;
mod level_event;
mod sampler;

use crate::click_info::ClickInfo;
use crate::level_event::LevelEvent;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let exit_flag = LevelEvent::new();
    let click_info = Arc::new(Mutex::new(ClickInfo::new()));
    let gui_join = {
        let mut exit_flag = exit_flag.clone();
        let click_info = click_info.clone();
        thread::spawn(move || {
            gui::main(exit_flag.clone(), click_info);
            exit_flag.activate();
        })
    };
    let click_mute_join = {
        let mut exit_flag = exit_flag.clone();
        let click_info = click_info.clone();
        thread::spawn(move || {
            click_mute::main(exit_flag.clone(), click_info);
            exit_flag.activate();
        })
    };
    exit_flag.wait();
    click_mute_join.join().unwrap();
    gui_join.join().unwrap();
}
