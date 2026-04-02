use std::collections::VecDeque;

use iced::widget::{button, canvas, checkbox, column, row, scrollable, slider, text, Canvas};
use iced::{Alignment, Color, Element, Length, Pixels, Point, Rectangle};

use crate::app::{
    GraphMode, GraphSample, Message, NevcApp,
    GRAPH_CHANNEL_NAMES, GRAPH_CHANNEL_UNIT_GROUP, GRAPH_CHANNEL_UNITS,
    NUM_CHANNELS, UNIT_GROUP_NAMES,
};
use crate::serial::ConnectionState;

// ---------------------------------------------------------------------------
// Channel colour palette (8 channels)
// ---------------------------------------------------------------------------

const PALETTE: [Color; NUM_CHANNELS] = [
    Color { r: 0.28, g: 0.73, b: 0.96, a: 1.0 }, // Speed             – sky blue
    Color { r: 0.96, g: 0.68, b: 0.26, a: 1.0 }, // System Current    – amber
    Color { r: 0.40, g: 0.86, b: 0.47, a: 1.0 }, // Phase U Current   – green
    Color { r: 0.96, g: 0.41, b: 0.58, a: 1.0 }, // Phase V Current   – rose
    Color { r: 0.68, g: 0.52, b: 0.96, a: 1.0 }, // Phase W Current   – violet
    Color { r: 0.96, g: 0.96, b: 0.40, a: 1.0 }, // Duty Cycle        – yellow
    Color { r: 0.96, g: 0.55, b: 0.33, a: 1.0 }, // System Voltage    – orange
    Color { r: 0.80, g: 0.85, b: 0.90, a: 1.0 }, // System Power      – silver
];

// One colour per unit group (RPM, A, %, V, W)
const GROUP_PALETTE: [Color; 5] = [
    Color { r: 0.28, g: 0.73, b: 0.96, a: 1.0 },
    Color { r: 0.96, g: 0.68, b: 0.26, a: 1.0 },
    Color { r: 0.96, g: 0.96, b: 0.40, a: 1.0 },
    Color { r: 0.96, g: 0.55, b: 0.33, a: 1.0 },
    Color { r: 0.80, g: 0.85, b: 0.90, a: 1.0 },
];

const BG: Color = Color { r: 0.07, g: 0.07, b: 0.09, a: 1.0 };

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute per-unit-group (min, max) across all selected channels.
fn unit_group_ranges(
    history: &VecDeque<GraphSample>,
    channels: &[bool; NUM_CHANNELS],
) -> [(f32, f32); 5] {
    let mut ranges = [(f32::INFINITY, f32::NEG_INFINITY); 5];
    for sample in history {
        for ch in 0..NUM_CHANNELS {
            if !channels[ch] { continue; }
            if let Some(v) = sample.values[ch] {
                let g = GRAPH_CHANNEL_UNIT_GROUP[ch];
                if v < ranges[g].0 { ranges[g].0 = v; }
                if v > ranges[g].1 { ranges[g].1 = v; }
            }
        }
    }
    ranges
}

fn normalize_ch(v: f32, ch: usize, ranges: &[(f32, f32); 5]) -> f32 {
    let (mn, mx) = ranges[GRAPH_CHANNEL_UNIT_GROUP[ch]];
    if !mn.is_finite() || !mx.is_finite() || (mx - mn).abs() < 1e-9 { return 0.5; }
    (v - mn) / (mx - mn)
}

fn nice_interval(range: f32) -> f32 {
    if range <= 0.0 { return 1.0; }
    let raw = range / 5.0;
    let mag = raw.log10().floor();
    let p = 10_f32.powf(mag);
    let f = raw / p;
    let nice = if f < 1.5 { 1.0 } else if f < 3.5 { 2.0 } else if f < 7.5 { 5.0 } else { 10.0 };
    nice * p
}

fn fmt_val(v: f32) -> String {
    if !v.is_finite() { return String::from("?"); }
    if v.abs() >= 1000.0 { format!("{:.0}", v) }
    else if v.abs() >= 10.0 { format!("{:.1}", v) }
    else { format!("{:.2}", v) }
}

