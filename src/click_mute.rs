use crate::{clicky_events::ClickyEvents, delay::Delay, fader::Fader};
// use crossbeam_channel::bounded;
use std::io;
use std::sync::{Arc, Mutex};

struct ClickMute {
    in_a: jack::Port<jack::AudioIn>,
    in_b: jack::Port<jack::AudioIn>,
    out_a: jack::Port<jack::AudioOut>,
    out_b: jack::Port<jack::AudioOut>,
    sample_rate: usize,

    delay_seconds: f64,         // how long is the delay buffer
    mute_offset_seconds: f64,   // how long to wait until we start mute
    mute_duration_seconds: f64, // how long will the mute last
    fade_samples: usize,        // how many sample_index will the fade in/fade out last

    delay_a: Delay,
    delay_b: Delay,

    fader_a: Fader,
    fader_b: Fader,

    clicky_events: ClickyEvents,

    sample_index: usize,

    mute_t0_index: Option<usize>,
    mute_t1_index: usize,
}

impl ClickMute {
    fn new(client: &jack::Client) -> ClickMute {
        let in_a = client
            .register_port("in_a", jack::AudioIn::default())
            .unwrap();
        let in_b: jack::Port<jack::AudioIn> = client
            .register_port("in_b", jack::AudioIn::default())
            .unwrap();
        let out_a = client
            .register_port("out_a", jack::AudioOut::default())
            .unwrap();
        let out_b = client
            .register_port("out_b", jack::AudioOut::default())
            .unwrap();

        let mute_offset_seconds = -0.06; // delta from the time we detect an event to until we mute sound (so, negative because we hear it before we get the vent)
        let delay_seconds = -mute_offset_seconds; // size of the delay buffer in seconds; sensibly just as long as the mute_offset is
        let mute_duration_seconds = 0.08; // how long do we mute for?
        let fade_seconds = 0.01; // how long is the fade in/out to avoid pops?

        let sample_rate = client.sample_rate();
        let delay_samples = (delay_seconds * sample_rate as f64) as usize;
        let fade_samples = (fade_seconds * sample_rate as f64) as usize;

        let mut fader_a = Fader::new(0.0);
        let mut fader_b = Fader::new(0.0);

        fader_a.fade_in(fade_samples);
        fader_b.fade_in(fade_samples);

        ClickMute {
            in_a,
            in_b,
            out_a,
            out_b,
            sample_rate,

            delay_seconds,
            mute_offset_seconds,
            mute_duration_seconds,
            fade_samples,
            delay_a: Delay::new(delay_samples),
            delay_b: Delay::new(delay_samples),

            fader_a,
            fader_b,

            clicky_events: ClickyEvents::new(),

            sample_index: 0,
            mute_t0_index: None,
            mute_t1_index: 0,
        }
    }

    fn stop(&mut self) {
        self.clicky_events.stop()
    }

    fn process(&mut self, ps: &jack::ProcessScope) -> jack::Control {
        let in_a = self.in_a.as_slice(ps);
        let in_b = self.in_b.as_slice(ps);
        let out_a = self.out_a.as_mut_slice(ps);
        let out_b = self.out_b.as_mut_slice(ps);

        match self.clicky_events.when_clicked() {
            None => (),
            Some((t0, t1)) => {
                let mute_wait_seconds = self.delay_seconds + t0 + self.mute_offset_seconds;
                if self.mute_t0_index == None {
                    let mute_t0_index = self.sample_index
                        + f64::max(0.0, mute_wait_seconds * self.sample_rate as f64) as usize;
                    self.mute_t0_index = Some(mute_t0_index);
                }

                self.mute_t1_index = self.sample_index
                    + ((self.delay_seconds + self.mute_duration_seconds + t1)
                        * self.sample_rate as f64) as usize;

                // println!(
                //     "set {} {}",
                //     (match self.mute_t0_index {
                //         Some(index) => index as i64 - self.sample_index as i64,
                //         None => 0,
                //     }),
                //     self.mute_t1_index as i64 - self.sample_index as i64
                // );
                assert!(Some(self.mute_t1_index) >= self.mute_t0_index);
            }
        }

        for (((in_a, in_b), out_a), out_b) in (in_a.iter())
            .zip(in_b.iter())
            .zip(out_a.iter_mut())
            .zip(out_b.iter_mut())
        {
            if Some(self.sample_index) == self.mute_t0_index {
                println!("fade out at {}", self.sample_index);
                self.fader_a.fade_out(self.fade_samples);
                self.fader_b.fade_out(self.fade_samples);
                self.mute_t0_index = None;
            }

            let a = self.delay_a.process(*in_a);
            let b = self.delay_b.process(*in_b);
            let (a, b) = (self.fader_a.process(a), self.fader_b.process(b));
            // let muting =
            //     self.mute_t0_index <= self.sample_index && self.sample_index <= self.mute_t1_index;
            // let a = if muting { 0.0 } else { a };
            // let b = if muting { 0.0 } else { b };
            *out_a = a;
            *out_b = b;

            if self.sample_index == self.mute_t1_index {
                println!("fade in at {}", self.sample_index);
                self.fader_a.fade_in(self.fade_samples);
                self.fader_b.fade_in(self.fade_samples);
            }

            self.sample_index += 1
        }

        jack::Control::Continue
    }
}

pub fn main() {
    let (client, _status) =
        jack::Client::new("click_mute", jack::ClientOptions::NO_START_SERVER).unwrap();

    let mute = Arc::new(Mutex::new(Some(ClickMute::new(&client))));

    // let (tx, rx) = bounded(1_000_000);
    let process = jack::ClosureProcessHandler::new({
        let mute = mute.clone();
        move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
            if let Ok(mut x) = mute.lock() {
                match &mut *x {
                    Some(click_mute) => click_mute.process(&ps),
                    None => jack::Control::Quit,
                }
            } else {
                jack::Control::Quit
            }
        }
    });

    let active_client = client.activate_async((), process).unwrap();

    println!("^D to exit");
    let mut user_input = String::new();
    // ignore result
    let _ = io::stdin().read_line(&mut user_input);

    active_client.deactivate().unwrap();

    if let Ok(mut x) = mute.lock() {
        match &mut *x {
            Some(click_mute) => click_mute.stop(),
            None => (),
        }
    };
}
