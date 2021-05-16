// use evdev;
extern crate libc;

pub struct ClickyEvents {
    devices: Vec<evdev::Device>,
}

impl ClickyEvents {
    pub fn new() -> ClickyEvents {
        let devices = evdev::enumerate();
        let mut kbd_devices: Vec<evdev::Device> = vec![];

        for device in devices {
            if device.events_supported().contains(evdev::KEY) {
                println!("using device {:?}", &device);
                kbd_devices.push(device);
            }
        }

        ClickyEvents {
            devices: kbd_devices,
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
        for device in &mut self.devices {
            // TODO: handle device disappearing gracefully
            let events = device.events().unwrap();
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
                    let delta = f64::min(-0.0, -(sec_delta as f64 + usec_delta as f64 / 1000000.0));

                    clicked = match clicked {
                        None => Some((delta, delta)),
                        Some((oldest, newest)) => {
                            Some((f64::min(oldest, delta), f64::max(newest, delta)))
                        }
                    };
                }
            }
        }
        clicked
    }
}
