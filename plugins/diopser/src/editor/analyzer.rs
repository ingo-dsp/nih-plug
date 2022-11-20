// Diopser: a phase rotation plugin
// Copyright (C) 2021-2022 Robbert van der Helm
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use atomic_float::AtomicF32;
use nih_plug::nih_debug_assert;
use nih_plug::prelude::FloatRange;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use crate::spectrum::SpectrumOutput;

/// A very abstract spectrum analyzer. This draws the magnitude spectrum's bins as vertical lines
/// with the same distirubtion as the filter frequency parmaeter..
pub struct SpectrumAnalyzer {
    spectrum: Arc<Mutex<SpectrumOutput>>,
    sample_rate: Arc<AtomicF32>,

    /// The same range as that used by the filter frequency parameter. We'll use this to make sure
    /// we draw the spectrum analyzer's ticks at locations that match the frequency parameter linked
    /// to the X-Y pad's X-axis.
    frequency_range: FloatRange,
}

impl SpectrumAnalyzer {
    /// Creates a new [`SpectrumAnalyzer`]. The uses custom drawing.
    pub fn new<LSpectrum, LRate>(
        cx: &mut Context,
        spectrum: LSpectrum,
        sample_rate: LRate,
    ) -> Handle<Self>
    where
        LSpectrum: Lens<Target = Arc<Mutex<SpectrumOutput>>>,
        LRate: Lens<Target = Arc<AtomicF32>>,
    {
        Self {
            spectrum: spectrum.get(cx),
            sample_rate: sample_rate.get(cx),

            frequency_range: crate::filter_frequency_range(),
        }
        .build(
            cx,
            // This is an otherwise empty element only used for custom drawing
            |_cx| (),
        )
    }
}

impl View for SpectrumAnalyzer {
    fn element(&self) -> Option<&'static str> {
        Some("spectrum-analyzer")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        if bounds.w == 0.0 || bounds.h == 0.0 {
            return;
        }

        // This spectrum buffer is written to at the end of the process function when the editor is
        // open
        let mut spectrum = self.spectrum.lock().unwrap();
        let spectrum = spectrum.read();
        let nyquist = self.sample_rate.load(Ordering::Relaxed) / 2.0;

        // This skips background and border drawing
        let line_width = cx.style.dpi_factor as f32 * 1.5;
        let paint = vg::Paint::color(cx.font_color().cloned().unwrap_or_default().into())
            .with_line_width(line_width);
        for (bin_idx, magnetude) in spectrum.iter().enumerate() {
            // We'll match up the bin's x-coordinate with the filter frequency parameter
            let frequency = (bin_idx as f32 / spectrum.len() as f32) * nyquist;
            let t = self.frequency_range.normalize(frequency);
            if t <= 0.0 || t >= 1.0 {
                continue;
            }

            // Scale this so that 1.0/0 dBFS magnetude is at 80% of the height, the bars begin at
            // -80 dBFS, and that the scaling is linear
            nih_debug_assert!(*magnetude >= 0.0);
            let magnetude_db = nih_plug::util::gain_to_db(*magnetude);
            let height = ((magnetude_db + 80.0) / 100.0).clamp(0.0, 1.0);

            let mut path = vg::Path::new();
            path.move_to(
                bounds.x + (bounds.w * t),
                bounds.y + (bounds.h * (1.0 - height)),
            );
            path.line_to(bounds.x + (bounds.w * t), bounds.y + bounds.h);

            canvas.stroke_path(&mut path, &paint);
        }
    }
}
