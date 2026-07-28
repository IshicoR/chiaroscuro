use iced_plot::{Color, PointId, ShapeId};

use super::{ChartMarker, ChartRange, TimeSeriesChart};

pub(super) struct SeriesState {
    pub(super) ids: Vec<ShapeId>,
    pub(super) labels: Vec<&'static str>,
    pub(super) colors: Vec<Color>,
    pub(super) lengths: Vec<usize>,
    pub(super) visible: Vec<bool>,
    #[cfg(test)]
    pub(super) update_count: usize,
}

impl TimeSeriesChart {
    pub fn set_markers(&mut self, markers: &[ChartMarker]) {
        if self.markers != markers {
            self.markers = markers.to_vec();
        }
    }

    pub fn set_ranges(&mut self, ranges: &[ChartRange]) {
        if self.ranges != ranges {
            self.ranges = ranges.to_vec();
        }
    }

    pub fn set_series_points(&mut self, index: usize, points: &[[f64; 2]]) {
        if let Some(id) = self.series.ids.get(index) {
            if points.is_empty() && self.series.lengths.get(index) == Some(&0) {
                return;
            }
            #[cfg(test)]
            {
                self.series.update_count += 1;
            }
            self.plot.set_series_positions(id, points);
            if let Some(length) = self.series.lengths.get_mut(index) {
                *length = points.len();
            }
            if !self.axis.live_mode {
                self.interaction.focus_index = None;
            }
        }
    }

    /// Trims an existing live series and appends only the samples captured
    /// since its previous telemetry refresh.
    pub fn update_live_series_points(
        &mut self,
        index: usize,
        minimum_x: f64,
        appended: &[[f64; 2]],
    ) {
        let Some(id) = self.series.ids.get(index).copied() else {
            return;
        };

        let should_trim = minimum_x.is_finite()
            && self
                .plot
                .point_position(PointId {
                    series_id: id,
                    point_index: 0,
                })
                .is_some_and(|position| position[0] < minimum_x);
        if appended.is_empty() && !should_trim {
            return;
        }

        let mut updated_length = None;
        if self
            .plot
            .update_series(&id, |series| {
                let removed = if minimum_x.is_finite() {
                    series
                        .positions
                        .partition_point(|position| position[0] < minimum_x)
                } else {
                    0
                };
                let previous_length = series.positions.len();

                if let Some(colors) = &mut series.point_colors {
                    colors.resize(previous_length, series.color);
                    colors.drain(..removed);
                    colors.extend(std::iter::repeat_n(series.color, appended.len()));
                }

                series.positions.drain(..removed);
                series.positions.extend_from_slice(appended);
                updated_length = Some(series.positions.len());
            })
            .is_err()
        {
            return;
        }

        #[cfg(test)]
        {
            self.series.update_count += 1;
        }
        if let Some(length) = updated_length
            && let Some(series_length) = self.series.lengths.get_mut(index)
        {
            *series_length = length;
        }
        if !self.axis.live_mode {
            self.interaction.focus_index = None;
        }
    }

    #[doc(hidden)]
    pub fn series_length(&self, index: usize) -> Option<usize> {
        self.series.lengths.get(index).copied()
    }
}
