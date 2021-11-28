use std::convert::TryFrom;
use std::error::Error;
use std::fmt;
use std::ops::{Add, Sub};
use std::str::FromStr;

use itertools::Itertools;

use crate::{
    ChordType, Note, PitchClass, Semitones, UkeString, Voicing, VoicingConfig, STRING_COUNT,
};

/// Custom error for strings that cannot be parsed into chords.
#[derive(Debug)]
pub struct ParseChordError {
    name: String,
}

impl Error for ParseChordError {}

impl fmt::Display for ParseChordError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Could not parse chord name \"{}\"", self.name)
    }
}

/// A chord such as C, Cm and so on.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Chord {
    pub root: Note,
    pub chord_type: ChordType,
    pub notes: Vec<Note>,
}

impl Chord {
    pub fn new(root: Note, chord_type: ChordType) -> Self {
        let notes = chord_type.intervals().map(|i| root + i).collect();
        Self {
            root,
            chord_type,
            notes,
        }
    }

    /// Return an iterator over the chord's notes that are played on our instrument.
    ///
    /// If the chord contains more notes than we have strings on our instrument,
    /// only required notes are played.
    pub fn played_notes(&self) -> impl Iterator<Item = Note> + '_ {
        self.chord_type
            .required_intervals()
            .chain(self.chord_type.optional_intervals())
            .take(STRING_COUNT)
            .map(move |i| self.root + i)
    }

    pub fn voicings(&self, config: VoicingConfig) -> impl Iterator<Item = Voicing> + '_ {
        config
            .tuning
            .roots()
            // For each ukulele string, keep track of all the frets that when pressed down
            // while playing the string result in a note of the chord.
            .map(|root| {
                self.played_notes()
                    // Allow each note to be checked twice on the fretboard.
                    .cartesian_product(vec![0, 12])
                    // Determine the fret on which `note` is played.
                    .map(|(note, st)| (root, (note.pitch_class - root.pitch_class) + st, note))
                    // Keep only frets within the given boundaries.
                    .filter(|(_r, fret, _n)| fret >= &config.min_fret && fret <= &config.max_fret)
                    .collect::<Vec<UkeString>>()
            })
            // At this point, we have collected all possible positions of the notes in the chord
            // on each ukulele string. Now let's check all combinations and determine the ones
            // that result in a valid voicing of the chord.
            .multi_cartesian_product()
            // Create voicing from the UkeString vec.
            .map(|us_vec| Voicing::from(&us_vec[..]))
            // Keep only valid voicings.
            .filter(|voicing| voicing.spells_out(self) && voicing.get_span() <= config.max_span)
            .sorted()
    }

    pub fn transpose(&self, semitones: i8) -> Chord {
        match semitones {
            s if s < 0 => self.clone() - semitones.abs() as Semitones,
            _ => self.clone() + semitones as Semitones,
        }
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format!("{}{}", self.root, self.chord_type.to_symbol());
        write!(f, "{} - {} {}", name, self.root, self.chord_type)
    }
}

impl FromStr for Chord {
    type Err = ParseChordError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 1. Check the two first characters of the input string (for notes such as `C#`).
        // 2. Check only the first character (for notes such as `C`).
        for i in (1..3).rev() {
            if let Some(prefix) = s.get(0..i) {
                // Try to convert the prefix into a `Note`.
                if let Ok(root) = Note::from_str(prefix) {
                    // Try to convert the remaining string into a `ChordType`.
                    if let Some(suffix) = s.get(i..) {
                        if let Ok(chord_type) = ChordType::from_str(suffix) {
                            return Ok(Self::new(root, chord_type));
                        }
                    }
                }
            }
        }

        let name = s.to_string();
        Err(ParseChordError { name })
    }
}

impl TryFrom<&[PitchClass]> for Chord {
    type Error = &'static str;

    /// Determine the chord that is represented by a list of pitch classes.
    fn try_from(pitches: &[PitchClass]) -> Result<Self, Self::Error> {
        let chord_type = ChordType::try_from(pitches)?;
        let root = Note::from(pitches[0]);

        Ok(Self::new(root, chord_type))
    }
}

impl Add<Semitones> for Chord {
    type Output = Self;

    fn add(self, n: Semitones) -> Self {
        Self::new(self.root + n, self.chord_type)
    }
}

impl Sub<Semitones> for Chord {
    type Output = Self;

