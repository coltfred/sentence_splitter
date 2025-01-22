use lazy_static::lazy_static;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Custom error type for sentence splitter operations
#[derive(Error, Debug)]
pub enum SentenceSplitterError {
    #[error("Invalid language code: {0}")]
    InvalidLanguageCode(String),

    #[error("Non-breaking prefix file not found at path: {0}")]
    PrefixFileNotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
}

/// Type alias for Result with SentenceSplitterError
type Result<T> = std::result::Result<T, SentenceSplitterError>;

lazy_static! {
    static ref QUESTION_EXCLAMATION_RE: Regex = Regex::new("([?!])\\s+([\\p{Lu}])").unwrap();
    static ref MULTI_DOT_RE: Regex = Regex::new("(\\.{2,})\\s+([\\p{Lu}])").unwrap();
    static ref PUNCTUATION_RE: Regex = Regex::new("([.?!])\\s+(['\"(\\[]*[\\p{Lu}])").unwrap();
    static ref CLEANUP_SPACES: Regex = Regex::new("\\s{2,}").unwrap();
    static ref SENTENCE_END_PATTERN: Regex =
        Regex::new("([\\w\\.\\-]*)([\'\"\\)\\]%]*?)(\\.+)$").unwrap();
    static ref ACRONYM_RE: Regex = Regex::new(r"(?:^|\s)\.?[\p{Lu}\-]+\.").unwrap();
    static ref STARTS_UPPER_RE: Regex = Regex::new("^(\\s*[\'\"\\(\\[]*\\s*[\\p{Lu}0-9])").unwrap();
    static ref CLEANUP_NEWLINE_START: Regex = Regex::new("\\n\\s+").unwrap();
    static ref CLEANUP_NEWLINE_END: Regex = Regex::new("\\s+\\n").unwrap();
    static ref NUMERIC_ONLY_RE: Regex = Regex::new("^[0-9]+").unwrap();
    pub static ref NON_BREAKING_PREFIXES: HashMap<String, String> = {
        {
            let prefixes = [
                ("ca", include_str!("non_breaking_prefixes/ca.txt")),
                ("cs", include_str!("non_breaking_prefixes/cs.txt")),
                ("da", include_str!("non_breaking_prefixes/da.txt")),
                ("de", include_str!("non_breaking_prefixes/de.txt")),
                ("el", include_str!("non_breaking_prefixes/el.txt")),
                ("en", include_str!("non_breaking_prefixes/en.txt")),
                ("es", include_str!("non_breaking_prefixes/es.txt")),
                ("fi", include_str!("non_breaking_prefixes/fi.txt")),
                ("fr", include_str!("non_breaking_prefixes/fr.txt")),
                ("hu", include_str!("non_breaking_prefixes/hu.txt")),
                ("is", include_str!("non_breaking_prefixes/is.txt")),
                ("it", include_str!("non_breaking_prefixes/it.txt")),
                ("lt", include_str!("non_breaking_prefixes/lt.txt")),
                ("lv", include_str!("non_breaking_prefixes/lv.txt")),
                ("nl", include_str!("non_breaking_prefixes/nl.txt")),
                ("no", include_str!("non_breaking_prefixes/no.txt")),
                ("pl", include_str!("non_breaking_prefixes/pl.txt")),
                ("pt", include_str!("non_breaking_prefixes/pt.txt")),
                ("ro", include_str!("non_breaking_prefixes/ro.txt")),
                ("ru", include_str!("non_breaking_prefixes/ru.txt")),
                ("sk", include_str!("non_breaking_prefixes/sk.txt")),
                ("sl", include_str!("non_breaking_prefixes/sl.txt")),
                ("sv", include_str!("non_breaking_prefixes/sv.txt")),
                ("tr", include_str!("non_breaking_prefixes/tr.txt")),
            ];

            prefixes
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        }
    };
}

/// Enum representing the type of non-breaking prefix
#[derive(Debug, Clone, PartialEq)]
pub enum PrefixType {
    Default,
    NumericOnly,
}

/// Main struct for splitting text into sentences
pub struct SentenceSplitter {
    /// Dictionary of non-breaking prefixes; keys are string prefixes, values are PrefixType enums
    non_breaking_prefixes: HashMap<String, PrefixType>,
}

fn is_closing_punctuation(c: char) -> bool {
    matches!(
        c,
        ')' | ']'
            | '}'
            | '"'
            | '\'' // Smart quote
            | '»'
            | '›'
            | '」'
            | '』'
            | '〉'
            | '》'
            | '】'
            | '〕'
            | '｣'
    )
}

