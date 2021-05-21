use crate::click_info::ClickInfo;
use crate::level_event::LevelEvent;
use egui;
use egui::plot::{Curve, Plot, Value};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use {egui_miniquad as egui_mq, miniquad as mq};

#[derive(PartialEq, Clone)]
enum PlotMode {
    LiveSignal,
    Capture,
    NoView,
}

struct Stage {
    egui_mq: egui_mq::EguiMq,
    quit: LevelEvent,

    // shared data with click_mute
    click_info: Arc<Mutex<ClickInfo>>,

    // plot mode
    plot_mode: PlotMode,
}

impl Stage {
    fn new(ctx: &mut mq::Context, quit: LevelEvent, click_info: Arc<Mutex<ClickInfo>>) -> Self {
        Self {
            egui_mq: egui_mq::EguiMq::new(ctx),
            quit,
            click_info,
            plot_mode: PlotMode::LiveSignal,
        }
    }

    fn ui(&mut self) {
        let plot_mode = &mut self.plot_mode;

        let egui_ctx = self.egui_mq.egui_ctx();

        let click_info = self.click_info.clone();

        egui::CentralPanel::default().show(egui_ctx, |ui| {
            {
                let mut click_info = click_info.lock().unwrap();
                ui.checkbox(&mut click_info.mute_enabled, "Automatic muting enabled");
            }

            ui.separator();

            ui.horizontal(|ui| {
                ui.selectable_value(plot_mode, PlotMode::NoView, "No view");
                ui.selectable_value(plot_mode, PlotMode::LiveSignal, "Live signal");
                ui.selectable_value(plot_mode, PlotMode::Capture, "Capture");

                if *plot_mode == PlotMode::Capture {
                    ui.separator();
                    let mut click_info = click_info.lock().unwrap();
                    let click_sampler = &mut click_info.click_sampler;
                    if ui
                        .selectable_label(click_sampler.is_in_hold(), "Hold")
                        .clicked()
                    {
                        if click_sampler.is_in_hold() {
                            click_sampler
                                .acquire_after(Instant::now() + Duration::from_millis(200));
                            click_sampler.clear();
                        } else {
                            click_sampler.hold();
                        }
                    }
                }
            });

            match *plot_mode {
                PlotMode::NoView => (),
                _ => {
                    let click_info = click_info.lock().unwrap();
                    let (sampler, sample_max_x) = if click_info.click_sampler.is_in_hold()
                        && !click_info.click_sampler.is_empty()
                        && *plot_mode == PlotMode::Capture
                    {
                        (
                            &click_info.click_sampler,
                            Some(click_info.click_sampler.get().len() as f64 / 48000.0),
                        )
                    } else {
                        (&click_info.live_sampler, None)
                    };
                    let samples = sampler.get();
                    let width = ui.available_size().x;
                    let scale =
                        (2 as usize).pow(i32::clamp((width / 200.0).log2() as i32, 0, 2) as u32);
                    // 200 * scale cannot get greater than 1000 or so, or it segfaults in nvidia libraries.
                    let curve = if sample_max_x.is_some() {
                        Curve::from_values_iter(
                            (0..200 * scale)
                                .map(|i| (samples.len() / 200) / scale * i)
                                .map(|i| {
                                    let x = i as f64 / 48000.0;
                                    Value::new(
                                        x as f64,
                                        if i < samples.len() {
                                            samples[i] as f64
                                        } else {
                                            0.0
                                        },
                                    )
                                }),
                        )
                    } else {
                        Curve::from_values_iter((0..200 * scale).map(|i| 20 / scale * i).map(|i| {
                            let x = i as f64 / 48000.0;
                            Value::new(
                                x as f64,
                                if i < samples.len() {
                                    samples[i] as f64
                                } else {
                                    0.0
                                },
                            )
                        }))
                    };
                    ui.add({
                        let plot = Plot::new("Captured audio")
                            .curve(curve)
                            .center_y_axis(true)
                            .width(width)
                            .height(ui.available_size().y);
                        match sample_max_x {
                            Some(max_x) => plot.include_x(0.0).include_x(max_x),
                            None => plot.include_y(-1.0).include_y(1.0),
                        }
                    });
                }
            }
        });
    }
}

impl mq::EventHandler for Stage {
    fn update(&mut self, _ctx: &mut mq::Context) {}

    fn draw(&mut self, ctx: &mut mq::Context) {
        if self.quit.test() {
            ctx.quit();
        } else {
            ctx.clear(Some((1., 1., 1., 1.)), None, None);
            ctx.begin_default_pass(mq::PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
            ctx.end_render_pass();

            self.egui_mq.begin_frame(ctx);
            self.ui();
            self.egui_mq.end_frame(ctx);

            // Draw things behind egui here

            self.egui_mq.draw(ctx);

            // Draw things in front of egui here

            ctx.commit_frame();
        }
    }

    fn mouse_motion_event(&mut self, ctx: &mut mq::Context, x: f32, y: f32) {
        self.egui_mq.mouse_motion_event(ctx, x, y);
    }

    fn mouse_wheel_event(&mut self, ctx: &mut mq::Context, dx: f32, dy: f32) {
        self.egui_mq.mouse_wheel_event(ctx, dx, dy);
    }

    fn mouse_button_down_event(
        &mut self,
        ctx: &mut mq::Context,
        mb: mq::MouseButton,
        x: f32,
        y: f32,
    ) {
        self.egui_mq.mouse_button_down_event(ctx, mb, x, y);
    }

    fn mouse_button_up_event(
        &mut self,
        ctx: &mut mq::Context,
        mb: mq::MouseButton,
        x: f32,
        y: f32,
    ) {
        self.egui_mq.mouse_button_up_event(ctx, mb, x, y);
    }

    fn char_event(
        &mut self,
        _ctx: &mut mq::Context,
        character: char,
        _keymods: mq::KeyMods,
        _repeat: bool,
    ) {
        self.egui_mq.char_event(character);
    }

    fn key_down_event(
        &mut self,
        ctx: &mut mq::Context,
        keycode: mq::KeyCode,
        keymods: mq::KeyMods,
        _repeat: bool,
    ) {
        self.egui_mq.key_down_event(ctx, keycode, keymods);
    }

    fn key_up_event(&mut self, _ctx: &mut mq::Context, keycode: mq::KeyCode, keymods: mq::KeyMods) {
        self.egui_mq.key_up_event(keycode, keymods);
    }
}

pub fn main(quit: LevelEvent, click_info: Arc<Mutex<ClickInfo>>) {
    let conf = mq::conf::Conf {
        // high_dpi: true,
        window_height: 200,
        window_width: 600,
        ..Default::default()
    };
    mq::start(conf, |mut ctx| {
        mq::UserData::owning(Stage::new(&mut ctx, quit, click_info), ctx)
    });
}