    fn sub(self, n: Semitones) -> Self {
        Self::new(self.root - n, self.chord_type)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use PitchClass::*;

    use super::*;

    #[rstest(
        chord,
        case("Z"),
        case("c"),
        case("ABC"),
        case("C#mb5"),
        case("C#mbla"),
        case("CmMaj"),
        case("CmMaj7b5")
    )]
    fn test_from_str_fail(chord: &str) {
        assert!(Chord::from_str(chord).is_err())
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        case("C", "C", "E", "G"),
        case("C#", "C#", "F", "G#"),
        case("Db", "Db", "F", "Ab"),
        case("D", "D", "F#", "A"),
        case("D#", "D#", "G", "A#"),
        case("Eb", "Eb", "G", "Bb"),
        case("E", "E", "G#", "B"),
        case("F", "F", "A", "C"),
        case("F#", "F#", "A#", "C#"),
        case("Gb", "Gb", "Bb", "Db"),
        case("G", "G", "B", "D"),
        case("G#", "G#", "C", "D#"),
        case("Ab", "Ab", "C", "Eb"),
        case("A", "A", "C#", "E"),
        case("A#", "A#", "D", "F"),
        case("Bb", "Bb", "D", "F"),
        case("B", "B", "D#", "F#")
    )]
    fn test_from_str_major(chord: Chord, root: Note, third: Note, fifth: Note) {
        assert_eq!(chord.notes, vec![root, third, fifth]);
        assert_eq!(chord.chord_type, ChordType::Major);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        seventh,
        case("Cmaj7", "C", "E", "G", "B"),
        case("C#maj7", "C#", "F", "G#", "C"),
        case("Dbmaj7", "Db", "F", "Ab", "C"),
        case("Dmaj7", "D", "F#", "A", "C#"),
        case("D#maj7", "D#", "G", "A#", "D"),
        case("Ebmaj7", "Eb", "G", "Bb", "D"),
        case("Emaj7", "E", "G#", "B", "D#"),
        case("Fmaj7", "F", "A", "C", "E"),
        case("F#maj7", "F#", "A#", "C#", "F"),
        case("Gbmaj7", "Gb", "Bb", "Db", "F"),
        case("Gmaj7", "G", "B", "D", "F#"),
        case("G#maj7", "G#", "C", "D#", "G"),
        case("Abmaj7", "Ab", "C", "Eb", "G"),
        case("Amaj7", "A", "C#", "E", "G#"),
        case("A#maj7", "A#", "D", "F", "A"),
        case("Bbmaj7", "Bb", "D", "F", "A"),
        case("Bmaj7", "B", "D#", "F#", "A#")
    )]
    fn test_from_str_major_seventh(
        chord: Chord,
        root: Note,
        third: Note,
        fifth: Note,
        seventh: Note,
    ) {
        assert_eq!(chord.notes, vec![root, third, fifth, seventh]);
        assert_eq!(chord.chord_type, ChordType::MajorSeventh);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        seventh,
        ninth,
        case("Cmaj9", "C", "E", "G", "B", "D"),
        case("C#maj9", "C#", "F", "G#", "C", "D#"),
        case("Dbmaj9", "Db", "F", "Ab", "C", "Eb"),
        case("Dmaj9", "D", "F#", "A", "C#", "E"),
        case("D#maj9", "D#", "G", "A#", "D", "F"),
        case("Ebmaj9", "Eb", "G", "Bb", "D", "F"),
        case("Emaj9", "E", "G#", "B", "D#", "F#"),
        case("Fmaj9", "F", "A", "C", "E", "G"),
        case("F#maj9", "F#", "A#", "C#", "F", "G#"),
        case("Gbmaj9", "Gb", "Bb", "Db", "F", "Ab"),
        case("Gmaj9", "G", "B", "D", "F#", "A"),
        case("G#maj9", "G#", "C", "D#", "G", "A#"),
        case("Abmaj9", "Ab", "C", "Eb", "G", "Bb"),
        case("Amaj9", "A", "C#", "E", "G#", "B"),
        case("A#maj9", "A#", "D", "F", "A", "C"),
        case("Bbmaj9", "Bb", "D", "F", "A", "C"),
        case("Bmaj9", "B", "D#", "F#", "A#", "C#")
    )]
    fn test_from_str_major_ninth(
        chord: Chord,
        root: Note,
        third: Note,
        fifth: Note,
        seventh: Note,
        ninth: Note,
    ) {
        assert_eq!(chord.notes, vec![root, third, fifth, seventh, ninth]);
        assert_eq!(chord.chord_type, ChordType::MajorNinth);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        seventh,
        ninth,
        eleventh,
        case("Cmaj11", "C", "E", "G", "B", "D", "F"),
        case("C#maj11", "C#", "F", "G#", "C", "D#", "F#"),
        case("Dbmaj11", "Db", "F", "Ab", "C", "Eb", "Gb"),
        case("Dmaj11", "D", "F#", "A", "C#", "E", "G"),
        case("D#maj11", "D#", "G", "A#", "D", "F", "G#"),
        case("Ebmaj11", "Eb", "G", "Bb", "D", "F", "Ab"),
        case("Emaj11", "E", "G#", "B", "D#", "F#", "A"),
        case("Fmaj11", "F", "A", "C", "E", "G", "A#"),
        case("F#maj11", "F#", "A#", "C#", "F", "G#", "B"),
        case("Gbmaj11", "Gb", "Bb", "Db", "F", "Ab", "B"),
        case("Gmaj11", "G", "B", "D", "F#", "A", "C"),
        case("G#maj11", "G#", "C", "D#", "G", "A#", "C#"),
        case("Abmaj11", "Ab", "C", "Eb", "G", "Bb", "Db"),
        case("Amaj11", "A", "C#", "E", "G#", "B", "D"),
        case("A#maj11", "A#", "D", "F", "A", "C", "D#"),
        case("Bbmaj11", "Bb", "D", "F", "A", "C", "Eb"),
        case("Bmaj11", "B", "D#", "F#", "A#", "C#", "E")
    )]
    fn test_from_str_major_eleventh(
        chord: Chord,
        root: Note,
        third: Note,
        fifth: Note,
        seventh: Note,
        ninth: Note,
        eleventh: Note,
    ) {
        assert_eq!(
            chord.notes,
            vec![root, third, fifth, seventh, ninth, eleventh]
        );
        assert_eq!(chord.chord_type, ChordType::MajorEleventh);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        seventh,
        ninth,
        eleventh,
        thirteenth,
        case("Cmaj13", "C", "E", "G", "B", "D", "F", "A"),
        case("C#maj13", "C#", "F", "G#", "C", "D#", "F#", "A#"),
        case("Dbmaj13", "Db", "F", "Ab", "C", "Eb", "Gb", "Bb"),
        case("Dmaj13", "D", "F#", "A", "C#", "E", "G", "B"),
        case("D#maj13", "D#", "G", "A#", "D", "F", "G#", "C"),
        case("Ebmaj13", "Eb", "G", "Bb", "D", "F", "Ab", "C"),
        case("Emaj13", "E", "G#", "B", "D#", "F#", "A", "C#"),
        case("Fmaj13", "F", "A", "C", "E", "G", "A#", "D"),
        case("F#maj13", "F#", "A#", "C#", "F", "G#", "B", "D#"),
        case("Gbmaj13", "Gb", "Bb", "Db", "F", "Ab", "B", "Eb"),
        case("Gmaj13", "G", "B", "D", "F#", "A", "C", "E"),
        case("G#maj13", "G#", "C", "D#", "G", "A#", "C#", "F"),
        case("Abmaj13", "Ab", "C", "Eb", "G", "Bb", "Db", "F"),
        case("Amaj13", "A", "C#", "E", "G#", "B", "D", "F#"),
        case("A#maj13", "A#", "D", "F", "A", "C", "D#", "G"),
        case("Bbmaj13", "Bb", "D", "F", "A", "C", "Eb", "G"),
        case("Bmaj13", "B", "D#", "F#", "A#", "C#", "E", "G#")
    )]
    fn test_from_str_major_thirteenth(
        chord: Chord,
        root: Note,
        third: Note,
        fifth: Note,
        seventh: Note,
        ninth: Note,
        eleventh: Note,
        thirteenth: Note,
    ) {
        assert_eq!(
            chord.notes,
            vec![root, third, fifth, seventh, ninth, eleventh, thirteenth]
        );
        assert_eq!(chord.chord_type, ChordType::MajorThirteenth);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        sixth,
        case("C6", "C", "E", "G", "A"),
        case("C#6", "C#", "F", "G#", "A#"),
        case("Db6", "Db", "F", "Ab", "Bb"),
        case("D6", "D", "F#", "A", "B"),
        case("D#6", "D#", "G", "A#", "C"),
        case("Eb6", "Eb", "G", "Bb", "C"),
        case("E6", "E", "G#", "B", "C#"),
        case("F6", "F", "A", "C", "D"),
        case("F#6", "F#", "A#", "C#", "D#"),
        case("Gb6", "Gb", "Bb", "Db", "Eb"),
        case("G6", "G", "B", "D", "E"),
        case("G#6", "G#", "C", "D#", "F"),
        case("Ab6", "Ab", "C", "Eb", "F"),
        case("A6", "A", "C#", "E", "F#"),
        case("A#6", "A#", "D", "F", "G"),
        case("Bb6", "Bb", "D", "F", "G"),
        case("B6", "B", "D#", "F#", "G#")
    )]
    fn test_from_str_major_sixth(chord: Chord, root: Note, third: Note, fifth: Note, sixth: Note) {
        assert_eq!(chord.notes, vec![root, third, fifth, sixth]);
        assert_eq!(chord.chord_type, ChordType::MajorSixth);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        sixth,
        ninth,
        case("C6/9", "C", "E", "G", "A", "D"),
        case("C#6/9", "C#", "F", "G#", "A#", "D#"),
        case("Db6/9", "Db", "F", "Ab", "Bb", "Eb"),
        case("D6/9", "D", "F#", "A", "B", "E"),
        case("D#6/9", "D#", "G", "A#", "C", "F"),
        case("Eb6/9", "Eb", "G", "Bb", "C", "F"),
        case("E6/9", "E", "G#", "B", "C#", "F#"),
        case("F6/9", "F", "A", "C", "D", "G"),
        case("F#6/9", "F#", "A#", "C#", "D#", "G#"),
        case("Gb6/9", "Gb", "Bb", "Db", "Eb", "Ab"),
        case("G6/9", "G", "B", "D", "E", "A"),
        case("G#6/9", "G#", "C", "D#", "F", "A#"),
        case("Ab6/9", "Ab", "C", "Eb", "F", "Bb"),
        case("A6/9", "A", "C#", "E", "F#", "B"),
        case("A#6/9", "A#", "D", "F", "G", "C"),
        case("Bb6/9", "Bb", "D", "F", "G", "C"),
        case("B6/9", "B", "D#", "F#", "G#", "C#")
    )]
    fn test_from_str_sixth_ninth(
        chord: Chord,
        root: Note,
        third: Note,
        fifth: Note,
        sixth: Note,
        ninth: Note,
    ) {
        assert_eq!(chord.notes, vec![root, third, fifth, sixth, ninth]);
        assert_eq!(chord.chord_type, ChordType::SixthNinth);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        seventh,
        case("C7", "C", "E", "G", "Bb"),
        case("C#7", "C#", "F", "G#", "B"),
        case("Db7", "Db", "F", "Ab", "B"),
        case("D7", "D", "F#", "A", "C"),
        case("D#7", "D#", "G", "A#", "C#"),
        case("Eb7", "Eb", "G", "Bb", "Db"),
        case("E7", "E", "G#", "B", "D"),
        case("F7", "F", "A", "C", "Eb"),
        case("F#7", "F#", "A#", "C#", "E"),
        case("Gb7", "Gb", "Bb", "Db", "E"),
        case("G7", "G", "B", "D", "F"),
        case("G#7", "G#", "C", "D#", "F#"),
        case("Ab7", "Ab", "C", "Eb", "Gb"),
        case("A7", "A", "C#", "E", "G"),
        case("A#7", "A#", "D", "F", "G#"),
        case("Bb7", "Bb", "D", "F", "Ab"),
        case("B7", "B", "D#", "F#", "A")
    )]
    fn test_from_str_dominant_seventh(
        chord: Chord,
        root: Note,
        third: Note,
        fifth: Note,
        seventh: Note,
    ) {
        assert_eq!(chord.notes, vec![root, third, fifth, seventh]);
        assert_eq!(chord.chord_type, ChordType::DominantSeventh);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        seventh,
        ninth,
        case("C9", "C", "E", "G", "Bb", "D"),
        case("C#9", "C#", "F", "G#", "B", "D#"),
        case("Db9", "Db", "F", "Ab", "B", "Eb"),
        case("D9", "D", "F#", "A", "C", "E"),
        case("D#9", "D#", "G", "A#", "C#", "F"),
        case("Eb9", "Eb", "G", "Bb", "Db", "F"),
        case("E9", "E", "G#", "B", "D", "F#"),
        case("F9", "F", "A", "C", "Eb", "G"),
        case("F#9", "F#", "A#", "C#", "E", "G#"),
        case("Gb9", "Gb", "Bb", "Db", "E", "Ab"),
        case("G9", "G", "B", "D", "F", "A"),
        case("G#9", "G#", "C", "D#", "F#", "A#"),
        case("Ab9", "Ab", "C", "Eb", "Gb", "Bb"),
        case("A9", "A", "C#", "E", "G", "B"),
        case("A#9", "A#", "D", "F", "G#", "C"),
        case("Bb9", "Bb", "D", "F", "Ab", "C"),
        case("B9", "B", "D#", "F#", "A", "C#")
    )]
    fn test_from_str_dominant_ninth(
        chord: Chord,
        root: Note,
        third: Note,
        fifth: Note,
        seventh: Note,
        ninth: Note,
    ) {
        assert_eq!(chord.notes, vec![root, third, fifth, seventh, ninth]);
        assert_eq!(chord.chord_type, ChordType::DominantNinth);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        seventh,
        ninth,
        eleventh,
        case("C11", "C", "E", "G", "Bb", "D", "F"),
        case("C#11", "C#", "F", "G#", "B", "D#", "F#"),
        case("Db11", "Db", "F", "Ab", "B", "Eb", "Gb"),
        case("D11", "D", "F#", "A", "C", "E", "G"),
        case("D#11", "D#", "G", "A#", "C#", "F", "G#"),
        case("Eb11", "Eb", "G", "Bb", "Db", "F", "Ab"),
        case("E11", "E", "G#", "B", "D", "F#", "A"),
        case("F11", "F", "A", "C", "Eb", "G", "A#"),
        case("F#11", "F#", "A#", "C#", "E", "G#", "B"),
        case("Gb11", "Gb", "Bb", "Db", "E", "Ab", "B"),
        case("G11", "G", "B", "D", "F", "A", "C"),
        case("G#11", "G#", "C", "D#", "F#", "A#", "C#"),
        case("Ab11", "Ab", "C", "Eb", "Gb", "Bb", "Db"),
        case("A11", "A", "C#", "E", "G", "B", "D"),
        case("A#11", "A#", "D", "F", "G#", "C", "D#"),
        case("Bb11", "Bb", "D", "F", "Ab", "C", "Eb"),
        case("B11", "B", "D#", "F#", "A", "C#", "E")
    )]
    fn test_from_str_dominant_eleventh(
        chord: Chord,
        root: Note,
        third: Note,
        fifth: Note,
        seventh: Note,
        ninth: Note,
        eleventh: Note,
    ) {
        assert_eq!(
            chord.notes,
            vec![root, third, fifth, seventh, ninth, eleventh]
        );
        assert_eq!(chord.chord_type, ChordType::DominantEleventh);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        seventh,
        ninth,
        eleventh,
        thirteenth,
        case("C13", "C", "E", "G", "Bb", "D", "F", "A"),
        case("C#13", "C#", "F", "G#", "B", "D#", "F#", "A#"),
        case("Db13", "Db", "F", "Ab", "B", "Eb", "Gb", "Bb"),
        case("D13", "D", "F#", "A", "C", "E", "G", "B"),
        case("D#13", "D#", "G", "A#", "C#", "F", "G#", "C"),
        case("Eb13", "Eb", "G", "Bb", "Db", "F", "Ab", "C"),
        case("E13", "E", "G#", "B", "D", "F#", "A", "C#"),
        case("F13", "F", "A", "C", "Eb", "G", "A#", "D"),
        case("F#13", "F#", "A#", "C#", "E", "G#", "B", "D#"),
        case("Gb13", "Gb", "Bb", "Db", "E", "Ab", "B", "Eb"),
        case("G13", "G", "B", "D", "F", "A", "C", "E"),
        case("G#13", "G#", "C", "D#", "F#", "A#", "C#", "F"),
        case("Ab13", "Ab", "C", "Eb", "Gb", "Bb", "Db", "F"),
        case("A13", "A", "C#", "E", "G", "B", "D", "F#"),
        case("A#13", "A#", "D", "F", "G#", "C", "D#", "G"),
        case("Bb13", "Bb", "D", "F", "Ab", "C", "Eb", "G"),
        case("B13", "B", "D#", "F#", "A", "C#", "E", "G#")
    )]
    fn test_from_str_dominant_thirteenth(
        chord: Chord,
        root: Note,
        third: Note,
        fifth: Note,
        seventh: Note,
        ninth: Note,
        eleventh: Note,
        thirteenth: Note,
    ) {
        assert_eq!(
            chord.notes,
            vec![root, third, fifth, seventh, ninth, eleventh, thirteenth]
        );
        assert_eq!(chord.chord_type, ChordType::DominantThirteenth);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        seventh,
        ninth,
        case("C7b9", "C", "E", "G", "Bb", "Db"),
        case("C#7b9", "C#", "F", "G#", "B", "D"),
        case("Db7b9", "Db", "F", "Ab", "B", "D"),
        case("D7b9", "D", "F#", "A", "C", "Eb"),
        case("D#7b9", "D#", "G", "A#", "C#", "E"),
        case("Eb7b9", "Eb", "G", "Bb", "Db", "E"),
        case("E7b9", "E", "G#", "B", "D", "F"),
        case("F7b9", "F", "A", "C", "Eb", "F#"),
        case("F#7b9", "F#", "A#", "C#", "E", "G"),
        case("Gb7b9", "Gb", "Bb", "Db", "E", "G"),
        case("G7b9", "G", "B", "D", "F", "Ab"),
        case("G#7b9", "G#", "C", "D#", "F#", "A"),
        case("Ab7b9", "Ab", "C", "Eb", "Gb", "A"),
        case("A7b9", "A", "C#", "E", "G", "Bb"),
        case("A#7b9", "A#", "D", "F", "G#", "B"),
        case("Bb7b9", "Bb", "D", "F", "Ab", "B"),
        case("B7b9", "B", "D#", "F#", "A", "C")
    )]
    fn test_from_str_dominant_seventh_flat_ninth(
        chord: Chord,
        root: Note,
        third: Note,
        fifth: Note,
        seventh: Note,
        ninth: Note,
    ) {
        assert_eq!(chord.notes, vec![root, third, fifth, seventh, ninth]);
        assert_eq!(chord.chord_type, ChordType::DominantSeventhFlatNinth);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        seventh,
        ninth,
        case("C7#9", "C", "E", "G", "Bb", "D#"),
        case("C#7#9", "C#", "F", "G#", "B", "E"),
        case("Db7#9", "Db", "F", "Ab", "B", "E"),
        case("D7#9", "D", "F#", "A", "C", "F"),
        case("D#7#9", "D#", "G", "A#", "C#", "F#"),
        case("Eb7#9", "Eb", "G", "Bb", "Db", "F#"),
        case("E7#9", "E", "G#", "B", "D", "G"),
        case("F7#9", "F", "A", "C", "Eb", "G#"),
        case("F#7#9", "F#", "A#", "C#", "E", "A"),
        case("Gb7#9", "Gb", "Bb", "Db", "E", "A"),
        case("G7#9", "G", "B", "D", "F", "A#"),
        case("G#7#9", "G#", "C", "D#", "F#", "B"),
        case("Ab7#9", "Ab", "C", "Eb", "Gb", "B"),
        case("A7#9", "A", "C#", "E", "G", "C"),
        case("A#7#9", "A#", "D", "F", "G#", "C#"),
        case("Bb7#9", "Bb", "D", "F", "Ab", "C#"),
        case("B7#9", "B", "D#", "F#", "A", "D")
    )]
    fn test_from_str_dominant_seventh_sharp_ninth(
        chord: Chord,
        root: Note,
        third: Note,
        fifth: Note,
        seventh: Note,
        ninth: Note,
    ) {
        assert_eq!(chord.notes, vec![root, third, fifth, seventh, ninth]);
        assert_eq!(chord.chord_type, ChordType::DominantSeventhSharpNinth);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        seventh,
        case("C7b5", "C", "E", "Gb", "Bb"),
        case("C#7b5", "C#", "F", "G", "B"),
        case("Db7b5", "Db", "F", "G", "B"),
        case("D7b5", "D", "F#", "Ab", "C"),
        case("D#7b5", "D#", "G", "A", "C#"),
        case("Eb7b5", "Eb", "G", "A", "Db"),
        case("E7b5", "E", "G#", "Bb", "D"),
        case("F7b5", "F", "A", "B", "Eb"),
        case("F#7b5", "F#", "A#", "C", "E"),
        case("Gb7b5", "Gb", "Bb", "C", "E"),
        case("G7b5", "G", "B", "Db", "F"),
        case("G#7b5", "G#", "C", "D", "F#"),
        case("Ab7b5", "Ab", "C", "D", "Gb"),
        case("A7b5", "A", "C#", "Eb", "G"),
        case("A#7b5", "A#", "D", "E", "G#"),
        case("Bb7b5", "Bb", "D", "E", "Ab"),
        case("B7b5", "B", "D#", "F", "A")
    )]
    fn test_from_str_dominant_seventh_flat_fifth(
        chord: Chord,
        root: Note,
        third: Note,
        fifth: Note,
        seventh: Note,
    ) {
        assert_eq!(chord.notes, vec![root, third, fifth, seventh]);
        assert_eq!(chord.chord_type, ChordType::DominantSeventhFlatFifth);
    }

    #[rstest(
        chord,
        root,
        fourth,
        fifth,
        case("Csus4", "C", "F", "G"),
        case("C#sus4", "C#", "F#", "G#"),
        case("Dbsus4", "Db", "Gb", "Ab"),
        case("Dsus4", "D", "G", "A"),
        case("D#sus4", "D#", "G#", "A#"),
        case("Ebsus4", "Eb", "Ab", "Bb"),
        case("Esus4", "E", "A", "B"),
        case("Fsus4", "F", "Bb", "C"),
        case("F#sus4", "F#", "B", "C#"),
        case("Gbsus4", "Gb", "B", "Db"),
        case("Gsus4", "G", "C", "D"),
        case("G#sus4", "G#", "C#", "D#"),
        case("Absus4", "Ab", "Db", "Eb"),
        case("Asus4", "A", "D", "E"),
        case("A#sus4", "A#", "D#", "F"),
        case("Bbsus4", "Bb", "Eb", "F"),
        case("Bsus4", "B", "E", "F#")
    )]
    fn test_from_str_suspended_fourth(chord: Chord, root: Note, fourth: Note, fifth: Note) {
        assert_eq!(chord.notes, vec![root, fourth, fifth]);
        assert_eq!(chord.chord_type, ChordType::SuspendedFourth);
    }

    #[rstest(
        chord,
        root,
        second,
        fifth,
        case("Csus2", "C", "D", "G"),
        case("C#sus2", "C#", "D#", "G#"),
        case("Dbsus2", "Db", "Eb", "Ab"),
        case("Dsus2", "D", "E", "A"),
        case("D#sus2", "D#", "F", "A#"),
        case("Ebsus2", "Eb", "F", "Bb"),
        case("Esus2", "E", "F#", "B"),
        case("Fsus2", "F", "G", "C"),
        case("F#sus2", "F#", "G#", "C#"),
        case("Gbsus2", "Gb", "Ab", "Db"),
        case("Gsus2", "G", "A", "D"),
        case("G#sus2", "G#", "A#", "D#"),
        case("Absus2", "Ab", "Bb", "Eb"),
        case("Asus2", "A", "B", "E"),
        case("A#sus2", "A#", "C", "F"),
        case("Bbsus2", "Bb", "C", "F"),
        case("Bsus2", "B", "C#", "F#")
    )]
    fn test_from_str_suspended_second(chord: Chord, root: Note, second: Note, fifth: Note) {
        assert_eq!(chord.notes, vec![root, second, fifth]);
        assert_eq!(chord.chord_type, ChordType::SuspendedSecond);
    }

    #[rstest(
        chord,
        root,
        fourth,
        fifth,
        seventh,
        case("C7sus4", "C", "F", "G", "Bb"),
        case("C#7sus4", "C#", "F#", "G#", "B"),
        case("Db7sus4", "Db", "Gb", "Ab", "B"),
        case("D7sus4", "D", "G", "A", "C"),
        case("D#7sus4", "D#", "G#", "A#", "C#"),
        case("Eb7sus4", "Eb", "Ab", "Bb", "Db"),
        case("E7sus4", "E", "A", "B", "D"),
        case("F7sus4", "F", "Bb", "C", "Eb"),
        case("F#7sus4", "F#", "B", "C#", "E"),
        case("Gb7sus4", "Gb", "B", "Db", "E"),
        case("G7sus4", "G", "C", "D", "F"),
        case("G#7sus4", "G#", "C#", "D#", "F#"),
        case("Ab7sus4", "Ab", "Db", "Eb", "Gb"),
        case("A7sus4", "A", "D", "E", "G"),
        case("A#7sus4", "A#", "D#", "F", "G#"),
        case("Bb7sus4", "Bb", "Eb", "F", "Ab"),
        case("B7sus4", "B", "E", "F#", "A")
    )]
    fn test_from_str_dominant_seventh_suspended_fourth(
        chord: Chord,
        root: Note,
        fourth: Note,
        fifth: Note,
        seventh: Note,
    ) {
        assert_eq!(chord.notes, vec![root, fourth, fifth, seventh]);
        assert_eq!(chord.chord_type, ChordType::DominantSeventhSuspendedFourth);
    }

    #[rstest(
        chord,
        root,
        second,
        fifth,
        seventh,
        case("C7sus2", "C", "D", "G", "Bb"),
        case("C#7sus2", "C#", "D#", "G#", "B"),
        case("Db7sus2", "Db", "Eb", "Ab", "B"),
        case("D7sus2", "D", "E", "A", "C"),
        case("D#7sus2", "D#", "F", "A#", "C#"),
        case("Eb7sus2", "Eb", "F", "Bb", "Db"),
        case("E7sus2", "E", "F#", "B", "D"),
        case("F7sus2", "F", "G", "C", "Eb"),
        case("F#7sus2", "F#", "G#", "C#", "E"),
        case("Gb7sus2", "Gb", "Ab", "Db", "E"),
        case("G7sus2", "G", "A", "D", "F"),
        case("G#7sus2", "G#", "A#", "D#", "F#"),
        case("Ab7sus2", "Ab", "Bb", "Eb", "Gb"),
        case("A7sus2", "A", "B", "E", "G"),
        case("A#7sus2", "A#", "C", "F", "G#"),
        case("Bb7sus2", "Bb", "C", "F", "Ab"),
        case("B7sus2", "B", "Db", "F#", "A")
    )]
    fn test_from_str_dominant_seventh_suspended_second(
        chord: Chord,
        root: Note,
        second: Note,
        fifth: Note,
        seventh: Note,
    ) {
        assert_eq!(chord.notes, vec![root, second, fifth, seventh]);
        assert_eq!(chord.chord_type, ChordType::DominantSeventhSuspendedSecond);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        case("Cm", "C", "Eb", "G"),
        case("C#m", "C#", "E", "G#"),
        case("Dbm", "Db", "E", "Ab"),
        case("Dm", "D", "F", "A"),
        case("D#m", "D#", "F#", "A#"),
        case("Ebm", "Eb", "Gb", "Bb"),
        case("Em", "E", "G", "B"),
        case("Fm", "F", "Ab", "C"),
        case("F#m", "F#", "A", "C#"),
        case("Gbm", "Gb", "A", "Db"),
        case("Gm", "G", "Bb", "D"),
        case("G#m", "G#", "B", "D#"),
        case("Abm", "Ab", "B", "Eb"),
        case("Am", "A", "C", "E"),
        case("A#m", "A#", "C#", "F"),
        case("Bbm", "Bb", "Db", "F"),
        case("Bm", "B", "D", "F#")
    )]
    fn test_from_str_minor(chord: Chord, root: Note, third: Note, fifth: Note) {
        assert_eq!(chord.notes, vec![root, third, fifth]);
        assert_eq!(chord.chord_type, ChordType::Minor);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        seventh,
        case("Cm7", "C", "Eb", "G", "Bb"),
        case("C#m7", "C#", "E", "G#", "B"),
        case("Dbm7", "Db", "E", "Ab", "B"),
        case("Dm7", "D", "F", "A", "C"),
        case("D#m7", "D#", "F#", "A#", "C#"),
        case("Ebm7", "Eb", "Gb", "Bb", "Db"),
        case("Em7", "E", "G", "B", "D"),
        case("Fm7", "F", "Ab", "C", "Eb"),
        case("F#m7", "F#", "A", "C#", "E"),
        case("Gbm7", "Gb", "A", "Db", "E"),
        case("Gm7", "G", "Bb", "D", "F"),
        case("G#m7", "G#", "B", "D#", "F#"),
        case("Abm7", "Ab", "B", "Eb", "Gb"),
        case("Am7", "A", "C", "E", "G"),
        case("A#m7", "A#", "C#", "F", "G#"),
        case("Bbm7", "Bb", "Db", "F", "Ab"),
        case("Bm7", "B", "D", "F#", "A")
    )]
    fn test_from_str_minor_seventh(
        chord: Chord,
        root: Note,
        third: Note,
        fifth: Note,
        seventh: Note,
    ) {
        assert_eq!(chord.notes, vec![root, third, fifth, seventh]);
        assert_eq!(chord.chord_type, ChordType::MinorSeventh);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        seventh,
        case("CmMaj7", "C", "Eb", "G", "B"),
        case("C#mMaj7", "C#", "E", "G#", "C"),
        case("DbmMaj7", "Db", "E", "Ab", "C"),
        case("DmMaj7", "D", "F", "A", "C#"),
        case("D#mMaj7", "D#", "F#", "A#", "D"),
        case("EbmMaj7", "Eb", "Gb", "Bb", "D"),
        case("EmMaj7", "E", "G", "B", "D#"),
        case("FmMaj7", "F", "Ab", "C", "E"),
        case("F#mMaj7", "F#", "A", "C#", "F"),
        case("GbmMaj7", "Gb", "A", "Db", "F"),
        case("GmMaj7", "G", "Bb", "D", "F#"),
        case("G#mMaj7", "G#", "B", "D#", "G"),
        case("AbmMaj7", "Ab", "B", "Eb", "G"),
        case("AmMaj7", "A", "C", "E", "G#"),
        case("A#mMaj7", "A#", "C#", "F", "A"),
        case("BbmMaj7", "Bb", "Db", "F", "A"),
        case("BmMaj7", "B", "D", "F#", "A#")
    )]
    fn test_from_str_minor_major_seventh(
        chord: Chord,
        root: Note,
        third: Note,
        fifth: Note,
        seventh: Note,
    ) {
        assert_eq!(chord.notes, vec![root, third, fifth, seventh]);
        assert_eq!(chord.chord_type, ChordType::MinorMajorSeventh);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        sixth,
        case("Cm6", "C", "Eb", "G", "A"),
        case("C#m6", "C#", "E", "G#", "A#"),
        case("Dbm6", "Db", "E", "Ab", "Bb"),
        case("Dm6", "D", "F", "A", "B"),
        case("D#m6", "D#", "F#", "A#", "C"),
        case("Ebm6", "Eb", "Gb", "Bb", "C"),
        case("Em6", "E", "G", "B", "C#"),
        case("Fm6", "F", "Ab", "C", "D"),
        case("F#m6", "F#", "A", "C#", "D#"),
        case("Gbm6", "Gb", "A", "Db", "Eb"),
        case("Gm6", "G", "Bb", "D", "E"),
        case("G#m6", "G#", "B", "D#", "F"),
        case("Abm6", "Ab", "B", "Eb", "F"),
        case("Am6", "A", "C", "E", "F#"),
        case("A#m6", "A#", "C#", "F", "G"),
        case("Bbm6", "Bb", "Db", "F", "G"),
        case("Bm6", "B", "D", "F#", "G#")
    )]
    fn test_from_str_minor_sixth(chord: Chord, root: Note, third: Note, fifth: Note, sixth: Note) {
        assert_eq!(chord.notes, vec![root, third, fifth, sixth]);
        assert_eq!(chord.chord_type, ChordType::MinorSixth);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        seventh,
        ninth,
        case("Cm9", "C", "Eb", "G", "Bb", "D"),
        case("C#m9", "C#", "E", "G#", "B", "D#"),
        case("Dbm9", "Db", "E", "Ab", "B", "Eb"),
        case("Dm9", "D", "F", "A", "C", "E"),
        case("D#m9", "D#", "F#", "A#", "C#", "F"),
        case("Ebm9", "Eb", "Gb", "Bb", "Db", "F"),
        case("Em9", "E", "G", "B", "D", "F#"),
        case("Fm9", "F", "Ab", "C", "Eb", "G"),
        case("F#m9", "F#", "A", "C#", "E", "G#"),
        case("Gbm9", "Gb", "A", "Db", "E", "Ab"),
        case("Gm9", "G", "Bb", "D", "F", "A"),
        case("G#m9", "G#", "B", "D#", "F#", "A#"),
        case("Abm9", "Ab", "B", "Eb", "Gb", "Bb"),
        case("Am9", "A", "C", "E", "G", "B"),
        case("A#m9", "A#", "C#", "F", "G#", "C"),
        case("Bbm9", "Bb", "Db", "F", "Ab", "C"),
        case("Bm9", "B", "D", "F#", "A", "C#")
    )]
    fn test_from_str_minor_ninth(
        chord: Chord,
        root: Note,
        third: Note,
        fifth: Note,
        seventh: Note,
        ninth: Note,
    ) {
        assert_eq!(chord.notes, vec![root, third, fifth, seventh, ninth]);
        assert_eq!(chord.chord_type, ChordType::MinorNinth);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        seventh,
        ninth,
        eleventh,
        case("Cm11", "C", "Eb", "G", "Bb", "D", "F"),
        case("C#m11", "C#", "E", "G#", "B", "D#", "F#"),
        case("Dbm11", "Db", "E", "Ab", "B", "Eb", "Gb"),
        case("Dm11", "D", "F", "A", "C", "E", "G"),
        case("D#m11", "D#", "F#", "A#", "C#", "F", "G#"),
        case("Ebm11", "Eb", "Gb", "Bb", "Db", "F", "Ab"),
        case("Em11", "E", "G", "B", "D", "F#", "A"),
        case("Fm11", "F", "Ab", "C", "Eb", "G", "A#"),
        case("F#m11", "F#", "A", "C#", "E", "G#", "B"),
        case("Gbm11", "Gb", "A", "Db", "E", "Ab", "B"),
        case("Gm11", "G", "Bb", "D", "F", "A", "C"),
        case("G#m11", "G#", "B", "D#", "F#", "A#", "C#"),
        case("Abm11", "Ab", "B", "Eb", "Gb", "Bb", "Db"),
        case("Am11", "A", "C", "E", "G", "B", "D"),
        case("A#m11", "A#", "C#", "F", "G#", "C", "D#"),
        case("Bbm11", "Bb", "Db", "F", "Ab", "C", "Eb"),
        case("Bm11", "B", "D", "F#", "A", "C#", "E")
    )]
    fn test_from_str_minor_eleventh(
        chord: Chord,
        root: Note,
        third: Note,
        fifth: Note,
        seventh: Note,
        ninth: Note,
        eleventh: Note,
    ) {
        assert_eq!(
            chord.notes,
            vec![root, third, fifth, seventh, ninth, eleventh]
        );
        assert_eq!(chord.chord_type, ChordType::MinorEleventh);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        seventh,
        ninth,
        eleventh,
        thirteenth,
        case("Cm13", "C", "Eb", "G", "Bb", "D", "F", "A"),
        case("C#m13", "C#", "E", "G#", "B", "D#", "F#", "A#"),
        case("Dbm13", "Db", "E", "Ab", "B", "Eb", "Gb", "Bb"),
        case("Dm13", "D", "F", "A", "C", "E", "G", "B"),
        case("D#m13", "D#", "F#", "A#", "C#", "F", "G#", "C"),
        case("Ebm13", "Eb", "Gb", "Bb", "Db", "F", "Ab", "C"),
        case("Em13", "E", "G", "B", "D", "F#", "A", "C#"),
        case("Fm13", "F", "Ab", "C", "Eb", "G", "A#", "D"),
        case("F#m13", "F#", "A", "C#", "E", "G#", "B", "D#"),
        case("Gbm13", "Gb", "A", "Db", "E", "Ab", "B", "Eb"),
        case("Gm13", "G", "Bb", "D", "F", "A", "C", "E"),
        case("G#m13", "G#", "B", "D#", "F#", "A#", "C#", "F"),
        case("Abm13", "Ab", "B", "Eb", "Gb", "Bb", "Db", "F"),
        case("Am13", "A", "C", "E", "G", "B", "D", "F#"),
        case("A#m13", "A#", "C#", "F", "G#", "C", "D#", "G"),
        case("Bbm13", "Bb", "Db", "F", "Ab", "C", "Eb", "G"),
        case("Bm13", "B", "D", "F#", "A", "C#", "E", "G#")
    )]
    fn test_from_str_minor_thirteenth(
        chord: Chord,
        root: Note,
        third: Note,
        fifth: Note,
        seventh: Note,
        ninth: Note,
        eleventh: Note,
        thirteenth: Note,
    ) {
        assert_eq!(
            chord.notes,
            vec![root, third, fifth, seventh, ninth, eleventh, thirteenth]
        );
        assert_eq!(chord.chord_type, ChordType::MinorThirteenth);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        case("Cdim", "C", "Eb", "Gb"),
        case("C#dim", "C#", "E", "G"),
        case("Dbdim", "Db", "E", "G"),
        case("Ddim", "D", "F", "Ab"),
        case("D#dim", "D#", "F#", "A"),
        case("Ebdim", "Eb", "Gb", "A"),
        case("Edim", "E", "G", "Bb"),
        case("Fdim", "F", "Ab", "B"),
        case("F#dim", "F#", "A", "C"),
        case("Gbdim", "Gb", "A", "C"),
        case("Gdim", "G", "Bb", "Db"),
        case("G#dim", "G#", "B", "D"),
        case("Abdim", "Ab", "B", "D"),
        case("Adim", "A", "C", "Eb"),
        case("A#dim", "A#", "C#", "E"),
        case("Bbdim", "Bb", "Db", "E"),
        case("Bdim", "B", "D", "F")
    )]
    fn test_from_str_diminished(chord: Chord, root: Note, third: Note, fifth: Note) {
        assert_eq!(chord.notes, vec![root, third, fifth]);
        assert_eq!(chord.chord_type, ChordType::Diminished);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        seventh,
        case("Cdim7", "C", "Eb", "Gb", "A"),
        case("C#dim7", "C#", "E", "G", "Bb"),
        case("Dbdim7", "Db", "E", "G", "Bb"),
        case("Ddim7", "D", "F", "Ab", "B"),
        case("D#dim7", "D#", "F#", "A", "C"),
        case("Ebdim7", "Eb", "Gb", "A", "C"),
        case("Edim7", "E", "G", "Bb", "Db"),
        case("Fdim7", "F", "Ab", "B", "D"),
        case("F#dim7", "F#", "A", "C", "Eb"),
        case("Gbdim7", "Gb", "A", "C", "Eb"),
        case("Gdim7", "G", "Bb", "Db", "E"),
        case("G#dim7", "G#", "B", "D", "F"),
        case("Abdim7", "Ab", "B", "D", "F"),
        case("Adim7", "A", "C", "Eb", "Gb"),
        case("A#dim7", "A#", "C#", "E", "G"),
        case("Bbdim7", "Bb", "Db", "E", "G"),
        case("Bdim7", "B", "D", "F", "Ab")
    )]
    fn test_from_str_diminished_seventh(
        chord: Chord,
        root: Note,
        third: Note,
        fifth: Note,
        seventh: Note,
    ) {
        assert_eq!(chord.notes, vec![root, third, fifth, seventh]);
        assert_eq!(chord.chord_type, ChordType::DiminishedSeventh);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        seventh,
        case("Cm7b5", "C", "Eb", "Gb", "Bb"),
        case("C#m7b5", "C#", "E", "G", "B"),
        case("Dbm7b5", "Db", "E", "G", "B"),
        case("Dm7b5", "D", "F", "Ab", "C"),
        case("D#m7b5", "D#", "F#", "A", "C#"),
        case("Ebm7b5", "Eb", "Gb", "A", "Db"),
        case("Em7b5", "E", "G", "Bb", "D"),
        case("Fm7b5", "F", "Ab", "B", "Eb"),
        case("F#m7b5", "F#", "A", "C", "E"),
        case("Gbm7b5", "Gb", "A", "C", "E"),
        case("Gm7b5", "G", "Bb", "Db", "F"),
        case("G#m7b5", "G#", "B", "D", "F#"),
        case("Abm7b5", "Ab", "B", "D", "Gb"),
        case("Am7b5", "A", "C", "Eb", "G"),
        case("A#m7b5", "A#", "C#", "E", "G#"),
        case("Bbm7b5", "Bb", "Db", "E", "Ab"),
        case("Bm7b5", "B", "D", "F", "A")
    )]
    fn test_from_str_half_diminished_seventh(
        chord: Chord,
        root: Note,
        third: Note,
        fifth: Note,
        seventh: Note,
    ) {
        assert_eq!(chord.notes, vec![root, third, fifth, seventh]);
        assert_eq!(chord.chord_type, ChordType::HalfDiminishedSeventh);
    }

    #[rstest(
        chord,
        root,
        fifth,
        case("C5", "C", "G"),
        case("C#5", "C#", "G#"),
        case("Db5", "Db", "Ab"),
        case("D5", "D", "A"),
        case("D#5", "D#", "A#"),
        case("Eb5", "Eb", "Bb"),
        case("E5", "E", "B"),
        case("F5", "F", "C"),
        case("F#5", "F#", "C#"),
        case("Gb5", "Gb", "Db"),
        case("G5", "G", "D"),
        case("G#5", "G#", "D#"),
        case("Ab5", "Ab", "Eb"),
        case("A5", "A", "E"),
        case("A#5", "A#", "F"),
        case("Bb5", "Bb", "F"),
        case("B5", "B", "F#")
    )]
    fn test_from_str_fifth(chord: Chord, root: Note, fifth: Note) {
        assert_eq!(chord.notes, vec![root, fifth]);
        assert_eq!(chord.chord_type, ChordType::Fifth);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        case("Caug", "C", "E", "G#"),
        case("C#aug", "C#", "F", "A"),
        case("Dbaug", "Db", "F", "A"),
        case("Daug", "D", "F#", "A#"),
        case("D#aug", "D#", "G", "B"),
        case("Ebaug", "Eb", "G", "B"),
        case("Eaug", "E", "G#", "C"),
        case("Faug", "F", "A", "C#"),
        case("F#aug", "F#", "A#", "D"),
        case("Gbaug", "Gb", "Bb", "D"),
        case("Gaug", "G", "B", "D#"),
        case("G#aug", "G#", "C", "E"),
        case("Abaug", "Ab", "C", "E"),
        case("Aaug", "A", "C#", "F"),
        case("A#aug", "A#", "D", "F#"),
        case("Bbaug", "Bb", "D", "F#"),
        case("Baug", "B", "D#", "G")
    )]
    fn test_from_str_augmented(chord: Chord, root: Note, third: Note, fifth: Note) {
        assert_eq!(chord.notes, vec![root, third, fifth]);
        assert_eq!(chord.chord_type, ChordType::Augmented);
    }

    #[rstest(
        chord_base,
        root,
        third,
        fifth,
        seventh,
        case("C", "C", "E", "G#", "Bb"),
        case("C#", "C#", "F", "A", "B"),
        case("Db", "Db", "F", "A", "B"),
        case("D", "D", "F#", "A#", "C"),
        case("D#", "D#", "G", "B", "C#"),
        case("Eb", "Eb", "G", "B", "Db"),
        case("E", "E", "G#", "C", "D"),
        case("F", "F", "A", "C#", "Eb"),
        case("F#", "F#", "A#", "D", "E"),
        case("Gb", "Gb", "Bb", "D", "E"),
        case("G", "G", "B", "D#", "F"),
        case("G#", "G#", "C", "E", "F#"),
        case("Ab", "Ab", "C", "E", "Gb"),
        case("A", "A", "C#", "F", "G"),
        case("A#", "A#", "D", "F#", "G#"),
        case("Bb", "Bb", "D", "F#", "Ab"),
        case("B", "B", "D#", "G", "A")
    )]
    fn test_from_str_augmented_seventh(
        #[values("aug7", "7#5")] chord_suffix: &str,
        chord_base: &str,
        root: Note,
        third: Note,
        fifth: Note,
        seventh: Note,
    ) {
        let chord = Chord::from_str(&format!("{}{}", chord_base, chord_suffix)).unwrap();

        assert_eq!(chord.notes, vec![root, third, fifth, seventh]);
        assert_eq!(chord.chord_type, ChordType::AugmentedSeventh);
    }

    #[rstest(
        chord,
        root,
        third,
        fifth,
        seventh,
        case("CaugMaj7", "C", "E", "G#", "B"),
        case("C#augMaj7", "C#", "F", "A", "C"),
        case("DbaugMaj7", "Db", "F", "A", "C"),
        case("DaugMaj7", "D", "F#", "A#", "C#"),
        case("D#augMaj7", "D#", "G", "B", "D"),
        case("EbaugMaj7", "Eb", "G", "B", "D"),
        case("EaugMaj7", "E", "G#", "C", "D#"),
        case("FaugMaj7", "F", "A", "C#", "E"),
        case("F#augMaj7", "F#", "A#", "D", "F"),
        case("GbaugMaj7", "Gb", "Bb", "D", "F"),
        case("GaugMaj7", "G", "B", "D#", "F#"),
        case("G#augMaj7", "G#", "C", "E", "G"),
        case("AbaugMaj7", "Ab", "C", "E", "G"),
        case("AaugMaj7", "A", "C#", "F", "G#"),
        case("A#augMaj7", "A#", "D", "F#", "A"),
        case("BbaugMaj7", "Bb", "D", "F#", "A"),
        case("BaugMaj7", "B", "D#", "G", "A#")
    )]
    fn test_from_str_augmented_major_seventh(
        chord: Chord,
        root: Note,
        third: Note,
        fifth: Note,
        seventh: Note,
    ) {
        assert_eq!(chord.notes, vec![root, third, fifth, seventh]);
        assert_eq!(chord.chord_type, ChordType::AugmentedMajorSeventh);
    }

    #[rstest(
        chord_base,
        root,
        third,
        fifth,
        ninth,
        case("C", "C", "E", "G", "D"),
        case("C#", "C#", "F", "G#", "D#"),
        case("Db", "Db", "F", "Ab", "Eb"),
        case("D", "D", "F#", "A", "E"),
        case("D#", "D#", "G", "A#", "F"),
        case("Eb", "Eb", "G", "Bb", "F"),
        case("E", "E", "G#", "B", "F#"),
        case("F", "F", "A", "C", "G"),
        case("F#", "F#", "A#", "C#", "G#"),
        case("Gb", "Gb", "Bb", "Db", "Ab"),
        case("G", "G", "B", "D", "A"),
        case("G#", "G#", "C", "D#", "A#"),
        case("Ab", "Ab", "C", "Eb", "Bb"),
        case("A", "A", "C#", "E", "B"),
        case("A#", "A#", "D", "F", "C"),
        case("Bb", "Bb", "D", "F", "C"),
        case("B", "B", "D#", "F#", "C#")
    )]
    fn test_from_str_added_ninth(
        #[values("add9", "add2")] chord_suffix: &str,
        chord_base: &str,
        root: Note,
        third: Note,
        fifth: Note,
        ninth: Note,
    ) {
        let chord = Chord::from_str(&format!("{}{}", chord_base, chord_suffix)).unwrap();

        assert_eq!(chord.notes, vec![root, third, fifth, ninth]);
        assert_eq!(chord.chord_type, ChordType::AddedNinth);
    }

    #[rstest(
        chord,
        root,
        third,
        fourth,
        fifth,
        case("Cadd4", "C", "E", "F", "G"),
        case("C#add4", "C#", "F", "F#", "G#"),
        case("Dbadd4", "Db", "F", "Gb", "Ab"),
        case("Dadd4", "D", "F#", "G", "A"),
        case("D#add4", "D#", "G", "G#", "A#"),
        case("Ebadd4", "Eb", "G", "Ab", "Bb"),
        case("Eadd4", "E", "G#", "A", "B"),
        case("Fadd4", "F", "A", "Bb", "C"),
        case("F#add4", "F#", "A#", "B", "C#"),
        case("Gbadd4", "Gb", "Bb", "B", "Db"),
        case("Gadd4", "G", "B", "C", "D"),
        case("G#add4", "G#", "C", "C#", "D#"),
        case("Abadd4", "Ab", "C", "Db", "Eb"),
        case("Aadd4", "A", "C#", "D", "E"),
        case("A#add4", "A#", "D", "D#", "F"),
        case("Bbadd4", "Bb", "D", "Eb", "F"),
        case("Badd4", "B", "D#", "E", "F#")
    )]
    fn test_from_str_added_fourth(
        chord: Chord,
        root: Note,
        third: Note,
        fourth: Note,
        fifth: Note,
    ) {
        assert_eq!(chord.notes, vec![root, third, fourth, fifth]);
        assert_eq!(chord.chord_type, ChordType::AddedFourth);
    }

    #[rstest(
        pitches,
        chord,
        // Test C-chords.
        case(vec![C, E, G], "C"),
        case(vec![C, DSharp, G], "Cm"),
        case(vec![C, D, G], "Csus2"),
        case(vec![C, F, G], "Csus4"),
        case(vec![C, E, GSharp], "Caug"),
        case(vec![C, DSharp, FSharp], "Cdim"),
        case(vec![C, E, G, ASharp], "C7"),
        case(vec![C, DSharp, G, ASharp], "Cm7"),
        case(vec![C, E, G, B], "Cmaj7"),
        case(vec![C, DSharp, G, B], "CmMaj7"),
        case(vec![C, E, GSharp, ASharp], "Caug7"),
        case(vec![C, E, GSharp, B], "CaugMaj7"),
        case(vec![C, DSharp, FSharp, A], "Cdim7"),
        case(vec![C, DSharp, FSharp, ASharp], "Cm7b5"),
        // Test some chords with other root notes.
        case(vec![D, FSharp, A], "D"),
        case(vec![D, F, A], "Dm"),
        case(vec![D, FSharp, A, C], "D7"),
        case(vec![G, B, D], "G"),
        // Test pitch class list in different order.
        case(vec![C, G, E], "C"),
    )]
    fn test_get_chord_type(pitches: Vec<PitchClass>, chord: Chord) {
        assert_eq!(Chord::try_from(&pitches[..]).unwrap(), chord);
    }

    #[rstest(
        chord1,
        n,
        chord2,
        case("C", 0, "C"),
        case("C#", 0, "C#"),
        case("Db", 0, "Db"),
        case("Cm", 1, "C#m"),
        case("Cmaj7", 2, "Dmaj7"),
        case("Cdim", 4, "Edim"),
        case("C#", 2, "D#"),
        case("A#m", 3, "C#m"),
        case("A", 12, "A"),
        case("A#", 12, "A#"),
        case("Ab", 12, "Ab")
    )]
    fn test_add_semitones(chord1: Chord, n: Semitones, chord2: Chord) {
        assert_eq!(chord1 + n, chord2);
    }

    #[rstest(
        chord1,
        n,
        chord2,
        case("C", 0, "C"),
        case("C#", 0, "C#"),
        case("Db", 0, "Db"),
        case("Cm", 1, "Bm"),
        case("Cmaj7", 2, "Bbmaj7"),
        case("Adim", 3, "Gbdim"),
        case("A", 12, "A"),
        case("A#", 12, "A#"),
        case("Ab", 12, "Ab")
    )]
    fn test_subtract_semitones(chord1: Chord, n: Semitones, chord2: Chord) {
        assert_eq!(chord1 - n, chord2);
    }

    #[rstest(
        chord1,
        n,
        chord2,
        case("C", 0, "C"),
        case("C#", 0, "C#"),
        case("Db", 0, "Db"),
        case("Cm", 1, "C#m"),
        case("Cmaj7", 2, "Dmaj7"),
        case("Cdim", 4, "Edim"),
        case("C#", 2, "D#"),
        case("A#m", 3, "C#m"),
        case("A", 12, "A"),
        case("A#", 12, "A#"),
        case("Ab", 12, "Ab"),
        case("Cm", -1, "Bm"),
        case("Cmaj7", -2, "Bbmaj7"),
        case("Adim", -3, "Gbdim"),
        case("A", -12, "A"),
        case("A#", -12, "A#"),
        case("Ab", -12, "Ab")
    )]
    fn test_transpose(chord1: Chord, n: i8, chord2: Chord) {
        assert_eq!(chord1.transpose(n), chord2);
    }

    #[rstest(
        chord,
        played_notes,
        case("C", vec!["C", "E", "G"]),
        case("C7", vec!["C", "E", "Bb", "G"]),
        case("C11", vec!["C", "E", "Bb", "F"]),
        case("C13", vec!["C", "E", "Bb", "A"]),
    )]
    fn test_played_notes(chord: Chord, played_notes: Vec<&str>) {
        let pn1: Vec<_> = chord.played_notes().collect();
        let pn2: Vec<_> = played_notes
            .iter()
            .map(|&s| Note::from_str(s).unwrap())
            .collect();

        assert_eq!(pn1, pn2);
    }
}