fn ct(
    content: impl Into<String>,
    pos: Point,
    color: Color,
    size: f32,
    ha: iced::alignment::Horizontal,
    va: iced::alignment::Vertical,
) -> canvas::Text {
    canvas::Text {
        content: content.into(),
        position: pos,
        color,
        size: Pixels(size),
        line_height: iced::widget::text::LineHeight::Relative(1.0),
        font: iced::Font::default(),
        horizontal_alignment: ha,
        vertical_alignment: va,
        shaping: iced::widget::text::Shaping::Basic,
    }
}

// ---------------------------------------------------------------------------
// Overlay canvas (all selected channels in one plot, per-unit-group scaling)
// ---------------------------------------------------------------------------

struct OverlayCanvas<'a> {
    history: &'a VecDeque<GraphSample>,
    channels: &'a [bool; NUM_CHANNELS],
    ranges: [(f32, f32); 5],
}

impl<'a> canvas::Program<Message> for OverlayCanvas<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        const ML: f32 = 8.0; const MR: f32 = 8.0;
        const MT: f32 = 10.0; const MB: f32 = 26.0;
        let (pw, ph) = (w - ML - MR, h - MT - MB);

        frame.fill_rectangle(Point::ORIGIN, bounds.size(), BG);

        // Horizontal grid lines
        let gc = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.07 };
        for i in 1u8..4 {
            let y = MT + ph * (i as f32 / 4.0);
            let line = canvas::Path::new(|b| { b.move_to(Point::new(ML, y)); b.line_to(Point::new(ML + pw, y)); });
            frame.stroke(&line, canvas::Stroke::default().with_color(gc).with_width(1.0));
        }

        let n = self.history.len();
        if n < 2 { return vec![frame.into_geometry()]; }

        let t_min = self.history.front().map_or(0.0, |s| s.t);
        let t_max = self.history.back().map_or(1.0, |s| s.t);
        let t_range = if (t_max - t_min).abs() < 1e-6 { 1.0 } else { t_max - t_min };

        // Time-axis ticks
        let tc = Color { r: 0.55, g: 0.55, b: 0.55, a: 0.8 };
        let dt = nice_interval(t_range);
        let mut t = (t_min / dt).ceil() * dt;
        while t <= t_max + 1e-6 {
            let x = ML + (t - t_min) / t_range * pw;
            let tick = canvas::Path::new(|b| {
                b.move_to(Point::new(x, MT + ph));
                b.line_to(Point::new(x, MT + ph + 4.0));
            });
            frame.stroke(&tick, canvas::Stroke::default().with_color(tc).with_width(1.0));
            frame.fill_text(ct(format!("{:.1}s", t), Point::new(x, MT + ph + 6.0),
                tc, 10.0, iced::alignment::Horizontal::Center, iced::alignment::Vertical::Top));
            t += dt;
        }

        // Channel polylines
        for ch in 0..NUM_CHANNELS {
            if !self.channels[ch] { continue; }
            let g = GRAPH_CHANNEL_UNIT_GROUP[ch];
            let (mn, mx) = self.ranges[g];
            if !mn.is_finite() || !mx.is_finite() { continue; }

            let path = canvas::Path::new(|b| {
                let mut first = true;
                for sample in self.history.iter() {
                    let Some(v) = sample.values[ch] else { first = true; continue; };
                    let x = ML + (sample.t - t_min) / t_range * pw;
                    let y = MT + (1.0 - normalize_ch(v, ch, &self.ranges)) * ph;
                    if first { b.move_to(Point::new(x, y)); first = false; }
                    else { b.line_to(Point::new(x, y)); }
                }
            });
            frame.stroke(&path, canvas::Stroke::default().with_color(PALETTE[ch]).with_width(1.5));
        }

        // Unit-group range labels — right-aligned, stacked in top-right corner
        let active_groups: Vec<usize> = (0..5)
            .filter(|&g| (0..NUM_CHANNELS).any(|ch| self.channels[ch] && GRAPH_CHANNEL_UNIT_GROUP[ch] == g))
            .collect();
        let rx = ML + pw - 4.0;
        for (i, g) in active_groups.iter().enumerate() {
            let (mn, mx) = self.ranges[*g];
            if !mn.is_finite() || !mx.is_finite() { continue; }
            let label = format!("{}\u{2013}{} {}", fmt_val(mn), fmt_val(mx), UNIT_GROUP_NAMES[*g]);
            frame.fill_text(ct(label, Point::new(rx, MT + 4.0 + i as f32 * 14.0),
                GROUP_PALETTE[*g], 10.0,
                iced::alignment::Horizontal::Right, iced::alignment::Vertical::Top));
        }

        vec![frame.into_geometry()]
    }
}

