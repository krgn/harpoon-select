/// Scores `target` against `query` as a case-insensitive subsequence match.
///
/// Returns `None` if `query` is not a subsequence of `target`. Higher scores
/// indicate a tighter/more relevant match (consecutive runs and matches at
/// word boundaries score higher; matches starting later in the string score
/// slightly lower).
pub fn fuzzy_score(query: &str, target: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }

    let query_chars: Vec<char> = query.chars().collect();
    let target_chars: Vec<char> = target.chars().collect();

    let mut score = 0i32;
    let mut target_idx = 0usize;
    let mut last_matched_idx: Option<usize> = None;
    let mut first_match: Option<usize> = None;

    for &qc in &query_chars {
        let qc_lower = qc.to_ascii_lowercase();
        let mut matched = false;

        while target_idx < target_chars.len() {
            let tc = target_chars[target_idx];
            if tc.to_ascii_lowercase() == qc_lower {
                if first_match.is_none() {
                    first_match = Some(target_idx);
                }
                // Reward runs of consecutive matches; penalize skipped
                // characters between matches so tighter matches always
                // outscore looser ones, regardless of boundary bonuses.
                match last_matched_idx {
                    Some(prev) if target_idx == prev + 1 => score += 15,
                    Some(prev) => score += 5 - (target_idx - prev - 1) as i32,
                    None => score += 15,
                }
                if target_idx == 0 || !target_chars[target_idx - 1].is_alphanumeric() {
                    score += 10;
                }
                last_matched_idx = Some(target_idx);
                target_idx += 1;
                matched = true;
                break;
            }
            target_idx += 1;
        }

        if !matched {
            return None;
        }
    }

    if let Some(pos) = first_match {
        score -= pos as i32;
    }

    Some(score)
}

/// Returns the indices of `items` whose text (produced by `to_text`) matches
/// `query`, ordered from best to worst match. When `query` is empty, returns
/// all indices in their original order.
pub fn fuzzy_filter<T>(items: &[T], query: &str, to_text: impl Fn(&T) -> String) -> Vec<usize> {
    if query.is_empty() {
        return (0..items.len()).collect();
    }

    let mut scored: Vec<(usize, i32)> = items
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| fuzzy_score(query, &to_text(item)).map(|score| (idx, score)))
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    scored.into_iter().map(|(idx, _)| idx).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_matches_everything_in_order() {
        assert_eq!(fuzzy_filter(&["b", "a", "c"], "", |s| s.to_string()), vec![0, 1, 2]);
    }

    #[test]
    fn subsequence_matches() {
        assert!(fuzzy_score("hpn", "harpoon").is_some());
        assert!(fuzzy_score("xyz", "harpoon").is_none());
    }

    #[test]
    fn prefers_tighter_matches() {
        let tight = fuzzy_score("main", "main.rs").unwrap();
        let loose = fuzzy_score("main", "m a i n").unwrap();
        assert!(tight > loose);
    }

    #[test]
    fn filter_orders_best_match_first() {
        let items = ["tests/main.rs", "main.rs", "domain.rs"];
        let result = fuzzy_filter(&items, "main", |s| s.to_string());
        assert_eq!(result[0], 1);
    }
}
