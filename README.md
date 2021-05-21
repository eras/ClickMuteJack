# Click Muter for Jack

Licensed under the [MIT license](LICENSE.MIT).

![Screenshot of ClickMuteJack with Jack Qjackctl and its patchbay](doc/screenshot.png)

A tool for helping with teleconferencing, when you have loud input
devices. Such as a mechanical keyboard..

It works by inputting a jack stream from microphone, and then directly
connecting all key-producing devices from /dev/input. Then when an
input arrives, its timing is checked and muting is applied for the
pre-configured duration. Finally the module outputs a jack stream that
can be used in place of the microphone.

To work this of course needs a short delay. For me a 60-millisecond
buffer is enough, and Pipewire is configured with
`PIPEWIRE_LATENCY=128/48000`.

## Installation

1) Install the Rust compiler with Cargo e.g. with https://rustup.rs/

2) `cargo install --git https://github.com/eras/ClickMuteJack`

3) `$HOME/.cargo/bin/click_mute` has now been installed

## Setting it up with Pipewire

1) create a source that is visible in the PulseAudio side:

```
pactl load-module module-null-sink                   \
                  media.class=Audio/Source/Virtual   \
                  sink_name=my-source                \
                  channel_map=front-left,front-right
```

2) Set up the `qjackctl` patchbay so that `click_mute` is connected
to `my-source` input and your microphone is connected to `click_mute`
input and activate the patchbay.

3) Set `my-source` as the default input in your audio control tool, such as
`pavucontrol`

4) Maybe restart your browser to re-enumrate audio devices; it seems
the `pactl` command does not cause a plug-in event to make it happen
automatically.

5) Boom, you can use https://online-voice-recorder.com/ to test if it
works.

If you just want to test (even more) locally, instead connect the
`click_mute` output directly to your speakers. Be aware that this can
cause an audio loop.

Results not satisfactory? Adjust the parameters `mute_offset_seconds`
and `mute_duration_seconds` in [`click_mute.rs`](src/click_mute.rs).

Erkki Seppälä <erkki.seppala@vincit.fi>