// ---------------------------------------------------------------------------
// Single-channel canvas (individual mode)
// ---------------------------------------------------------------------------

struct SingleChannelCanvas<'a> {
    history: &'a VecDeque<GraphSample>,
    channel: usize,
    ymin: f32,
    ymax: f32,
}

impl<'a> canvas::Program<Message> for SingleChannelCanvas<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        const ML: f32 = 44.0; const MR: f32 = 6.0;
        const MT: f32 = 6.0;  const MB: f32 = 24.0;
        let (pw, ph) = (w - ML - MR, h - MT - MB);
        let ch = self.channel;
        let color = PALETTE[ch];

        frame.fill_rectangle(Point::ORIGIN, bounds.size(), BG);

        let gc = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.07 };
        for i in 1u8..4 {
            let y = MT + ph * (i as f32 / 4.0);
            let line = canvas::Path::new(|b| { b.move_to(Point::new(ML, y)); b.line_to(Point::new(ML + pw, y)); });
            frame.stroke(&line, canvas::Stroke::default().with_color(gc).with_width(1.0));
        }

        let n = self.history.len();
        if n < 2 { return vec![frame.into_geometry()]; }

        let t_min = self.history.front().map_or(0.0, |s| s.t);
        let t_max = self.history.back().map_or(1.0, |s| s.t);
        let t_range = if (t_max - t_min).abs() < 1e-6 { 1.0 } else { t_max - t_min };
        let y_range = if (self.ymax - self.ymin).abs() < 1e-9 { 1.0 } else { self.ymax - self.ymin };

        // Y-axis labels
        let lc = Color { r: 0.65, g: 0.65, b: 0.65, a: 0.9 };
        frame.fill_text(ct(fmt_val(self.ymax), Point::new(ML - 3.0, MT),
            lc, 9.0, iced::alignment::Horizontal::Right, iced::alignment::Vertical::Top));
        frame.fill_text(ct(fmt_val((self.ymax + self.ymin) / 2.0), Point::new(ML - 3.0, MT + ph * 0.5),
            lc, 9.0, iced::alignment::Horizontal::Right, iced::alignment::Vertical::Center));
        frame.fill_text(ct(fmt_val(self.ymin), Point::new(ML - 3.0, MT + ph),
            lc, 9.0, iced::alignment::Horizontal::Right, iced::alignment::Vertical::Bottom));
        // Unit
        frame.fill_text(ct(GRAPH_CHANNEL_UNITS[ch].to_string(), Point::new(2.0, MT + ph * 0.5),
            color, 9.0, iced::alignment::Horizontal::Left, iced::alignment::Vertical::Center));

        // X-axis ticks
        let tc = Color { r: 0.55, g: 0.55, b: 0.55, a: 0.8 };
        let dt = nice_interval(t_range);
        let mut t = (t_min / dt).ceil() * dt;
        while t <= t_max + 1e-6 {
            let x = ML + (t - t_min) / t_range * pw;
            let tick = canvas::Path::new(|b| { b.move_to(Point::new(x, MT + ph)); b.line_to(Point::new(x, MT + ph + 4.0)); });
            frame.stroke(&tick, canvas::Stroke::default().with_color(tc).with_width(1.0));
            frame.fill_text(ct(format!("{:.1}s", t), Point::new(x, MT + ph + 6.0),
                tc, 10.0, iced::alignment::Horizontal::Center, iced::alignment::Vertical::Top));
            t += dt;
        }

        // Polyline
        let path = canvas::Path::new(|b| {
            let mut first = true;
            for sample in self.history.iter() {
                let Some(v) = sample.values[ch] else { first = true; continue; };
                let x = ML + (sample.t - t_min) / t_range * pw;
                let y = MT + (1.0 - (v - self.ymin) / y_range) * ph;
                if first { b.move_to(Point::new(x, y)); first = false; }
                else { b.line_to(Point::new(x, y)); }
            }
        });
        frame.stroke(&path, canvas::Stroke::default().with_color(color).with_width(1.5));

        vec![frame.into_geometry()]
    }
}

