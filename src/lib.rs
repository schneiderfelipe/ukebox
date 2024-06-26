#![allow(clippy::upper_case_acronyms)]

pub mod chord;
pub mod chord_chart;
pub mod chord_sequence;
pub mod chord_type;
pub mod distance;
pub mod fingering;
pub mod fret_pattern;
pub mod interval;
pub mod note;
pub mod pitch_class;
pub mod staff_position;
pub mod tuning;
pub mod voicing;
pub mod voicing_graph;

pub use chord::Chord;
pub use chord_chart::ChordChart;
pub use chord_sequence::ChordSequence;
pub use chord_type::{ChordType, NoMatchingChordTypeFoundError};
pub use distance::Distance;
pub use fingering::Fingering;
pub use fret_pattern::FretPattern;
pub use interval::Interval;
pub use note::Note;
pub use pitch_class::PitchClass;
pub use staff_position::StaffPosition;
pub use tuning::Tuning;
pub use voicing::Voicing;
pub use voicing_graph::VoicingGraph;

/// Number of strings on our string instrument.
pub const STRING_COUNT: usize = 4;

/// Number of fingers on our left hand to be used for pressing down strings.
pub const FINGER_COUNT: usize = 4;

/// Number of pitch classes.
pub const PITCH_CLASS_COUNT: Semitones = 12;

/// Minimal number of frets to be shown in a chord chart.
pub const MIN_CHART_WIDTH: Semitones = 4;

/// The ID of a fret on the fretboard. 0 corresponds to the nut,
/// 1 corresponds to the first fret, 2 to the second etc.
pub type FretID = u8;

/// The number of semitones (corresponds to the number of frets)
/// to move from one note or pitch class to another.
pub type Semitones = u8;

/// The number of steps in a staff to move from one staff position
/// to another.
pub type StaffSteps = u8;

/// The position of a finger on a certain string in a certain fret.
/// For example, (3, 4) depicts the fourth fret on the third string.
pub type FingerPosition = (u8, u8);

/// A certain configuration of a ukulele string consisting of
/// the string's root note, the ID of a fret on this string and
/// the note that is played if this fret is pressed down.
pub type UkeString = (Note, FretID, Note);

#[derive(Clone, Copy)]
pub struct VoicingConfig {
    pub tuning: Tuning,
    pub min_fret: FretID,
    pub max_fret: FretID,
    pub max_span: Semitones,
}

impl Default for VoicingConfig {
    fn default() -> Self {
        Self {
            tuning: Tuning::C,
            min_fret: 0,
            max_fret: 12,
            max_span: 4,
        }
    }
}
