#![feature(format_args_capture)]

mod click_mute;
mod clicky_events;
mod delay;
mod fader;
mod level_event;
use crate::level_event::LevelEvent;
use std::thread;

fn main() {
    let exit_flag = LevelEvent::new();
    let click_mute_join = {
        let mut exit_flag = exit_flag.clone();
        thread::spawn(move || {
            click_mute::main(exit_flag.clone());
            exit_flag.activate();
        })
    };
    exit_flag.wait();
    click_mute_join.join().unwrap();
}
