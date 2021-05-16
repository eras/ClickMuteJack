# Click Muter for Jack

Licensed under the [MIT license](LICENSE.MIT).

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

To setup the streams properly, well, I haven't tried yet with
teleconferencing use yet, but pipewire tools hopefully will be able to
pull this off.. For testing I've used qjackctl's patchbay.

Erkki Seppälä <erkki.seppala@vincit.fi>
