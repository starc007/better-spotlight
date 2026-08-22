/// Subsequence fuzzy matcher with Raycast-style scoring:
/// word-start and consecutive-character matches rank highest.
pub fn score(query: &str, target: &str) -> Option<i32> {
    let query: Vec<char> = query.to_lowercase().chars().collect();
    if query.is_empty() {
        return Some(0);
    }
    let target_lower: Vec<char> = target.to_lowercase().chars().collect();

    let mut total = 0;
    let mut last_match: Option<usize> = None;
    let mut qi = 0;

    for (ti, tc) in target_lower.iter().enumerate() {
        if qi < query.len() && *tc == query[qi] {
            total += match last_match {
                Some(last) if ti == last + 1 => 10,
                _ => 5,
            };

            let prev = if ti == 0 { ' ' } else { target_lower[ti - 1] };
            let is_word_start = ti == 0 || !prev.is_alphanumeric();
            if is_word_start {
                total += 15;
            }

            last_match = Some(ti);
            qi += 1;
        }
    }

    (qi == query.len()).then(|| total - (target.len() as i32 / 4))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_scores_higher_than_partial() {
        assert!(score("safari", "Safari").unwrap() > score("saf", "Safari").unwrap());
    }

    #[test]
    fn subsequence_matches() {
        assert!(score("ff", "Firefox").is_some());
    }

    #[test]
    fn non_matching_query_is_none() {
        assert_eq!(score("xyz", "Safari"), None);
    }

    #[test]
    fn case_insensitive() {
        assert!(score("CHROME", "Google Chrome").is_some());
    }

    #[test]
    fn word_start_beats_middle_match() {
        assert!(score("ch", "Chrome").unwrap() > score("ro", "Chrome").unwrap());
    }

    #[test]
    fn empty_query_matches_everything_neutrally() {
        assert_eq!(score("", "Anything"), Some(0));
    }
}
