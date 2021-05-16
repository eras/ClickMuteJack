#![feature(format_args_capture)]

mod click_mute;
mod clicky_events;
mod delay;
mod fader;

fn main() {
    let _foo = clicky_events::new();
    click_mute::main();
}
