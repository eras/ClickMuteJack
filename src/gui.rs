use crate::click_info::ClickInfo;
use crate::click_mute_control;
use crate::config::Config;
use crate::level_event::LevelEvent;
use egui::plot::{Curve, Plot, Value};
use std::ops::RangeInclusive;
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

    config: Config,
    control: click_mute_control::Sender,

    origo_at_click: bool,
    autoscale_y: bool,

    config_file: String,
}

impl Stage {
    fn new(
        ctx: &mut mq::Context,
        quit: LevelEvent,
        click_info: Arc<Mutex<ClickInfo>>,
        config: Config,
        config_file: String,
        control: click_mute_control::Sender,
    ) -> Self {
        Self {
            egui_mq: egui_mq::EguiMq::new(ctx),
            quit,
            click_info,
            plot_mode: PlotMode::LiveSignal,
            config,
            config_file,
            control,
            origo_at_click: false,
            autoscale_y: true,
        }
    }

    fn sample_min_max<I: Iterator<Item = usize>>(
        indices: I,
        samples: Vec<f32>,
        time_offset: f64,
    ) -> (Vec<egui::plot::Value>, Vec<egui::plot::Value>) {
        if samples.is_empty() {
            (vec![], vec![])
        } else {
            let (values_min, values_max, _) = indices.fold(
                (Vec::with_capacity(1024), Vec::with_capacity(1024), None),
                |(mut values_min, mut values_max, prev), i| {
                    let x = i as f64 / 48000.0 + time_offset;
                    if let Some(prev) = prev {
                        // TODO: don't require 0.0 to be min/max. buuut it doesn't matter because zoom is disabled, and
                        // this can rarely happen if data is zoomed out.
                        let minmax = samples
                            [usize::min(samples.len() - 1, prev)..usize::min(samples.len(), i)]
                            .iter()
                            .fold((0.0f32, 0.0f32), |(min, max), b| (min.min(*b), max.max(*b)));
                        values_min.push(Value::new(x as f64, minmax.0));
                        values_max.push(Value::new(x as f64, minmax.1));
                    }
                    (values_min, values_max, Some(i))
                },
            );
            (values_min, values_max)
        }
    }

