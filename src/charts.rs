// src/charts.rs

use eframe::egui;
use egui::Ui;
// Removed unused imports CoordinatesFormatter, Corner
use egui_plot::Bar;
use egui_plot::BarChart;
use egui_plot::Legend;
use egui_plot::Line;
use egui_plot::Plot;
use egui_plot::PlotPoint;
use egui_plot::PlotPoints;
use egui_plot::Text;
use tracing::trace;
use std::collections::HashMap;
// Added DateTime back, removed unused NaiveDateTime, NaiveTime (still used in process_*)
use chrono::DateTime;
use chrono::Datelike;
use chrono::Duration;
use chrono::NaiveDateTime;
use chrono::NaiveTime;
use chrono::Timelike;
use chrono::Utc;
use tracing::info; // Needed for formatter signature

use crate::t3_json::T3Message;

/// Enum to define the available quick chart views.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)] // Add Hash for HashMap key
pub enum ChartType {
    DayOfWeek,
    Last30Days,
    Last12Months,
    TimeOfDay,
}

impl ChartType {
    /// Returns a human-readable name for the chart type.
    pub fn name(&self) -> &'static str {
        match self {
            ChartType::DayOfWeek => "Message Volume by Day of Week",
            ChartType::Last30Days => "Message Volume Last 30 Days",
            ChartType::Last12Months => "Message Volume Last 12 Months",
            ChartType::TimeOfDay => "Message Volume by Time of Day",
        }
    }

    /// Returns all available chart types.
    pub fn all() -> &'static [ChartType] {
        &[
            ChartType::DayOfWeek,
            ChartType::Last30Days,
            ChartType::Last12Months,
            ChartType::TimeOfDay,
        ]
    }
}

/// Structure to hold the state related to charting for a file.
pub struct ChartState {
    // No lifetime here
    pub selected_chart: ChartType,
    /// Cache for processed chart data. Storing data that egui_plot can use directly.
    /// Using 'static lifetime as PlotPoints created from Vec<[f64; 2]> should own their data.
    processed_data: HashMap<ChartType, ProcessedChartData>, // No lifetime here
}

/// Structure to hold the processed data for drawing a chart.
#[derive(Clone)]
pub enum ProcessedChartData {
    Bars {
        points: Vec<Bar>,
        labels: Vec<String>,
    }, // For categorical data like Day of Week
    Lines {
        points: Vec<PlotPoint>,
    }, // Explicitly static lifetime for owned data
    None, // No data or error in processing
}

impl ChartState {
    // No lifetime on impl
    /// Creates a new ChartState with a default selected chart.
    pub fn new() -> Self {
        Self {
            selected_chart: ChartType::DayOfWeek, // Default view
            processed_data: HashMap::new(),
        }
    }

    /// Processes the raw messages data to generate plot points for a specific chart type.
    /// Caches the result.
    /// Note: The messages slice here only provides data for processing. The resulting
    /// PlotPoints and Bars should own their data ('static lifetime).
    fn process_messages(
        &mut self,
        chart_type: ChartType,
        messages: &[T3Message],
    ) -> ProcessedChartData {
        // Check cache first
        if let Some(data) = self.processed_data.get(&chart_type) {
            trace!("Chart data for {:?} found in cache.", chart_type);
            return data.clone();
        }

        info!("Processing chart data for {:?} (not in cache).", chart_type);

        let processed_data = match chart_type {
            ChartType::DayOfWeek => self.process_day_of_week(messages),
            ChartType::Last30Days => self.process_last_n_days(messages, 30),
            ChartType::Last12Months => self.process_last_n_months(messages, 12),
            ChartType::TimeOfDay => self.process_time_of_day(messages),
        };

        // Store in cache
        self.processed_data
            .insert(chart_type, processed_data.clone());
        processed_data
    }
    fn process_day_of_week(&self, messages: &[T3Message]) -> ProcessedChartData {
        // Map Sunday=0...Saturday=6 to Monday=0...Sunday=6 for typical charting
        let day_order = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let mut counts = HashMap::new();

        for message in messages {
            let weekday = message.created_at.weekday().num_days_from_monday() as usize;
            *counts.entry(weekday).or_insert(0) += 1;
        }

        let mut bars = vec![];
        // No need for a separate labels vec for BarChart - labels can be handled by
        // a custom axis formatter on the Plot.

        // Generate data for all 7 days, even if count is 0
        for i in 0..7 {
            let count = *counts.get(&i).unwrap_or(&0);
            // Use `i` as the x-coordinate, counts as y.
            bars.push(
                Bar::new(i as f64, count as f64)
                    // Optional: Add names to bars if needed for legend
                    .name(day_order[i]),
            );
        }

        // Sorting by argument (weekday index 0-6) ensures correct order
        bars.sort_by(|a, b| a.argument.partial_cmp(&b.argument).unwrap());

        // The labels Vec is used *within* the custom axis formatter closure, not stored directly
        // in ProcessedChartData::Bars points. Let's remove it from the enum variant.
        // We'll need to return labels separately or generate them in the formatter.
        // A better approach: The axis formatter closure captures the `labels` data.
        // Let's update ProcessedChartData::Bars to NOT store labels, but the formatter
        // will use the fixed `day_order` array.

        // Reverted this decision - keeping labels in the enum for easier access in the formatter.
        ProcessedChartData::Bars {
            points: bars,
            labels: day_order.iter().map(|s| s.to_string()).collect(),
        } // Keep labels here for the formatter to access easily
    }

