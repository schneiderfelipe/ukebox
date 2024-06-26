use clap::Parser;
use itertools::Itertools;
use lazy_static::lazy_static;
use ukebox::{
    Chord, ChordChart, ChordSequence, ChordType, FretID, FretPattern, Semitones, Tuning, Voicing,
    VoicingConfig, VoicingGraph,
};

/// Maximal possible fret ID.
/// According to Wikipedia, the biggest ukulele type (baritone) has 21 frets.
const MAX_FRET_ID: FretID = 21;

/// Maximal span of frets.
/// Playing a chord that spans more than 5 frets seems anatomically impossible to me.
const MAX_SPAN: Semitones = 5;

// See https://github.com/TeXitoi/structopt/issues/150
lazy_static! {
    static ref DEFAULT_CONFIG: VoicingConfig = VoicingConfig::default();
    static ref TUNING_STR: String = DEFAULT_CONFIG.tuning.to_string();
    static ref MIN_FRET_STR: String = DEFAULT_CONFIG.min_fret.to_string();
    static ref MAX_FRET_STR: String = DEFAULT_CONFIG.max_fret.to_string();
    static ref MAX_SPAN_STR: String = DEFAULT_CONFIG.max_span.to_string();
}

#[derive(Parser)]
struct Ukebox {
    /// Type of tuning to be used
    #[arg(short, long, global = true, value_name = "TUNING", default_value = &**TUNING_STR, value_enum)]
    tuning: Tuning,
    #[command(subcommand)]
    cmd: Subcommand,
}

#[derive(Parser)]
enum Subcommand {
    /// List all supported chord types and symbols
    Chords {},
    /// Chord chart lookup
    ///
    /// Enter note names as capital letters A - G.
    /// Add '#' for sharp notes, e.g. D#.
    /// Add 'b' for flat notes, e.g. Eb.
    ///
    /// Run "ukebox chords" to get a list of the chord types and symbols currently supported.
    #[command(verbatim_doc_comment)]
    Chart {
        /// Print out all voicings of <chord> that fulfill the given conditions
        #[arg(short, long)]
        all: bool,
        #[command(flatten)]
        voicing_opts: VoicingOpts,
        /// Name of the chord to be shown
        #[arg(value_name = "CHORD")]
        chord: Chord,
    },
    /// Chord name lookup
    Name {
        /// A compact chart representing the finger positions of the chord to be looked up
        #[arg(value_name = "FRET_PATTERN")]
        fret_pattern: FretPattern,
    },
    /// Voice leading for a sequence of chords
    VoiceLead {
        #[command(flatten)]
        voicing_opts: VoicingOpts,
        /// Chord sequence
        #[arg(value_name = "CHORD_SEQUENCE")]
        chord_seq: ChordSequence,
    },
}

#[derive(Parser)]
pub struct VoicingOpts {
    /// Minimal fret (= minimal position) from which to play <chord>
    #[arg(long, value_name = "FRET_ID", default_value = &**MIN_FRET_STR, value_parser = clap::value_parser!(FretID).range(0..=MAX_FRET_ID as i64))]
    min_fret: FretID,
    /// Maximal fret up to which to play <chord>
    #[arg(long, value_name = "FRET_ID", default_value = &**MAX_FRET_STR, value_parser = clap::value_parser!(FretID).range(0..=MAX_FRET_ID as i64))]
    max_fret: FretID,
    /// Maximal span between the first and the last fret pressed down when playing <chord>
    #[arg(long, value_name = "FRET_COUNT", default_value = &**MAX_SPAN_STR, value_parser = clap::value_parser!(Semitones).range(0..=MAX_SPAN as i64))]
    max_span: Semitones,
    /// Number of semitones to add (e.g. 1, +1) or to subtract (e.g. -1)
    #[arg(
        long,
        value_name = "SEMITONES",
        allow_hyphen_values = true,
        default_value = "0"
    )]
    transpose: i8,
}

fn main() {
    let args = Ukebox::parse();
    let tuning = args.tuning;

    match args.cmd {
        Subcommand::Chords {} => {
            println!("Supported chord types and symbols\n");
            println!("The root note C is used as an example.\n");

            for chord_type in ChordType::values() {
                let symbols = chord_type.symbols().map(|s| format!("C{s}")).join(", ");
                println!("C {chord_type} - {symbols}");
            }
        }
        Subcommand::Chart {
            all,
            voicing_opts,
            chord,
        } => {
            let chord = chord.transpose(voicing_opts.transpose);

            let config = VoicingConfig {
                tuning,
                min_fret: voicing_opts.min_fret,
                max_fret: voicing_opts.max_fret,
                max_span: voicing_opts.max_span,
            };

            let mut voicings = chord.voicings(config).peekable();

            if voicings.peek().is_none() {
                println!("No matching chord voicing was found");
            } else {
                println!("[{chord}]\n");
            }

            for voicing in voicings {
                let chart = ChordChart::new(voicing, voicing_opts.max_span);
                println!("{chart}");

                if !all {
                    break;
                }
            }
        }
        Subcommand::Name { fret_pattern } => {
            let voicing = Voicing::new(fret_pattern, tuning);
            let chords = voicing.get_chords();

            if chords.is_empty() {
                println!("No matching chord was found");
            }

            for chord in chords {
                println!("{chord}");
            }
        }
        Subcommand::VoiceLead {
            voicing_opts,
            chord_seq,
        } => {
            let chord_seq = chord_seq.transpose(voicing_opts.transpose);

            let config = VoicingConfig {
                tuning,
                min_fret: voicing_opts.min_fret,
                max_fret: voicing_opts.max_fret,
                max_span: voicing_opts.max_span,
            };

            let mut voicing_graph = VoicingGraph::new(config);
            voicing_graph.add(&chord_seq);

            let mut path_found = false;

            for (path, _dist) in voicing_graph.paths(1) {
                for (chord, voicing) in chord_seq.chords().zip(path.iter()) {
                    println!("[{chord}]\n");
                    let chart = ChordChart::new(*voicing, voicing_opts.max_span);
                    println!("{chart}");
                }
                //println!("{:?}\n", dist);
                //println!("---------------------------\n");

                path_found = true;
            }

            if !path_found {
                println!("No matching chord voicing sequence was found");
            }
        }
    }
}
