use crate::level_event::LevelEvent;
use std::collections::HashMap;
use std::ffi::CString;
use std::os::unix::prelude::RawFd;
use std::sync::{Arc, Mutex};
use std::{thread, time};
use thiserror::Error;
extern crate libc;

pub struct ClickyEvents {
    devices: Arc<Mutex<Option<DeviceState>>>,
    reenumerator_join: Option<thread::JoinHandle<()>>,
    reenumerator_stop: LevelEvent,
    epoll_fd: RawFd,
}

fn make_device_mapping(devices: Vec<evdev::Device>) -> HashMap<CString, evdev::Device> {
    let mut mapping = HashMap::new();
    for device in devices.into_iter() {
        match device.physical_path() {
            None => (), // ignore these, we cannot track them
            Some(ref name) => {
                mapping.insert(name.clone(), device);
            }
        }
    }
    mapping
}

struct DeviceState {
    devices: Vec<evdev::Device>,

    // keep a journal of updates for epoll; reset after processing
    added: Vec<RawFd>,
    removed: Vec<RawFd>,
}

impl DeviceState {
    fn new() -> Self {
        DeviceState {
            devices: vec![],
            added: vec![],
            removed: vec![],
        }
    }
}

fn reenumerator_thread(
    clicky_devices: Arc<Mutex<Option<DeviceState>>>,
    reenumerator_stop: LevelEvent,
) {
    let mut first = true;
    while if first {
        true
    } else {
        !reenumerator_stop.wait_timeout(time::Duration::from_millis(20000))
    } {
        let devices = evdev::enumerate();
        let mut key_devices: Vec<evdev::Device> = vec![];

        for device in devices {
            if device.events_supported().contains(evdev::KEY) {
                key_devices.push(device);
            }
        }

        if key_devices.is_empty() && first {
            println!("No devices with key output found; missing permissions to /dev/input?");
        }

        if let Ok(mut write_devices) = clicky_devices.clone().lock() {
            // find new and removed devices
            let device_state = (*write_devices).take().unwrap();
            let mut old = make_device_mapping(device_state.devices);
            let mut new = make_device_mapping(key_devices);
            let mut added_fds = device_state.added;
            let mut removed_fds = device_state.removed;

            let mut new_devices: Vec<evdev::Device> = vec![];

            let mut old_device_keys = vec![];
            // find out which devices we have in common
            for old_key in old.keys() {
                // new set has a device as we know
                if new.contains_key(old_key) {
                    // use the old one to avoid repeating events
                    old_device_keys.push(old_key.clone())
                } else {
                    println!("Device removed: {:?}", old_key);
                    removed_fds.push(old.get(old_key).unwrap().fd());
                }
            }

            let mut new_device_keys = vec![];
            // find out which devices are new
            for new_key in new.keys() {
                // new set has a device not known before
                if !old.contains_key(new_key) {
                    // so add that to the device list
                    println!("Device added: {:?}", new_key);
                    new_device_keys.push(new_key.clone())
                }
            }
            for old_key in old_device_keys {
                new_devices.push(old.remove(&old_key).take().unwrap());
            }
            for new_key in new_device_keys {
                added_fds.push(new.get(&new_key).unwrap().fd());
                new_devices.push(new.remove(&new_key).take().unwrap());
            }

            *write_devices = Some(DeviceState {
                devices: new_devices,
                added: added_fds,
                removed: removed_fds,
            })
        }

        first = false;
    }
}

#[derive(Error, Debug)]
pub enum Error {
    // produced by epoll::create
    #[error(transparent)]
    IOError(#[from] std::io::Error),
}

impl ClickyEvents {
    pub fn new() -> Result<ClickyEvents, Error> {
        let kbd_devices = Arc::new(Mutex::new(Some(DeviceState::new())));

        let reenumerator_stop = LevelEvent::new();

        let reenumerator_join = Option::Some({
            let kbd_devices = kbd_devices.clone();
            let reenumerator_stop = reenumerator_stop.clone();
            thread::spawn(move || reenumerator_thread(kbd_devices, reenumerator_stop))
        });

        let epoll_fd = epoll::create(true)?;

        Ok(ClickyEvents {
            devices: kbd_devices,
            reenumerator_join,
            reenumerator_stop,
            epoll_fd,
        })
    }

    pub fn stop(&mut self) {
        self.reenumerator_stop.activate();
        if let Some(join) = self.reenumerator_join.take() {
            join.join().unwrap();
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
            let mut local_devices = devices.take().unwrap();
            for removed in &local_devices.removed {
                epoll::ctl(
                    self.epoll_fd,
                    epoll::ControlOptions::EPOLL_CTL_DEL,
                    *removed,
                    epoll::Event::new(epoll::Events::EPOLLIN, *removed as u64),
                )
                .expect("epoll::ctl failed when removing fd");
            }
            local_devices.removed.clear();

            for added in &local_devices.added {
                epoll::ctl(
                    self.epoll_fd,
                    epoll::ControlOptions::EPOLL_CTL_ADD,
                    *added,
                    epoll::Event::new(epoll::Events::EPOLLIN, *added as u64),
                )
                .expect("epoll::ctl failed when adding fd");
            }
            local_devices.added.clear();

            let mut fd_events_in =
                vec![epoll::Event::new(epoll::Events::empty(), 0); 2 * local_devices.devices.len()];

            let num_events = if local_devices.devices.len() > 0 {
                epoll::wait(self.epoll_fd, 0, &mut fd_events_in).expect("epoll::wait failed")
            } else {
                0
            };

            let fd_events_out: Vec<_> = fd_events_in.splice(..num_events, vec![]).collect();

            for device in &mut local_devices.devices {
                let device_fd = device.fd();
                if let Some(_) = fd_events_out
                    .iter()
                    .position(|&event| event.data == device_fd as u64)
                {
                    if let Ok(events) = device.events() {
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
                    } else {
                        // actually let's just ignore the error; we will
                        // re-enumerate the devices shortly
                    }
                }
            }
            // put it back 🙄
            *devices = Some(local_devices);
        }
        clicked
    }
}