    /// Aggregates messages for the last N days.
    /// This processes all messages and then filters to the last N days.
    /// For Line chart, we need points sorted by date.
    fn process_last_n_days(&self, messages: &[T3Message], n: usize) -> ProcessedChartData {
        let now_date = Utc::now().date_naive(); // Use NaiveDate for date comparison

        let mut counts_by_date: HashMap<NaiveDateTime, usize> = HashMap::new();

        for message in messages {
            let message_date_time = message.created_at.naive_utc(); // Use naive_utc
            let message_date = message_date_time.date();

            // Check if the message date is within the last N days (inclusive of today)
            if message_date >= now_date - Duration::days(n as i64 - 1) && message_date <= now_date {
                // Bin by day (strip time component for counting)
                let day_start = message_date.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
                *counts_by_date.entry(day_start).or_insert(0) += 1;
            }
        }

        // Collect dates and counts, then sort by date
        let mut dated_counts: Vec<(NaiveDateTime, usize)> = counts_by_date.into_iter().collect();
        dated_counts.sort_by_key(|(date, _)| *date);

        // Convert to PlotPoints (using seconds since epoch for flexibility)
        let points: Vec<[f64; 2]> = dated_counts
            .into_iter()
            .map(|(date, count)| {
                // Convert NaiveDateTime to seconds since epoch as f64
                // Use and_utc().timestamp() as suggested by compiler warning
                let seconds_since_epoch = date.and_utc().timestamp() as f64;
                [seconds_since_epoch, count as f64]
            })
            .collect();

        if points.is_empty() {
            ProcessedChartData::None
        } else {
            ProcessedChartData::Lines {
                points: points.into_iter().map(|point| point.into()).collect(),
            }
        }
    }

    /// Aggregates messages for the last 12 months.
    /// Similar to last N days, but bins by month.
    fn process_last_n_months(&self, messages: &[T3Message], n: usize) -> ProcessedChartData {
        let now = Utc::now();
        // Calculate the start date for N months ago.
        // Chrono's Duration doesn't handle months well due to varying lengths.
        // A more robust way is to work with years and months.
        let mut start_date = now.date_naive();
        let mut current_month_start = start_date.with_day(1).unwrap_or(start_date);

        for _ in 0..(n - 1) {
            // Subtract n-1 months to get the start of the Nth month ago
            current_month_start = if current_month_start.month() == 1 {
                current_month_start
                    .with_year(current_month_start.year() - 1)
                    .unwrap_or(current_month_start)
                    .with_month(12)
                    .unwrap_or(current_month_start)
            } else {
                current_month_start
                    .with_month(current_month_start.month() - 1)
                    .unwrap_or(current_month_start)
            };
        }
        start_date = current_month_start;

        let mut counts_by_month: HashMap<NaiveDateTime, usize> = HashMap::new();

        for message in messages {
            let message_date_time = message.created_at.naive_utc();

            // Check if the message is within the last N months (inclusive of the start month)
            if message_date_time.date() >= start_date {
                // Bin by month (start of the month)
                let month_start = message_date_time
                    .date()
                    .with_day(1)
                    .unwrap_or(message_date_time.date())
                    .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
                *counts_by_month.entry(month_start).or_insert(0) += 1;
            }
        }

        // Collect dates and counts, then sort by date
        let mut dated_counts: Vec<(NaiveDateTime, usize)> = counts_by_month.into_iter().collect();
        dated_counts.sort_by_key(|(date, _)| *date);

        // Convert to PlotPoints (using seconds since epoch for flexibility)
        let points: Vec<[f64; 2]> = dated_counts
            .into_iter()
            .map(|(date, count)| {
                // Convert NaiveDateTime to seconds since epoch as f64
                let seconds_since_epoch = date.and_utc().timestamp() as f64;
                [seconds_since_epoch, count as f64]
            })
            .collect();

        if points.is_empty() {
            ProcessedChartData::None
        } else {
            ProcessedChartData::Lines {
                points: points.into_iter().map(|point| point.into()).collect(),
            }
        }
    }