fn is_sentence_starter(c: char) -> bool {
    c.is_uppercase()
        || c == '"'
        || c == '('
        || c.is_numeric()
        || c == '«'  // Add guillemet
        || c == '¿'  // Spanish/Portuguese question mark
        || c == '¡'  // Spanish/Portuguese exclamation mark
        || c == '"'  // Smart quote
        || c == 0x27 as char // Smart quote
        || c == '‹'  // Single guillemet
        || c == '「' // CJK quote
        || c == '『' // CJK quote
}

impl SentenceSplitter {
    /// Create a new SentenceSplitter instance
    ///
    /// # Arguments
    /// * `language` - ISO 639-1 language code
    /// * `non_breaking_prefix_file` - Optional path to non-breaking prefix file
    pub fn new<P: AsRef<Path>>(
        language: &str,
        non_breaking_prefix_file: Option<P>,
    ) -> Result<Self> {
        // Validate language code
        let lang_regex = Regex::new(r"^[a-z][a-z]$").unwrap();
        if !lang_regex.is_match(language) {
            return Err(SentenceSplitterError::InvalidLanguageCode(
                language.to_string(),
            ));
        }

        let mut non_breaking_prefixes: HashMap<String, PrefixType> = HashMap::new();
        let non_breaking_prefixes_file_contents = NON_BREAKING_PREFIXES
            .get(language)
            .cloned()
            .unwrap_or_default();

        // Create a reader for the non-breaking prefixes file contents
        let reader = BufReader::new(non_breaking_prefixes_file_contents.as_bytes());
        for line in reader.lines() {
            let line = line?;

            // Skip empty lines and comments
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }

            let prefix_type = if line.contains("#NUMERIC_ONLY#") {
                PrefixType::NumericOnly
            } else {
                PrefixType::Default
            };

            // Remove comments and clean up the line
            let clean_line = line.split('#').next().unwrap_or("").trim().to_string();

            if !clean_line.is_empty() {
                non_breaking_prefixes.insert(clean_line, prefix_type);
            }
        }

        Ok(SentenceSplitter {
            non_breaking_prefixes,
        })
    }

    /// Split text into sentences
    ///
    /// # Arguments
    /// * `text` - Text to be split into individual sentences
    /// Split text into sentences
    pub fn split(&self, text: &str) -> Vec<String> {
        if text.is_empty() {
            return vec![];
        }

        // Normalize spaces first
        let text = CLEANUP_SPACES.replace_all(text, " ");
        let text = text.trim();

        let mut sentences: Vec<String> = Vec::new();
        let mut current = String::new();

        let mut chars: Vec<char> = text.chars().collect();
        chars.push(' '); // Add trailing space for simpler processing

        let mut i = 0;
        let mut in_quotes = false;
        let mut quote_char = ' ';

        while i < chars.len() - 1 {
            let c = chars[i];
            current.push(c);

            // Track quote status
            if c == '"' || c == '"' || c == '"' {
                if !in_quotes {
                    in_quotes = true;
                    quote_char = c;
                } else if c == quote_char {
                    in_quotes = false;
                }
            }

            if c == '.' || c == '?' || c == '!' {
                // Look ahead for sentence boundary
                let mut next_char_idx = i + 1;
                while next_char_idx < chars.len() && chars[next_char_idx].is_whitespace() {
                    next_char_idx += 1;
                }

                // Get the current fragment for analysis
                let current_fragment = current.as_str();

                // Check for acronyms first
                if ACRONYM_RE.is_match(current_fragment) {
                    i += 1;
                    continue;
                }

                let mut should_split = false;
                if next_char_idx < chars.len() {
                    let next_char = chars[next_char_idx];

                    // Handle various splitting conditions
                    should_split = if in_quotes {
                        // Only split on quote end
                        false
                    } else if i > 0 && chars[i - 1] == ')' {
                        // Handle parenthetical endings
                        true
                    } else {
                        // Normal sentence starters
                        next_char.is_uppercase()
                            || next_char == '"'
                            || next_char == '('
                            || next_char.is_numeric()
                            || next_char == '«'
                    };

                    // Check for non-breaking prefixes
                    if should_split {
                        let current_word = current.split_whitespace().last().unwrap_or("");
                        let word_without_dot =
                            current_word.trim_end_matches(|c| c == '.' || c == '!' || c == '?');

                        if self.non_breaking_prefixes.contains_key(word_without_dot) {
                            match self.non_breaking_prefixes.get(word_without_dot).unwrap() {
                                PrefixType::NumericOnly => {
                                    should_split = !chars[next_char_idx].is_numeric();
                                }
                                PrefixType::Default => {
                                    should_split = false;
                                }
                            }
                        }
                    }
                }

                if should_split {
                    sentences.push(current.trim().to_string());
                    current.clear();
                    i = next_char_idx - 1;
                }
            }
            i += 1;
        }

        // Add final sentence if there's content
        if !current.is_empty() {
            sentences.push(current.trim().to_string());
        }

        // Clean up any empty sentences
        sentences.into_iter().filter(|s| !s.is_empty()).collect()
    }
}

