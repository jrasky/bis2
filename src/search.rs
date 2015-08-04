use std::collections::{HashMap, BinaryHeap};
use std::borrow::{Cow, IntoCow};
use std::cmp::Ordering;
use std::iter::FromIterator;

use constants::*;

#[derive(PartialEq, Eq)]
enum CharClass {
    Whitespace,
    Numeric,
    Alphabetic,
    First,
    Other
}

#[derive(Debug)]
pub struct LineInfo {
    line: Cow<'static, str>,
    char_map: HashMap<char, Vec<usize>>,
    heat_map: Vec<isize>,
    factor: isize
}

#[derive(Debug)]
pub struct LineMatch {
    score: isize,
    factor: isize,
    line: Cow<'static, str>
}

#[derive(Debug)]
pub struct SearchBase {
    lines: Vec<LineInfo>
}

impl Ord for LineMatch {
    fn cmp(&self, other: &LineMatch) -> Ordering {
        match self.score.cmp(&other.score) {
            Ordering::Equal => self.factor.cmp(&other.factor),
            order => order
        }
    }
}

impl PartialOrd for LineMatch {
    fn partial_cmp(&self, other: &LineMatch) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for LineMatch {
    fn eq(&self, other: &LineMatch) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for LineMatch {}

impl FromIterator<LineInfo> for SearchBase {
    fn from_iter<T: IntoIterator<Item=LineInfo>>(iterator: T) -> SearchBase {
        SearchBase {
            lines: Vec::from_iter(iterator)
        }
    }
}

impl SearchBase {
    pub fn query<T: AsRef<str>>(&self, query: T) -> Vec<Cow<'static, str>> {
        if query.as_ref().is_empty() {
            // an empty query means don't match anything
            return vec![];
        }

        let mut matches: BinaryHeap<LineMatch> = BinaryHeap::with_capacity(MATCH_NUMBER);

        for item in self.lines.iter() {
            let score = match item.score(&query) {
                None => {
                    // non-matching line
                    continue;
                },
                Some(score) => {
                    score
                }
            };

            let match_item = LineMatch {
                score: -score,
                factor: -item.factor,
                line: item.line.clone()
            };

            if matches.len() < MATCH_NUMBER {
                matches.push(match_item);
            } else if &match_item < matches.peek().unwrap() {
                matches.push_pop(match_item);
            }
        }

        matches.into_sorted_vec().into_iter().map(|x| {x.line}).collect()
    }
}

impl LineInfo {
    pub fn new<T: IntoCow<'static, str>>(item: T, factor: isize) -> LineInfo {
        let mut map: HashMap<char, Vec<usize>> = HashMap::new();
        let mut heat = vec![];
        let line = item.into_cow();

        let mut ws_score = 0isize;
        let mut cs_score = 0isize;
        let mut cur_class = CharClass::First;
        let mut cs_change = false;

        for (idx, c) in line.chars().enumerate() {
            if idx > MAX_LEN {
                break;
            }

            if c.is_whitespace() {
                cur_class = CharClass::Whitespace;
                ws_score = WHITESPACE_FACTOR;
            } else if c.is_numeric() {
                if cur_class != CharClass::Numeric {
                    cur_class = CharClass::Numeric;
                    if !cs_change {
                        cs_score += CLASS_FACTOR;
                        cs_change = true;
                    } else {
                        cs_change = false;
                    }
                }
            } else if c.is_alphabetic() {
                if cur_class != CharClass::Alphabetic {
                    cur_class = CharClass::Numeric;
                    if !cs_change {
                        cs_score += CLASS_FACTOR;
                        cs_change = true;
                    } else {
                        cs_change =  false;
                    }
                }
            } else {
                if cur_class != CharClass::Other {
                    if !cs_change {
                        cs_score += CLASS_FACTOR;
                        cs_change = true;
                    } else {
                        cs_change = false;
                    }
                }
            }

            if cur_class != CharClass::Whitespace {
                map.entry(c).or_insert(Vec::default()).push(idx);
                if c.is_uppercase() {
                    for lc in c.to_lowercase() {
                        map.entry(lc).or_insert(Vec::default()).push(idx);
                    }
                }
            }

            heat.push(ws_score + cs_score);

            ws_score /= WHITESPACE_REDUCE;
            if !cs_change {
                cs_score /= CLASS_REDUCE;
            }
        }

        LineInfo {
            line: line,
            char_map: map,
            heat_map: heat,
            factor: factor
        }
    }

    fn get_positions(&self, item: char, after: usize) -> Option<Vec<usize>> {
        match self.char_map.get(&item) {
            None => None,
            Some(list) => match list.binary_search(&after) {
                Ok(idx) if idx + 1 < list.len() => Some(list[idx + 1..].to_vec()),
                Err(idx) if idx < list.len() => Some(list[idx..].to_vec()),
                _ => None
            }
        }
    }

    fn position_list<T: AsRef<str>>(&self, item: T) -> Option<Vec<Vec<usize>>> {
        let mut positions = vec![];
        let mut last = 0;

        for c in item.as_ref().chars() {
            match self.get_positions(c, last) {
                None => return None,
                Some(list) => {
                    last = match list.get(0) {
                        None => return None,
                        Some(idx) => *idx
                    };
                    positions.push(list);
                }
            }
        }

        Some(positions)
    }

    fn permute_positions(mut list: Vec<Vec<usize>>) -> Option<Vec<Vec<usize>>> {
        let mut result = vec![];
        let mut buffer = vec![];

        match list.pop() {
            None => return None,
            Some(item) => {
                for idx in item {
                    result.push(vec![idx]);
                }
            }
        }

        loop {
            match list.pop() {
                None => break,
                Some(base_list) => {
                    for list in result.iter_mut() {
                        let compare = list.last().unwrap();
                        for item in base_list.iter() {
                            if item < compare {
                                let mut new = base_list.clone();
                                new.push(*item);
                                buffer.push(new);
                            }
                        }
                    }

                    if buffer.is_empty() {
                        return None;
                    }

                    let temp = result;
                    result = buffer;
                    buffer = temp;
                    buffer.clear();
                }
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    fn score_position(&self, position: Vec<usize>) -> isize {
        let avg_dist: usize;

        if position.len() < 2 {
            avg_dist = 0;
        } else {
            avg_dist = position.windows(2).map(|pair| {pair[0] - pair[1]}).sum::<usize>() / position.len();
        }

        let heat_sum: isize = position.iter().map(|idx| {self.heat_map[*idx]}).sum();

        avg_dist as isize * DIST_WEIGHT + heat_sum * HEAT_WEIGHT
    }

    fn best_position(&self, positions: Vec<Vec<usize>>) -> Option<isize> {
        positions.into_iter().map(|position| {self.score_position(position)}).max()
    }

    fn score<T: AsRef<str>>(&self, query: T) -> Option<isize> {
        self.position_list(query)
            .and_then(|positions| {LineInfo::permute_positions(positions)})
            .and_then(|positions| {self.best_position(positions)})
    }
}
