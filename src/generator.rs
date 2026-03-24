use rand::{Rng, thread_rng};

use crate::model::SearchOptions;
use crate::validation::ALLOWED_CHARS;

#[derive(Clone, Debug)]
pub struct CandidateGenerator {
    prefix: String,
    remaining_len: usize,
    charset: Vec<char>,
    next_index: u64,
    total_space: u64,
    start_index: u64,
    step: u64,
}

impl CandidateGenerator {
    pub fn new(options: &SearchOptions) -> Self {
        let remaining_len = options.remaining_len();
        let charset: Vec<char> = ALLOWED_CHARS.iter().map(|byte| char::from(*byte)).collect();
        let total_space = pow_u64(charset.len() as u64, remaining_len as u32);

        if total_space <= 1 {
            return Self::from_permutation(options, charset, total_space, 0, 1);
        }

        let mut rng = thread_rng();
        let start_index = rng.gen_range(0..total_space);
        let step = random_step(total_space, &mut rng);

        Self::from_permutation(options, charset, total_space, start_index, step)
    }

    fn from_permutation(
        options: &SearchOptions,
        charset: Vec<char>,
        total_space: u64,
        start_index: u64,
        step: u64,
    ) -> Self {
        debug_assert!(total_space <= 1 || gcd(step, total_space) == 1);
        let remaining_len = options.remaining_len();

        Self {
            prefix: options.prefix.clone(),
            remaining_len,
            charset,
            next_index: 0,
            total_space,
            start_index,
            step,
        }
    }

    pub fn total_space(&self) -> u64 {
        self.total_space
    }

    fn suffix_for(&self, mut index: u64) -> String {
        if self.remaining_len == 0 {
            return String::new();
        }

        let base = self.charset.len() as u64;
        let mut chars = vec![self.charset[0]; self.remaining_len];

        for slot in (0..self.remaining_len).rev() {
            chars[slot] = self.charset[(index % base) as usize];
            index /= base;
        }

        chars.into_iter().collect()
    }
}

impl Iterator for CandidateGenerator {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_index >= self.total_space {
            return None;
        }

        let current = if self.total_space <= 1 {
            0
        } else {
            ((self.start_index as u128 + self.next_index as u128 * self.step as u128)
                % self.total_space as u128) as u64
        };
        self.next_index += 1;
        Some(format!("{}{}", self.prefix, self.suffix_for(current)))
    }
}

fn pow_u64(base: u64, exponent: u32) -> u64 {
    let mut result = 1_u64;
    for _ in 0..exponent {
        result = result
            .checked_mul(base)
            .expect("generator search space fits into u64");
    }
    result
}

fn random_step(total_space: u64, rng: &mut impl Rng) -> u64 {
    loop {
        let candidate = rng.gen_range(2..total_space);
        if gcd(candidate, total_space) == 1 {
            return candidate;
        }
    }
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }

    left
}

#[cfg(test)]
mod tests {
    use crate::validation::{ALLOWED_CHARS, validate_search_options};

    use super::CandidateGenerator;

    #[test]
    fn yields_exact_prefix_once() {
        let options = validate_search_options(4, "test", 1, 5).expect("valid options");
        let mut generator = CandidateGenerator::new(&options);
        assert_eq!(generator.next().as_deref(), Some("test"));
        assert_eq!(generator.next(), None);
    }

    #[test]
    fn generates_unique_names_of_correct_length() {
        let options = validate_search_options(4, "e", 5, 20).expect("valid options");
        let charset: Vec<char> = ALLOWED_CHARS.iter().map(|byte| char::from(*byte)).collect();
        let total_space = super::pow_u64(charset.len() as u64, options.remaining_len() as u32);
        let generator = CandidateGenerator::from_permutation(&options, charset, total_space, 5, 2);
        let names: Vec<String> = generator.take(5).collect();

        assert_eq!(names.len(), 5);
        assert!(names.iter().all(|name| name.len() == 4));
        assert_eq!(
            names.iter().collect::<std::collections::HashSet<_>>().len(),
            5
        );
        assert_eq!(names[0], "eaaf");
        assert_eq!(names[1], "eaah");
        assert_ne!(names[0], "eaaa");
    }
}
