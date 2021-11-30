use druid::{Data, Lens};
use regex::{Regex, RegexBuilder};

#[derive(Clone, Default, Debug, Data, Lens)]
pub struct Finder {
    pub focused_result: usize,
    pub results: usize,
    pub show: bool,
    pub query: String,
}

impl Finder {
    pub fn reset(&mut self) {
        self.query = String::new();
        self.results = 0;
        self.focused_result = 0;
    }

    pub fn reset_matches(&mut self) {
        self.results = 0;
    }

    pub fn report_match(&mut self) -> usize {
        self.results += 1;
        self.results
    }

    pub fn focus_previous(&mut self) {
        self.focused_result = if self.focused_result > 0 {
            self.focused_result - 1
        } else {
            self.results.saturating_sub(1)
        };
    }

    pub fn focus_next(&mut self) {
        self.focused_result = if self.focused_result < self.results - 1 {
            self.focused_result + 1
        } else {
            0
        }
    }
}

#[derive(Clone)]
pub struct FindQuery {
    regex: Regex,
}

impl FindQuery {
    pub fn new(query: &str) -> Self {
        Self {
            regex: Self::build_regex(query),
        }
    }

    fn build_regex(query: &str) -> Regex {
        RegexBuilder::new(&regex::escape(query))
            .case_insensitive(true)
            .build()
            .unwrap()
    }

    pub fn is_empty(&self) -> bool {
        self.regex.as_str().is_empty()
    }

    pub fn matches_str(&self, s: &str) -> bool {
        self.regex.is_match(s)
    }
}

pub trait MatchFindQuery {
    fn matches_query(&self, query: &FindQuery) -> bool;
}
