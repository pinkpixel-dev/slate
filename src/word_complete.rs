//! Word completion from the text around the cursor and other open tabs: the
//! ghost text Tab accepts.

/// Shortest typed prefix that gets a suggestion, so one letter doesn't
/// flash a guess on every keystroke.
const MIN_PREFIX: usize = 2;

/// The rest of a word that starts with the word being typed at `cursor` (a
/// byte offset): the closest one in `text`, or else the first one in `others`
/// (the other open tabs, in order). `None` when nothing fits.
pub fn suggest(text: &str, cursor: usize, others: &[String]) -> Option<String> {
    if !text.is_char_boundary(cursor) || text[cursor..].chars().next().is_some_and(is_word_char) {
        return None; // Mid-word: a suggestion would push the rest of the word along.
    }
    let prefix_start = text[..cursor]
        .char_indices()
        .rev()
        .take_while(|&(_, c)| is_word_char(c))
        .last()
        .map_or(cursor, |(i, _)| i);
    let prefix = &text[prefix_start..cursor];
    if prefix.chars().count() < MIN_PREFIX || prefix.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }

    let completes = |word: &str| word.len() > prefix.len() && word.starts_with(prefix);
    let word = words(text)
        .filter(|&(start, word)| start != prefix_start && completes(word))
        .min_by_key(|&(start, _)| start.abs_diff(cursor))
        .map(|(_, word)| word)
        .or_else(|| others.iter().find_map(|other| words(other).map(|(_, word)| word).find(|word| completes(word))))?;
    Some(word[prefix.len()..].to_string())
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Every run of word characters in `text`, with its byte offset.
fn words(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut chars = text.char_indices().peekable();
    std::iter::from_fn(move || {
        let (start, _) = chars.by_ref().find(|&(_, c)| is_word_char(c))?;
        let mut end = text.len();
        while let Some(&(i, c)) = chars.peek() {
            if !is_word_char(c) {
                end = i;
                break;
            }
            chars.next();
        }
        Some((start, &text[start..end]))
    })
}

#[cfg(test)]
mod tests {
    use super::suggest;

    /// Runs `suggest` with the cursor where `|` is.
    fn at_bar(text: &str) -> Option<String> {
        let cursor = text.find('|').unwrap();
        suggest(&text.replacen('|', "", 1), cursor, &[])
    }

    #[test]
    fn finishes_a_word_from_elsewhere_in_the_text() {
        assert_eq!(at_bar("let counter = 1;\ncou|").as_deref(), Some("nter"));
    }

    #[test]
    fn picks_the_closest_match() {
        let text = "counter_far\n\n\n\ncounter_near cou|";
        assert_eq!(at_bar(text).as_deref(), Some("nter_near"));
        assert_eq!(at_bar("cou| counter_after\n\n\n\ncounter_far").as_deref(), Some("nter_after"));
    }

    #[test]
    fn falls_back_to_other_tabs_after_this_one() {
        let others = ["nothing here".to_string(), "let counter_other = 2;".to_string()];
        assert_eq!(suggest("cou", 3, &others).as_deref(), Some("nter_other"));
        assert_eq!(
            suggest("counter_here cou", 16, &others).as_deref(),
            Some("nter_here"),
            "this tab's words win"
        );
    }

    #[test]
    fn needs_two_letters_and_a_word_end() {
        assert_eq!(at_bar("counter c|"), None);
        assert_eq!(at_bar("counter co|x"), None);
        assert_eq!(at_bar("counter co| x").as_deref(), Some("unter"));
    }

    #[test]
    fn ignores_the_word_being_typed_and_exact_matches() {
        assert_eq!(at_bar("counter|"), None);
        assert_eq!(at_bar("count count|"), None);
    }

    #[test]
    fn is_case_sensitive_and_skips_numbers() {
        assert_eq!(at_bar("Counter cou|"), None);
        assert_eq!(at_bar("1234 12|"), None);
    }

    #[test]
    fn handles_non_ascii_words() {
        assert_eq!(at_bar("crème brûlée cr|").as_deref(), Some("ème"));
        assert_eq!(at_bar("naïve na|").as_deref(), Some("ïve"));
    }
}