    fn ui(&mut self) {
        let plot_mode = &mut self.plot_mode;
        let old_config = self.config;
        let config = &mut self.config;
        let control = &mut self.control;
        let origo_at_click = &mut self.origo_at_click;
        let autoscale_y = &mut self.autoscale_y;
        let config_file = &self.config_file;

        let egui_ctx = self.egui_mq.egui_ctx();

        let click_info = self.click_info.clone();

        egui::CentralPanel::default().show(egui_ctx, |ui| {
            ui.horizontal(|ui| {
                ui.columns(5, |columns| {
                    let mut click_info = click_info.lock().unwrap();
                    columns[0].checkbox(&mut click_info.mute_enabled, "Automatic\nmuting");
                    columns[1].checkbox(&mut click_info.invert_mute, "Invert\nmuting");
                    if click_info.invert_mute {
                        click_info.background_noise = false;
                    }
                    columns[2].checkbox(&mut click_info.background_noise, "Background\nnoise");
                    if columns[3]
                        .add_sized((0.0, 40.0), egui::Button::new("Save"))
                        .clicked()
                    {
                        // TODO: move to separate thread to not hang the GUI thread during the save
                        match config.save(&config_file) {
                            Ok(()) => (),
                            Err(error) => {
                                // TODO: better error reporting
                                println!("Failed to save config: {:?}", error);
                            }
                        }
                    }
                    columns[4].with_layout(egui::Layout::right_to_left(), |ui| {
                        ui.label(format!("#{}", click_info.num_clicks))
                    });
                });
            });

            ui.separator();

            ui.columns(3, |columns| {
                let slider = |column: &mut egui::Ui,
                              label: &str,
                              variable: &mut f64,
                              range: RangeInclusive<f64>| {
                    column.vertical(|ui| {
                        ui.with_layout(
                            egui::Layout::from_main_dir_and_cross_align(
                                egui::Direction::TopDown,
                                egui::Align::Center,
                            ),
                            |ui| ui.label(label),
                        );
                        ui.add(
                            egui::Slider::new(variable, range)
                                .text("s")
                                .fixed_decimals(3),
                        );
                    });
                };
                slider(
                    &mut columns[0],
                    "Mute offset",
                    &mut config.delays.mute_offset,
                    -0.2..=0.1,
                );
                slider(
                    &mut columns[1],
                    "Mute duration",
                    &mut config.delays.mute_duration,
                    0.0..=1.0,
                );
                slider(
                    &mut columns[2],
                    "Fade time",
                    &mut config.delays.fade,
                    0.0..=0.2,
                );
            });
            if *config != old_config {
                control
                    .send(click_mute_control::Message::UpdateConfig(*config))
                    .unwrap();
            }

            ui.separator();

            ui.horizontal(|ui| {
                ui.selectable_value(plot_mode, PlotMode::NoView, "No view");
                ui.selectable_value(plot_mode, PlotMode::LiveSignal, "Live signal");
                ui.selectable_value(plot_mode, PlotMode::Capture, "Capture");

                match *plot_mode {
                    PlotMode::LiveSignal => {
                        let mut click_info = click_info.lock().unwrap();
                        let click_sampler = &mut click_info.click_sampler;
                        click_sampler.live();
                    }
                    PlotMode::Capture => {
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

                        if ui
                            .selectable_label(click_sampler.is_in_auto(), "Auto")
                            .clicked()
                        {
                            if click_sampler.is_in_auto() {
                                click_sampler.hold();
                            } else {
                                click_sampler.auto();
                            }
                        }

                        if ui
                            .selectable_label(*origo_at_click, "Origo at click")
                            .clicked()
                        {
                            *origo_at_click = !*origo_at_click;
                        }
                    }
                    PlotMode::NoView => (),
                }
                if ui
                    .selectable_label(*autoscale_y, "Autoscale Y axis")
                    .clicked()
                {
                    *autoscale_y = !*autoscale_y;
                }
            });

            let click_info = click_info.lock().unwrap();
            match *plot_mode {
                PlotMode::NoView => (),
                _ if !click_info.click_sampler.is_in_auto_hold()
                    && click_info.click_sampler.is_in_auto() => {}
                _ => {
                    let use_captured = (click_info.click_sampler.is_in_hold()
                        | click_info.click_sampler.is_in_auto_hold())
                        && !click_info.click_sampler.is_empty()
                        && *plot_mode == PlotMode::Capture;
                    let sampler = if use_captured {
                        &click_info.click_sampler
                    } else {
                        &click_info.live_sampler
                    };
                    let samples = sampler.get();
                    let width = ui.available_size().x;
                    let scale = 2_usize.pow(i32::clamp((width / 200.0).log2() as i32, 0, 1) as u32);
                    // 200 * scale * number fo curves cannot get greater than 1000 or so, or it segfaults in nvidia
                    // libraries.
                    let (values_min, values_max) = if use_captured {
                        let samples_len = samples.len();
                        Self::sample_min_max(
                            (0..200 * scale).map(|i| (samples_len / 200) / scale * i),
                            samples,
                            if *origo_at_click {
                                config.delays.mute_offset
                            } else {
                                0.0
                            },
                        )
                    } else {
                        Self::sample_min_max((0..200 * scale).map(|i| 20 / scale * i), samples, 0.0)
                    };
                    if !values_min.is_empty() {
                        let min_x = values_min[0].x;
                        let max_x = values_min[values_min.len() - 1].x;
                        let curve_min = Curve::from_values(values_min);
                        let curve_max = Curve::from_values(values_max);
                        ui.add({
                            let plot = Plot::new("Captured audio")
                                .allow_zoom(false)
                                .allow_drag(false)
                                .curve(curve_min.color(egui::Rgba::from_rgb(0.2, 0.2, 0.2)))
                                .curve(curve_max.color(egui::Rgba::from_rgb(0.2, 0.2, 0.2)))
                                .center_y_axis(true)
                                .width(width)
                                .height(ui.available_size().y)
                                .include_x(min_x)
                                .include_x(max_x);
                            if !*autoscale_y {
                                plot.include_y(-1.0).include_y(1.0)
                            } else {
                                plot
                            }
                        });
                    }
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

pub fn main(
    quit: LevelEvent,
    click_info: Arc<Mutex<ClickInfo>>,
    config: Config,
    config_file: String,
    control: click_mute_control::Sender,
) {
    let conf = mq::conf::Conf {
        window_title: String::from("Click Mute"),
        // high_dpi: true,
        window_height: 300,
        window_width: 600,
        ..Default::default()
    };
    mq::start(conf, move |mut ctx| {
        mq::UserData::owning(
            Stage::new(&mut ctx, quit, click_info, config, config_file, control),
            ctx,
        )
    });
}
