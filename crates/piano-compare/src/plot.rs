//! Small hand-drawn plots: axes, ticks, series, a band and hover.

use egui::{Align2, Color32, FontId, Pos2, Rect, Shape, Stroke, StrokeKind, Ui, Vec2};

/// Width of the y axis, shared by the keyboard so that every plot's notes
/// stand under their keys.
pub const AXIS: f32 = 64.0;
pub const RIGHT: f32 = 12.0;

pub struct Series<'a> {
    pub color: Color32,
    pub width: f32,
    pub dashed: bool,
    pub label: &'a str,
    /// (x, y) sorted by x; y already transformed (log10 on a log plot).
    pub points: Vec<(f64, f64)>,
    /// Gaps wider than this are not joined.
    pub join: f64,
}

pub struct Frame {
    pub rect: Rect,
    pub x: (f64, f64),
    pub y: (f64, f64),
}

impl Frame {
    pub fn px(&self, x: f64, y: f64) -> Pos2 {
        let r = self.rect;
        Pos2::new(
            r.left() + ((x - self.x.0) / (self.x.1 - self.x.0)) as f32 * r.width(),
            r.bottom() - ((y - self.y.0) / (self.y.1 - self.y.0)) as f32 * r.height(),
        )
    }
}

/// 1-2-5 ticks covering [lo, hi].
pub fn ticks(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    if hi.is_nan() || lo.is_nan() || hi <= lo {
        return vec![lo];
    }
    let raw = (hi - lo) / n.max(1) as f64;
    let mag = 10f64.powf(raw.log10().floor());
    let step = [1.0, 2.0, 5.0, 10.0].into_iter().map(|m| m * mag).find(|s| *s >= raw).unwrap_or(10.0 * mag);
    let mut t = (lo / step).ceil() * step;
    let mut out = Vec::new();
    while t <= hi + 1e-9 * step {
        out.push(if t.abs() < 1e-12 * step { 0.0 } else { t });
        t += step;
    }
    out
}

/// Ticks of a log10 axis (values are log10): decades, then 2 and 5.
pub fn log_ticks(lo: f64, hi: f64) -> Vec<f64> {
    let mut out = Vec::new();
    for d in (lo.floor() as i32)..=(hi.ceil() as i32) {
        for m in [1.0, 2.0, 5.0] {
            let v = d as f64 + f64::log10(m);
            if v >= lo && v <= hi {
                out.push(v);
            }
        }
    }
    if out.len() > 8 {
        out.retain(|v| (v - v.round()).abs() < 1e-9);
    }
    out
}