    /// Aggregates messages by time of day, binned by 30 minutes.
    fn process_time_of_day(&self, messages: &[T3Message]) -> ProcessedChartData {
        let mut counts_by_bin = HashMap::new();
        let _bin_duration = Duration::minutes(30); // This variable was unused, prefix with _

        for message in messages {
            let time = message.created_at.time();
            // Calculate the 30-minute bin index (0-47)
            // Hours * 2 + (minutes / 30)
            let bin_index = (time.hour() * 2 + time.minute() / 30) as usize;
            *counts_by_bin.entry(bin_index).or_insert(0) += 1;
        }

        let mut bars = vec![];
        let mut labels = vec![]; // Labels for time bins (e.g., "00:00", "00:30")

        // Generate data for all 48 bins (00:00 to 23:30), even if count is 0
        for bin_index in 0..48 {
            let count = *counts_by_bin.get(&bin_index).unwrap_or(&0);
            // Use bin_index as the x-coordinate
            bars.push(Bar::new(bin_index as f64, count as f64));

            // Generate time label for the bin start
            let hour = bin_index / 2;
            let minute = (bin_index % 2) * 30;
            let time_label = format!("{:02}:{:02}", hour, minute);
            labels.push(time_label);
        }

        // Bars are naturally sorted by bin_index (argument)

        ProcessedChartData::Bars {
            points: bars,
            labels,
        }
    }

    /// Draws the currently selected chart using egui_plot.
    /// This will trigger processing if the data is not cached.
    pub fn draw(&mut self, ui: &mut Ui, messages: &[T3Message]) {
        // Process messages if needed and get the data
        let chart_data = self.process_messages(self.selected_chart, messages);

        ui.label(format!("Showing: {}", self.selected_chart.name()));

        let mut plot = Plot::new(format!("{}_plot", self.selected_chart.name()))
            .data_aspect(1.0) // Adjust aspect ratio as needed
            .legend(Legend::default());

        // Configure axis formatters based on the selected chart type BEFORE showing the plot
        match self.selected_chart {
            ChartType::DayOfWeek => {
                // Assuming ProcessedChartData::Bars contains the labels Vec for DayOfWeek
                if let ProcessedChartData::Bars { labels, .. } = &chart_data {
                    let labels_clone = labels.clone(); // Clone labels for the closure
                    // Explicitly type closure arguments
                    plot = plot.x_axis_formatter(move |x, _range| {
                        let index = x.value.round() as usize;
                        if index < labels_clone.len() {
                            labels_clone[index].clone()
                        } else {
                            "".to_string() // Should not happen if points and labels match
                        }
                    });
                }
            }
            ChartType::Last30Days | ChartType::Last12Months => {
                // Explicitly type closure arguments
                plot = plot.x_axis_formatter(|x, _range| {
                    // Basic formatter for seconds since epoch - can be improved
                    // Use DateTime::from_timestamp
                    if let Some(date_time) =
                        DateTime::<Utc>::from_timestamp(x.value.round() as i64, 0)
                    {
                        // Format date appropriately
                        if self.selected_chart == ChartType::Last30Days {
                            date_time.format("%m-%d").to_string() // Month-Day for recent days
                        } else {
                            // Last12Months
                            date_time.format("%Y-%m").to_string() // Year-Month for months
                        }
                    } else {
                        "".to_string() // Invalid timestamp
                    }
                });
            }
            ChartType::TimeOfDay => {
                // Assuming ProcessedChartData::Bars contains the labels Vec for TimeOfDay
                if let ProcessedChartData::Bars { labels, .. } = &chart_data {
                    let labels_clone = labels.clone(); // Clone labels for the closure
                    // Explicitly type closure arguments
                    plot = plot.x_axis_formatter(move |x, _range| {
                        let index = x.value.round() as usize;
                        if index < labels_clone.len() {
                            labels_clone[index].clone()
                        } else {
                            "".to_string() // Should not happen if points and labels match
                        }
                    });
                }
            }
        }

        plot.show(ui, |plot_ui| {
            match &chart_data {
                ProcessedChartData::Bars { points, labels: _ } => {
                    // labels is not used here, only in formatter
                    if !points.is_empty() {
                        // BarChart::new takes name and vector of bars
                        let bar_chart =
                            BarChart::new(self.selected_chart.name(), points.clone()) // Provide name and bars
                                .name("Messages") // This name is for the legend entry for the set of bars
                                .color(egui::Color32::LIGHT_BLUE);
                        plot_ui.bar_chart(bar_chart);
                    }
                }
                ProcessedChartData::Lines { points } => {
                    if !points.is_empty() {
                        // Line::new takes name and PlotPoints
                        let line = Line::new("Messages", PlotPoints::Borrowed(points)) // points is PlotPoints
                            .color(egui::Color32::LIGHT_BLUE);
                        plot_ui.line(line);
                    }
                }
                ProcessedChartData::None => {
                    // Use Text::new(position, text) or Text::new(name, position, text) depending on egui_plot version
                    // Let's check egui_plot 0.32.1 docs... it's Text::new(name, position, text)
                    plot_ui.text(Text::new(
                        "",
                        PlotPoint::new(0.5, 0.5),
                        "No data available for this chart type.",
                    )); // Name, Position, Text
                }
            }
        });
    }
}
