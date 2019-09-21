mod chord_diagram;
mod string_diagram;

pub use self::chord_diagram::ChordDiagram;
pub use self::string_diagram::StringDiagram;

use crate::Frets;

/// Number of frets shown on the fretboard chart.
pub const CHART_WIDTH: Frets = 4;