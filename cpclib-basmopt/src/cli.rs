//! `clap`-derive command line, feature-gated behind `cmdline` so the rest of
//! this crate stays usable as a plain library (see `lib.rs`'s own doc
//! comment for why).

use camino::Utf8PathBuf;

use crate::{OptimizationGoal, Options};

#[derive(clap::Parser, Debug)]
#[command(
    name = "basmopt",
    about = "Z80 peephole-optimization advisor/fixer for basm sources",
    after_help = "EXIT STATUS:\n    \
        0   no optimization opportunities found, or -i successfully applied fixes\n    \
        1   opportunities found and not fixed (no -i given)\n    \
        2   an error occurred (bad file, parse/assemble failure, bad rule file)"
)]
pub struct Cli {
    /// The .asm source file to analyze.
    pub source: Utf8PathBuf,

    /// Rewrite the source in place instead of printing suggestions.
    #[arg(short = 'i', long = "in-place")]
    pub in_place: bool,

    /// What to optimize for - `size` and `speed` add rules the neutral
    /// (default) set leaves out, including some that actively disagree with
    /// each other (see `cpclib-asmoptim::builtin_rules`'s own doc comment).
    #[arg(long, value_enum, default_value_t = GoalArg::Neutral)]
    pub goal: GoalArg,

    /// Extra pattern file to load on top of the built-in rules, in the same
    /// format as the vendored ones (mdlz80optimizer's `pattern:`/`name:`/
    /// numbered-line/`constraints:` syntax). Repeatable.
    #[arg(long = "rules", value_name = "FILE")]
    pub extra_rules: Vec<Utf8PathBuf>,

    /// Rule name to skip, even if the built-in set or a --rules file defines
    /// it. Only named rules (a rule's `name:` line) can be targeted this
    /// way. Repeatable.
    #[arg(long = "disable", value_name = "NAME")]
    pub disabled: Vec<String>,

    /// Skip the built-in rule set entirely - only --rules are used.
    #[arg(long)]
    pub no_builtin: bool,

    /// Extra directory to search when resolving this file's `INCLUDE`s -
    /// same role as `basm`'s own `-I`/`--include`. Only ever needed if the
    /// active rule set actually requires a real assemble (currently just
    /// `--goal size`'s `jp2jr`); the default goal never touches `INCLUDE` at
    /// all. Repeatable.
    #[arg(short = 'I', long = "include", value_name = "DIR")]
    pub include_dirs: Vec<Utf8PathBuf>
}

impl Cli {
    pub fn options(&self) -> Options {
        Options {
            goal: self.goal.into(),
            extra_rule_files: self.extra_rules.clone(),
            disabled_rules: self.disabled.clone(),
            no_builtin: self.no_builtin,
            include_dirs: self.include_dirs.clone()
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GoalArg {
    #[default]
    Neutral,
    Size,
    Speed
}

impl From<GoalArg> for OptimizationGoal {
    fn from(goal: GoalArg) -> Self {
        match goal {
            GoalArg::Neutral => OptimizationGoal::Neutral,
            GoalArg::Size => OptimizationGoal::Size,
            GoalArg::Speed => OptimizationGoal::Speed
        }
    }
}
