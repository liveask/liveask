use chrono::{DateTime, Duration, Utc};
use plotters::prelude::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlottingError {
    #[error("Plot Error: {0}")]
    DrawError(#[from] DrawingAreaErrorKind<std::io::Error>),
}

pub fn plot_questions(
    buffer: &mut String,
    data: &[(DateTime<Utc>, i32)],
) -> std::result::Result<(), PlottingError> {
    let root = SVGBackend::with_string(buffer, (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;

    let (to_date, from_date) = (
        data[data.len() - 1].0 + Duration::hours(1),
        data[0].0 - Duration::hours(1),
    );

    let max_likes = data
        .iter()
        .map(|(_, likes)| *likes)
        .max()
        .unwrap_or_default()
        .saturating_add(2);

    let mut chart = ChartBuilder::on(&root)
        .x_label_area_size(50_i32)
        .y_label_area_size(50_i32)
        .caption("Questions", ("sans-serif", 40.0_f64).into_font())
        .build_cartesian_2d(
            (from_date..to_date).step(Duration::days(1_i64)),
            0_i32..max_likes,
        )?;

    chart.configure_mesh().light_line_style(WHITE).draw()?;

    chart.draw_series(
        data.iter()
            .map(|x| Circle::new((x.0, x.1), 4_i32, BLUE.filled())),
    )?;

    root.present()?;

    Ok(())
}