// ---------------------------------------------------------------------------
// Panel view
// ---------------------------------------------------------------------------

pub fn view(app: &NevcApp) -> Element<'_, Message> {
    if app.connection != ConnectionState::Connected {
        return column![
            text("Graphs").size(24),
            iced::widget::Space::with_height(20),
            text("Not connected \u{2014} connect to the board to enable live graphing.").size(14),
        ]
        .spacing(0)
        .into();
    }

    let has_data = !app.graph_history.is_empty();

    // ---- Controls -----------------------------------------------------------
    let (start_lbl, start_style) = if app.graph_running {
        ("\u{25a0} Stop", iced::theme::Button::Destructive)
    } else {
        ("\u{25ba} Start", iced::theme::Button::Positive)
    };
    let start_btn = button(text(start_lbl).size(14))
        .on_press(Message::GraphStartStop)
        .style(start_style)
        .padding([6, 16]);

    let mode_lbl = match app.graph_mode {
        GraphMode::Overlay => "Individual View",
        GraphMode::Individual => "Overlay View",
    };
    let mode_btn = button(text(mode_lbl).size(13))
        .on_press(Message::GraphModeToggled)
        .style(iced::theme::Button::Secondary)
        .padding([4, 10]);

    let dl_btn = button(text("Save CSV").size(13))
        .on_press(Message::GraphDownloadCsv)
        .style(iced::theme::Button::Secondary)
        .padding([4, 10]);

    let poll_lbl = format!("Rate: {:.0} Hz", app.graph_poll_hz);
    let poll_slider = slider(1.0_f32..=50.0_f32, app.graph_poll_hz, Message::GraphPollRateChanged)
        .step(1.0)
        .width(Length::Fixed(150.0));

    let elapsed = if has_data {
        let t = app.graph_history.back().map(|s| s.t).unwrap_or(0.0);
        format!("{:.1} s  |  {} samples", t, app.graph_history.len())
    } else {
        String::new()
    };

    let controls = row![
        start_btn,
        iced::widget::Space::with_width(8),
        mode_btn,
        iced::widget::Space::with_width(8),
        dl_btn,
        iced::widget::Space::with_width(16),
        text(poll_lbl).size(13),
        iced::widget::Space::with_width(6),
        poll_slider,
        iced::widget::Space::with_width(12),
        text(elapsed).size(11),
    ]
    .align_items(Alignment::Center)
    .spacing(0);

    // ---- Channel checkboxes -------------------------------------------------
    let make_cb = |i: usize| -> Element<'_, Message> {
        let lbl = GRAPH_CHANNEL_NAMES[i].to_string();
        checkbox(lbl, app.graph_channels[i])
            .on_toggle(move |_| Message::GraphChannelToggled(i))
            .size(13)
            .into()
    };

    let channel_row = column![
        row((0..4).map(make_cb).collect::<Vec<_>>()).spacing(14).align_items(Alignment::Center),
        row((4..NUM_CHANNELS).map(make_cb).collect::<Vec<_>>()).spacing(14).align_items(Alignment::Center),
    ]
    .spacing(4);

    // ---- Legend (live values) -----------------------------------------------
    let legend: Vec<Element<'_, Message>> = (0..NUM_CHANNELS)
        .filter(|&i| app.graph_channels[i])
        .map(|i| {
            let val = ch_live(app, i);
            let s = match val {
                Some(v) => format!("\u{25cf} {} {:.3}\u{202f}{}", GRAPH_CHANNEL_NAMES[i], v, GRAPH_CHANNEL_UNITS[i]),
                None => format!("\u{25cf} {} --", GRAPH_CHANNEL_NAMES[i]),
            };
            text(s).size(14).style(iced::theme::Text::Color(PALETTE[i])).into()
        })
        .collect();

    let legend_row: Element<'_, Message> = if legend.is_empty() {
        text("Select channels above to begin graphing.").size(12).into()
    } else {
        row(legend).spacing(16).into()
    };

    // ---- Plot ---------------------------------------------------------------
    let plot: Element<'_, Message> = if app.graph_running || has_data {
        match app.graph_mode {
            GraphMode::Overlay => overlay_view(app),
            GraphMode::Individual => individual_view(app),
        }
    } else {
        iced::widget::container(
            text("Press \u{25ba} Start to begin recording.").size(13),
        )
        .width(Length::Fill)
        .height(Length::Fixed(340.0))
        .center_x()
        .center_y()
        .style(iced::theme::Container::Box)
        .into()
    };

    let content = column![
        text("Graphs").size(24),
        iced::widget::Space::with_height(10),
        controls,
        iced::widget::Space::with_height(8),
        channel_row,
        iced::widget::Space::with_height(6),
        legend_row,
        iced::widget::Space::with_height(6),
        plot,
    ]
    .spacing(0)
    .padding(4);

    scrollable(content).width(Length::Fill).height(Length::Fill).into()
}

