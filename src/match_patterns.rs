/*
 * libssg
 *
 * Copyright 2020 Manos Pitsidianakis
 *
 * This file is part of libssg.
 *
 * libssg is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * libssg is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with libssg. If not, see <http://www.gnu.org/licenses/>.
 */

//! Match patterns for files with regexps or literals.

use super::*;
use std::env;

/// Match files in current directory by using literals, regex or a list of patterns.
#[derive(Debug)]
pub enum MatchPattern {
    Literal(String),
    Regex(regex::Regex),
    List(Vec<MatchPattern>),
}

impl<S: AsRef<str>> From<S> for MatchPattern {
    fn from(from: S) -> Self {
        regex::Regex::new(from.as_ref())
            .map_or_else(|_| Self::Literal(from.as_ref().to_string()), Self::Regex)
    }
}

impl MatchPattern {
    /// Returns iterator of [`std::fs::DirEntry`]s for every matching entry.
    pub fn list(self) -> MatchPathIter {
        let current_dir = env::current_dir().unwrap();
        MatchPathIter(
            self,
            vec![fs::read_dir(current_dir).expect("Could not read current directory")],
        )
    }
}

/// Iterator of [`std::fs::DirEntry`]s for every matching entry.
#[derive(Debug)]
pub struct MatchPathIter(MatchPattern, Vec<fs::ReadDir>);

impl Iterator for MatchPathIter {
    type Item = fs::DirEntry;
    fn next(&mut self) -> Option<Self::Item> {
        if self.1.is_empty() {
            return None;
        }
        let iter = self.1.last_mut().unwrap();
        let next = if let Some(next) = iter.next() {
            next
        } else {
            self.1.pop();
            return self.next();
        };

        let entry = next.unwrap();
        let path = entry.path();
        // FIXME: Smarter exclude patterns.
        if path.is_dir() && !path.ends_with(".git") && !path.ends_with("target") {
            if let Ok(dir) = fs::read_dir(path) {
                self.1.insert(0, dir);
            }
            return self.next();
        }
        match &self.0 {
            MatchPattern::Literal(lit)
                if lit
                    == &path
                        .strip_prefix(env::current_dir().unwrap())
                        .unwrap()
                        .display()
                        .to_string() =>
            {
                Some(entry)
            }
            MatchPattern::Regex(re)
                if re.is_match(
                    &path
                        .strip_prefix(env::current_dir().unwrap())
                        .unwrap()
                        .display()
                        .to_string(),
                ) =>
            {
                Some(entry)
            }

            MatchPattern::List(_) => unsafe { core::hint::unreachable_unchecked() },
            _ => self.next(),
        }
    }
}

pub struct MatchPatternIter(Option<MatchPattern>);

impl IntoIterator for MatchPattern {
    type Item = Self;
    type IntoIter = MatchPatternIter;
    fn into_iter(self) -> Self::IntoIter {
        MatchPatternIter(Some(self))
    }
}

impl Iterator for MatchPatternIter {
    type Item = MatchPattern;
    fn next(&mut self) -> Option<Self::Item> {
        match self.0.take() {
            None => None,
            Some(p @ MatchPattern::Literal(_)) | Some(p @ MatchPattern::Regex(_)) => Some(p),
            Some(MatchPattern::List(list)) if list.is_empty() => None,
            Some(MatchPattern::List(mut list)) => {
                let ret = list.pop();
                self.0 = Some(MatchPattern::List(list));
                ret
            }
        }
    }
}