pub fn fmt_tick(v: f64, log: bool) -> String {
    let v = if log { 10f64.powf(v) } else { v };
    let a = v.abs();
    if a == 0.0 {
        "0".into()
    } else if a >= 100.0 || (a >= 1.0 && (v - v.round()).abs() < 1e-6) {
        format!("{v:.0}")
    } else if a >= 0.01 {
        let s = format!("{v:.2}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        format!("{v:.0e}")
    }
}

/// Robust y range of the shown values (the 1st and 99th percentiles,
/// widened), so one wild reading does not flatten the others.
pub fn y_range(values: &mut Vec<f64>) -> (f64, f64) {
    values.retain(|v| v.is_finite());
    if values.is_empty() {
        return (0.0, 1.0);
    }
    values.sort_by(f64::total_cmp);
    let q = |p: f64| values[((values.len() - 1) as f64 * p).round() as usize];
    let (lo, hi) = if values.len() >= 50 { (q(0.01), q(0.99)) } else { (values[0], values[values.len() - 1]) };
    let pad = ((hi - lo) * 0.08).max(1e-6);
    (lo - pad, hi + pad)
}

pub fn axes(ui: &Ui, f: &Frame, log: bool, grid: Color32, text: Color32) {
    let p = ui.painter();
    p.rect_stroke(f.rect, 0.0, Stroke::new(1.0, grid), StrokeKind::Inside);
    let yt = if log { log_ticks(f.y.0, f.y.1) } else { ticks(f.y.0, f.y.1, 4) };
    for t in yt {
        let y = f.px(f.x.0, t).y;
        p.line_segment([Pos2::new(f.rect.left(), y), Pos2::new(f.rect.right(), y)], Stroke::new(1.0, grid));
        p.text(Pos2::new(f.rect.left() - 6.0, y), Align2::RIGHT_CENTER, fmt_tick(t, log), FontId::proportional(11.0), text);
    }
}

/// A shaded band with thin edges, so that two overlapping bands both keep
/// their limits.
pub fn band(ui: &Ui, f: &Frame, band: &[(f64, f64, f64)], color: Color32, edge: Color32, join: f64) {
    let p = ui.painter().with_clip_rect(f.rect);
    for w in band.windows(2) {
        let ((x0, lo0, hi0), (x1, lo1, hi1)) = (w[0], w[1]);
        if x1 - x0 > join {
            continue;
        }
        let pts = vec![f.px(x0, lo0), f.px(x1, lo1), f.px(x1, hi1), f.px(x0, hi0)];
        p.add(Shape::convex_polygon(pts, color, Stroke::NONE));
        let stroke = Stroke::new(1.0, edge);
        p.line_segment([f.px(x0, lo0), f.px(x1, lo1)], stroke);
        p.line_segment([f.px(x0, hi0), f.px(x1, hi1)], stroke);
    }
    for &(x, lo, hi) in band {
        // Isolated keys still show their spread.
        p.line_segment([f.px(x, lo), f.px(x, hi)], Stroke::new(2.0, color));
    }
}

pub fn series(ui: &Ui, f: &Frame, s: &Series) {
    let p = ui.painter().with_clip_rect(f.rect.expand(3.0));
    let stroke = Stroke::new(s.width, s.color);
    let mut run: Vec<Pos2> = Vec::new();
    let flush = |run: &mut Vec<Pos2>| {
        if run.len() >= 2 {
            if s.dashed {
                p.extend(Shape::dashed_line(run, stroke, 5.0, 3.0));
            } else {
                p.add(Shape::line(std::mem::take(run), stroke));
            }
        }
        run.clear();
    };
    let mut last: Option<f64> = None;
    for &(x, y) in &s.points {
        if last.is_some_and(|l| x - l > s.join) {
            flush(&mut run);
        }
        run.push(f.px(x, y));
        last = Some(x);
    }
    flush(&mut run);
    for &(x, y) in &s.points {
        p.circle_filled(f.px(x, y), s.width.max(1.5) + 0.5, s.color);
    }
}

/// The point nearest the pointer, within 10 px: (series index, point index).
pub fn nearest(f: &Frame, series: &[Series], pointer: Pos2) -> Option<(usize, usize)> {
    let mut best: Option<(f32, usize, usize)> = None;
    for (si, s) in series.iter().enumerate() {
        for (pi, &(x, y)) in s.points.iter().enumerate() {
            let d = (f.px(x, y) - pointer).length();
            if d < 10.0 && best.is_none_or(|b| d < b.0) {
                best = Some((d, si, pi));
            }
        }
    }
    best.map(|b| (b.1, b.2))
}

pub fn tooltip_box(ui: &Ui, at: Pos2, text: &str, bg: Color32, fg: Color32) {
    let p = ui.painter();
    let galley = p.layout_no_wrap(text.to_string(), FontId::proportional(12.0), fg);
    let size = galley.size() + Vec2::splat(8.0);
    let clip = ui.clip_rect();
    let mut pos = at + Vec2::new(12.0, -size.y - 6.0);
    pos.x = pos.x.min(clip.right() - size.x - 2.0);
    pos.y = pos.y.max(clip.top() + 2.0);
    let r = Rect::from_min_size(pos, size);
    let layer = egui::LayerId::new(egui::Order::Tooltip, ui.scope_id().with("plot-tip"));
    let tp = ui.ctx().layer_painter(layer);
    tp.rect_filled(r, 4.0, bg);
    tp.rect_stroke(r, 4.0, Stroke::new(1.0, fg.gamma_multiply(0.4)), StrokeKind::Inside);
    tp.galley(r.min + Vec2::splat(4.0), galley, fg);
}