/// Split text into sentences (convenience function)
///
/// For better performance, use SentenceSplitter struct directly to avoid reloading
/// non-breaking prefix file on every call.
pub fn split_text_into_sentences<P: AsRef<Path>>(
    text: &str,
    language: &str,
    non_breaking_prefix_file: Option<P>,
) -> Result<Vec<String>> {
    let splitter = SentenceSplitter::new(language, non_breaking_prefix_file)?;
    Ok(splitter.split(text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // #[test]
    // fn test_invalid_language_code() {
    //     let result = SentenceSplitter::new("/etc/passwd", None::<PathBuf>);
    //     assert!(matches!(
    //         result,
    //         Err(SentenceSplitterError::InvalidLanguageCode(_))
    //     ));
    // }

    // #[test]
    // fn test_unsupported_language() {
    //     let result = SentenceSplitter::new("xx", None::<PathBuf>);
    //     // Should succeed but with empty prefix list
    //     assert!(result.is_ok());
    // }

    // #[test]
    // fn test_text_empty() {
    //     let splitter = SentenceSplitter::new("en", None::<PathBuf>).unwrap();
    //     assert_eq!(splitter.split(""), Vec::<String>::new());
    // }

    #[test]
    fn test_en() {
        let splitter = SentenceSplitter::new("en", None::<PathBuf>).unwrap();

        // Test case 1
        let input_text =
            "This is a paragraph. It contains several sentences. \"But why,\" you ask?";
        let expected_sentences = vec![
            "This is a paragraph.",
            "It contains several sentences.",
            "\"But why,\" you ask?",
        ];
        assert_eq!(splitter.split(input_text), expected_sentences);

        // Test case 2
        let input_text = "Hey! Now.";
        let expected_sentences = vec!["Hey!", "Now."];
        assert_eq!(splitter.split(input_text), expected_sentences);

        // Test case 3
        let input_text = "Hey... Now.";
        let expected_sentences = vec!["Hey...", "Now."];
        assert_eq!(splitter.split(input_text), expected_sentences);

        // Test case 4
        let input_text = "Hey. Now.";
        let expected_sentences = vec!["Hey.", "Now."];
        assert_eq!(splitter.split(input_text), expected_sentences);

        // Test case 5
        let input_text = "Hey.  Now.";
        let expected_sentences = vec!["Hey.", "Now."];
        assert_eq!(splitter.split(input_text), expected_sentences);
    }

    #[test]
    fn test_en_numeric_only() {
        let splitter = SentenceSplitter::new("en", None::<PathBuf>).unwrap();
        let input_text = "Hello. No. 1. No. 2. Prefix. 1. Prefix. 2. Good bye.";
        let expected_sentences = vec![
            "Hello.",
            "No. 1.",
            "No. 2.",
            "Prefix.",
            "1.",
            "Prefix.",
            "2.",
            "Good bye.",
        ];
        assert_eq!(splitter.split(input_text), expected_sentences);
    }

    #[test]
    fn test_en_uppercase_acronym() {
        let splitter = SentenceSplitter::new("en", None::<PathBuf>).unwrap();
        let input_text = "Hello. .NATO. Good bye.";
        let expected_sentences = vec!["Hello. .NATO. Good bye."];
        assert_eq!(splitter.split(input_text), expected_sentences);
    }

    #[test]
    fn test_en_sentence_within_brackets() {
        let splitter = SentenceSplitter::new("en", None::<PathBuf>).unwrap();
        let input_text = "Foo bar. (Baz foo.) Bar baz.";
        let expected_sentences = vec!["Foo bar.", "(Baz foo.)", "Bar baz."];
        assert_eq!(splitter.split(input_text), expected_sentences);
    }

    #[test]
    fn test_de() {
        let splitter = SentenceSplitter::new("de", None::<PathBuf>).unwrap();
        let input_text = "Nie hätte das passieren sollen. Dr. Soltan sagte: \"Der Fluxcompensator war doch kalibriert!\".";
        let expected_sentences = vec![
            "Nie hätte das passieren sollen.",
            "Dr. Soltan sagte: \"Der Fluxcompensator war doch kalibriert!\".",
        ];
        assert_eq!(splitter.split(input_text), expected_sentences);
    }

    #[test]
    fn test_fr() {
        let splitter = SentenceSplitter::new("fr", None::<PathBuf>).unwrap();
        let input_text = "Brookfield Office Properties Inc. (« BOPI »), dont les actifs liés aux immeubles directement...";
        let expected_sentences = vec![input_text];
        assert_eq!(splitter.split(input_text), expected_sentences);
    }

    #[test]
    fn test_el() {
        let splitter = SentenceSplitter::new("el", None::<PathBuf>).unwrap();
        let input_text = "Όλα τα συστήματα ανώτατης εκπαίδευσης σχεδιάζονται σε εθνικό επίπεδο. Η ΕΕ αναλαμβάνει κυρίως να συμβάλει στη βελτίωση της συγκρισιμότητας μεταξύ των διάφορων συστημάτων και να βοηθά φοιτητές και καθηγητές να μετακινούνται με ευκολία μεταξύ των συστημάτων των κρατών μελών.";
        let expected_sentences = vec![
            "Όλα τα συστήματα ανώτατης εκπαίδευσης σχεδιάζονται σε εθνικό επίπεδο.",
            "Η ΕΕ αναλαμβάνει κυρίως να συμβάλει στη βελτίωση της συγκρισιμότητας μεταξύ των διάφορων συστημάτων και να βοηθά φοιτητές και καθηγητές να μετακινούνται με ευκολία μεταξύ των συστημάτων των κρατών μελών."
        ];
        assert_eq!(splitter.split(input_text), expected_sentences);
    }

    #[test]
    fn test_pt() {
        let splitter = SentenceSplitter::new("pt", None::<PathBuf>).unwrap();
        let input_text = "Isto é um parágrafo. Contém várias frases. «Mas porquê,» perguntas tu?";
        let expected_sentences = vec![
            "Isto é um parágrafo.",
            "Contém várias frases.",
            "«Mas porquê,» perguntas tu?",
        ];
        assert_eq!(splitter.split(input_text), expected_sentences);
    }

    #[test]
    fn test_es() {
        let splitter = SentenceSplitter::new("es", None::<PathBuf>).unwrap();
        let input_text = "La UE ofrece una gran variedad de empleos en un entorno multinacional y multilingüe. La Oficina Europea de Selección de Personal (EPSO) se ocupa de la contratación, sobre todo mediante oposiciones generales.";
        let expected_sentences = vec![
            "La UE ofrece una gran variedad de empleos en un entorno multinacional y multilingüe.",
            "La Oficina Europea de Selección de Personal (EPSO) se ocupa de la contratación, sobre todo mediante oposiciones generales."
        ];
        assert_eq!(splitter.split(input_text), expected_sentences);
    }

    // #[test]
    // fn test_custom_non_breaking_prefixes() {
    //     let mut temp_file = NamedTempFile::new().unwrap();
    //     writeln!(
    //         temp_file,
    //         "# \n# Temporary prefix file\n# \n\nPrefix1\nPrefix2\n"
    //     )
    //     .unwrap();

    //     let splitter = SentenceSplitter::new("xx", Some(temp_file.path())).unwrap();
    //     let input_text = "Hello. Prefix1. Prefix2. Hello again. Good bye.";
    //     let expected_sentences = vec!["Hello.", "Prefix1. Prefix2. Hello again.", "Good bye."];
    //     assert_eq!(splitter.split(input_text), expected_sentences);
    // }

    #[test]
    fn test_split_text_into_sentences() {
        let input_text =
            "This is a paragraph. It contains several sentences. \"But why,\" you ask?";
        let expected_sentences = vec![
            "This is a paragraph.",
            "It contains several sentences.",
            "\"But why,\" you ask?",
        ];
        let result = split_text_into_sentences(input_text, "en", None::<PathBuf>).unwrap();
        assert_eq!(result, expected_sentences);
    }
}
