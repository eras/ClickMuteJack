use crate::level_event::LevelEvent;
use std::sync::{Arc, Mutex};
use std::{thread, time};
extern crate libc;

pub struct ClickyEvents {
    devices: Arc<Mutex<Vec<evdev::Device>>>,
    reenumerator_join: Option<thread::JoinHandle<()>>,
    reenumerator_stop: LevelEvent,
}

fn reenumerator_thread(
    clicky_devices: Arc<Mutex<Vec<evdev::Device>>>,
    reenumerator_stop: LevelEvent,
) {
    let mut first = true;
    while if first {
        true
    } else {
        !reenumerator_stop.wait_timeout(time::Duration::from_millis(1000))
    } {
        let devices = evdev::enumerate();
        let mut kbd_devices: Vec<evdev::Device> = vec![];

        for device in devices {
            if device.events_supported().contains(evdev::KEY) {
                kbd_devices.push(device);
            }
        }

        if kbd_devices.is_empty() && first {
            println!("No devices with key output found; missing permissions to /dev/input?");
        }

        if let Ok(mut new_devices) = clicky_devices.clone().lock() {
            *new_devices = kbd_devices
        }

        first = false;
    }
}

impl ClickyEvents {
    pub fn new() -> ClickyEvents {
        let kbd_devices = Arc::new(Mutex::new(vec![]));

        let reenumerator_stop = LevelEvent::new();

        let reenumerator_join = Option::Some({
            let kbd_devices = kbd_devices.clone();
            let reenumerator_stop = reenumerator_stop.clone();
            thread::spawn(move || reenumerator_thread(kbd_devices, reenumerator_stop))
        });

        ClickyEvents {
            devices: kbd_devices,
            reenumerator_join,
            reenumerator_stop,
        }
    }

    pub fn stop(&mut self) {
        self.reenumerator_stop.activate();
        match self.reenumerator_join.take() {
            Some(join) => join.join().unwrap(),
            None => {
                // can't stop twice..
            }
        }
    }

    /** If clicked, returns a timespan of two negative numbers indicating in which
     * time window relative to the current time the events occurred (in seconds) */
    pub fn when_clicked(&mut self) -> Option<(f64, f64)> {
        let mut clicked = None;
        let mut time_t1: libc::timespec = unsafe { std::mem::zeroed() };
        unsafe {
            libc::clock_gettime(libc::CLOCK_REALTIME, &mut time_t1);
        };
        if let Ok(mut devices) = self.devices.clone().lock() {
            for device in &mut *devices {
                match device.events() {
                    Ok(events) => {
                        for event in events {
                            if ((1_u32) << event._type) & evdev::KEY.bits() != 0
                                && (event.value == 0 || event.value == 1)
                            {
                                let mut usec_delta = time_t1.tv_nsec / 1000 - event.time.tv_usec;
                                let mut sec_delta = time_t1.tv_sec - event.time.tv_sec;
                                if usec_delta < 0 {
                                    usec_delta += 1000000;
                                    sec_delta -= 1;
                                }
                                let delta = f64::min(
                                    -0.0,
                                    -(sec_delta as f64 + usec_delta as f64 / 1000000.0),
                                );
                                if delta < -0.100 {
                                    // https://github.com/eras/ClickMuteJack/issues/6
                                    println!(
					"Dropped too old event value {} at {}+{} -> delta {} (issue #6)",
					event.value, sec_delta, usec_delta, delta
                                    );
                                } else {
                                    clicked = match clicked {
                                        None => Some((delta, delta)),
                                        Some((oldest, newest)) => {
                                            Some((f64::min(oldest, delta), f64::max(newest, delta)))
                                        }
                                    };
                                }
                            }
                        }
                    }
                    Err(_) => {
                        // actually let's just ignore the error; we will
                        // re-enumerate the devices shortly
                    }
                }
            }
        }
        clicked
    }
}
