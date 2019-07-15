use matchers::Pattern;
use std::str::FromStr;

pub struct Directive {
    span: Option<&'static str>,
}

enum ValueMatch {
    Bool(bool),
    U64(u64),
    I64(i64),
    Pat(Pattern),
}

// === impl ValueMatch ===

impl FromStr for ValueMatch {
    type Err = matchers::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<bool>().map(ValueMatch::Bool)
            .or_else(|_| s.parse::<u64>().map(ValueMatch::U64))
            .or_else(|_| s.parse::<i64>().map(ValueMatch::I64))
            .or_else(|_| s.parse::<Pattern>().map(ValueMatch::Pat))
    }
}
