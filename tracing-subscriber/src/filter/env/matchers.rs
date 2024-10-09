/// Regex Matchers on characters and byte streams. This code is inspired by the [matchers](https://github.com/hawkw/matchers) crate
/// The code was stripped down as everything was not needed and uses an updated version of regex-automata. 
use regex_automata::dfa::dense::{BuildError, DFA};
use regex_automata::dfa::Automaton;
use regex_automata::util::primitives::StateID;
use regex_automata::util::start::Config;
use regex_automata::Anchored;
use std::{fmt, fmt::Write};

#[derive(Debug, Clone)]
pub(crate) struct Pattern<A = DFA<Vec<u32>>> {
    automaton: A,
}

#[derive(Debug, Clone)]
pub(crate) struct Matcher<A = DFA<Vec<u32>>> {
    automaton: A,
    state: StateID,
}

impl Pattern {
    pub(crate) fn new(pattern: &str) -> Result<Self, BuildError> {
        let automaton = DFA::new(pattern)?;
        Ok(Pattern { automaton })
    }
}

impl<A: Automaton> Pattern<A> {
    pub(crate) fn matcher(&self) -> Matcher<&'_ A> {
        Matcher {
            automaton: &self.automaton,
            state: self
                .automaton
                .start_state(&Config::new().anchored(Anchored::Yes))
                .unwrap(),
        }
    }

    pub(crate) fn matches(&self, s: &impl AsRef<str>) -> bool {
        self.matcher().matches(s)
    }

    pub(crate) fn debug_matches(&self, d: &impl fmt::Debug) -> bool {
        self.matcher().debug_matches(d)
    }
}

// === impl Matcher ===

impl<A> Matcher<A>
where
    A: Automaton,
{
    #[inline]
    fn advance(&mut self, input: u8) {
        self.state = unsafe { self.automaton.next_state_unchecked(self.state, input) };
    }

    #[inline]
    pub(crate) fn is_matched(&self) -> bool {
        let eoi_state = self.automaton.next_eoi_state(self.state);
        self.automaton.is_match_state(eoi_state)
    }

    /// Returns `true` if this pattern matches the formatted output of the given
    /// type implementing `fmt::Debug`.
    pub(crate) fn matches(mut self, s: &impl AsRef<str>) -> bool {
        for &byte in s.as_ref().as_bytes() {
            self.advance(byte);
            if self.automaton.is_dead_state(self.state) {
                return false;
            }
        }
        self.is_matched()
    }

    pub(crate) fn debug_matches(mut self, d: &impl fmt::Debug) -> bool {
        write!(&mut self, "{:?}", d).expect("matcher write impl should not fail");
        self.is_matched()
    }
}

impl<A: Automaton> fmt::Write for Matcher<A> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &byte in s.as_bytes() {
            self.advance(byte);
            if self.automaton.is_dead_state(self.state) {
                break;
            }
        }
        Ok(())
    }
}