// ---------------------------------------------------------------------------
// Overlay / Individual sub-views
// ---------------------------------------------------------------------------

fn overlay_view<'a>(app: &'a NevcApp) -> Element<'a, Message> {
    let num_sel = app.graph_channels.iter().filter(|&&b| b).count();
    if num_sel == 0 {
        return empty_plot("Select at least one channel.");
    }
    let ranges = unit_group_ranges(&app.graph_history, &app.graph_channels);
    Canvas::new(OverlayCanvas {
        history: &app.graph_history,
        channels: &app.graph_channels,
        ranges,
    })
    .width(Length::Fill)
    .height(Length::Fixed(340.0))
    .into()
}

fn individual_view<'a>(app: &'a NevcApp) -> Element<'a, Message> {
    let selected: Vec<usize> = (0..NUM_CHANNELS).filter(|&i| app.graph_channels[i]).collect();
    if selected.is_empty() {
        return empty_plot("Select at least one channel.");
    }
    let ranges = unit_group_ranges(&app.graph_history, &app.graph_channels);
    let mut rows: Vec<Element<'_, Message>> = Vec::new();
    let mut chunks = selected.chunks(2);
    for chunk in &mut chunks {
        let left = build_single(app, chunk[0], &ranges);
        let row_elem: Element<'_, Message> = if chunk.len() == 2 {
            let right = build_single(app, chunk[1], &ranges);
            row![left, iced::widget::Space::with_width(8), right]
                .align_items(Alignment::Start)
                .into()
        } else {
            row![left].into()
        };
        rows.push(row_elem);
    }
    column(rows).spacing(8).into()
}

fn build_single<'a>(app: &'a NevcApp, ch: usize, ranges: &[(f32, f32); 5]) -> Element<'a, Message> {
    let g = GRAPH_CHANNEL_UNIT_GROUP[ch];
    let (ymin, ymax) = {
        let (mn, mx) = ranges[g];
        if mn.is_finite() && mx.is_finite() && (mx - mn).abs() > 1e-9 { (mn, mx) } else { (0.0, 1.0) }
    };
    let canvas_el: Element<'_, Message> = Canvas::new(SingleChannelCanvas {
        history: &app.graph_history,
        channel: ch,
        ymin,
        ymax,
    })
    .width(Length::Fill)
    .height(Length::Fixed(170.0))
    .into();

    column![
        text(format!("{} ({})", GRAPH_CHANNEL_NAMES[ch], GRAPH_CHANNEL_UNITS[ch]))
            .size(12)
            .style(iced::theme::Text::Color(PALETTE[ch])),
        canvas_el,
    ]
    .spacing(2)
    .width(Length::FillPortion(1))
    .into()
}

fn empty_plot<'a>(msg: &'a str) -> Element<'a, Message> {
    iced::widget::container(text(msg).size(13))
        .width(Length::Fill)
        .height(Length::Fixed(220.0))
        .center_x()
        .center_y()
        .style(iced::theme::Container::Box)
        .into()
}

// ---------------------------------------------------------------------------
// Live value lookup (for legend)
// ---------------------------------------------------------------------------

fn ch_live(app: &NevcApp, idx: usize) -> Option<f32> {
    match idx {
        0 => app.speed_rpm,
        1 => app.bus_current,
        2 => app.phase_u_current,
        3 => app.phase_v_current,
        4 => app.phase_w_current,
        5 => app.duty_cycle,
        6 => app.gate_voltage,
        7 => match (app.bus_current, app.gate_voltage) {
            (Some(i), Some(v)) => Some(i * v),
            _ => None,
        },
        _ => None,
    }
}
