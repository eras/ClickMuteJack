use crate::{clicky_events, delay, fader};
// use crossbeam_channel::bounded;
use std::io;

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

    delay_a: delay::Delay,
    delay_b: delay::Delay,

    fader_a: fader::Fader,
    fader_b: fader::Fader,

    clicky_events: clicky_events::ClickyEvents,

    sample_index: usize,

    mute_t0_index: Option<usize>,
    mute_t1_index: usize,
}

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

    let mut fader_a = fader::new(0.0);
    let mut fader_b = fader::new(0.0);

    fader::fade_in(&mut fader_a, fade_samples);
    fader::fade_in(&mut fader_b, fade_samples);

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
        delay_a: delay::new(delay_samples),
        delay_b: delay::new(delay_samples),

        fader_a,
        fader_b,

        clicky_events: clicky_events::new(),

        sample_index: 0,
        mute_t0_index: None,
        mute_t1_index: 0,
    }
}

fn process(mute: &mut ClickMute, ps: &jack::ProcessScope) -> jack::Control {
    let in_a = mute.in_a.as_slice(ps);
    let in_b = mute.in_b.as_slice(ps);
    let out_a = mute.out_a.as_mut_slice(ps);
    let out_b = mute.out_b.as_mut_slice(ps);

    match clicky_events::when_clicked(&mut mute.clicky_events) {
        None => (),
        Some((t0, t1)) => {
            let mute_wait_seconds = mute.delay_seconds + t0 + mute.mute_offset_seconds;
            if mute.mute_t0_index == None {
                let mute_t0_index = mute.sample_index
                    + f64::max(0.0, mute_wait_seconds * mute.sample_rate as f64) as usize;
                mute.mute_t0_index = Some(mute_t0_index);
            }

            mute.mute_t1_index = mute.sample_index
                + ((mute.delay_seconds + mute.mute_duration_seconds + t1) * mute.sample_rate as f64)
                    as usize;

            // println!(
            //     "set {} {}",
            //     (match mute.mute_t0_index {
            //         Some(index) => index as i64 - mute.sample_index as i64,
            //         None => 0,
            //     }),
            //     mute.mute_t1_index as i64 - mute.sample_index as i64
            // );
            assert!(Some(mute.mute_t1_index) >= mute.mute_t0_index);
        }
    }

    for (((in_a, in_b), out_a), out_b) in (in_a.iter())
        .zip(in_b.iter())
        .zip(out_a.iter_mut())
        .zip(out_b.iter_mut())
    {
        if Some(mute.sample_index) == mute.mute_t0_index {
            println!("fade out at {}", mute.sample_index);
            fader::fade_out(&mut mute.fader_a, mute.fade_samples);
            fader::fade_out(&mut mute.fader_b, mute.fade_samples);
            mute.mute_t0_index = None;
        }

        let a = delay::process(&mut mute.delay_a, *in_a);
        let b = delay::process(&mut mute.delay_b, *in_b);
        let (a, b) = (
            fader::process(&mut mute.fader_a, a),
            fader::process(&mut mute.fader_b, b),
        );
        // let muting =
        //     mute.mute_t0_index <= mute.sample_index && mute.sample_index <= mute.mute_t1_index;
        // let a = if muting { 0.0 } else { a };
        // let b = if muting { 0.0 } else { b };
        *out_a = a;
        *out_b = b;

        if mute.sample_index == mute.mute_t1_index {
            println!("fade in at {}", mute.sample_index);
            fader::fade_in(&mut mute.fader_a, mute.fade_samples);
            fader::fade_in(&mut mute.fader_b, mute.fade_samples);
        }

        mute.sample_index += 1
    }

    jack::Control::Continue
}

pub fn main() {
    let (client, _status) =
        jack::Client::new("click_mute", jack::ClientOptions::NO_START_SERVER).unwrap();

    let mut mute = new(&client);

    // let (tx, rx) = bounded(1_000_000);
    let process = jack::ClosureProcessHandler::new(
        move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
            process(&mut mute, &ps)
        },
    );

    let active_client = client.activate_async((), process).unwrap();

    println!("^D to exit");
    let mut user_input = String::new();
    // ignore result
    let _ = io::stdin().read_line(&mut user_input);

    active_client.deactivate().unwrap();
}
