# Text to Sentence Splitter

A Rust implementation of a text-to-sentence splitter using the heuristic algorithm by Philipp Koehn and Josh Schroeder. This library helps split text paragraphs into sentences using punctuation and capitalization clues.

## Features

- Split text into sentences using language-specific rules
- Support for 24 languages including English, French, German, Spanish and more
- Customizable non-breaking prefix rules
- Simple API with both struct and function interfaces

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
text_to_sentence = "0.1.0"  # replace with actual version
```

### Example Code

```rust
use text_to_sentence::{SentenceSplitter, split_text_into_sentences};

// Using the struct interface
let splitter = SentenceSplitter::new("en");
let sentences = splitter.split("This is a paragraph. It contains several sentences. \"But why,\" you ask?");
assert_eq!(sentences, vec![
    "This is a paragraph.",
    "It contains several sentences.",
    "\"But why,\" you ask?"
]);

// Using the functional interface
let sentences = split_text_into_sentences(
    "This is a paragraph. It contains several sentences. \"But why,\" you ask?",
    "en"
);
```

## Supported Languages

The library supports the following languages (specified by their ISO 639-1 codes):

- Catalan (ca)
- Czech (cs)
- Danish (da)
- Dutch (nl)
- English (en)
- Finnish (fi)
- French (fr)
- German (de)
- Greek (el)
- Hungarian (hu)
- Icelandic (is)
- Italian (it)
- Latvian (lv)
- Lithuanian (lt)
- Norwegian (Bokmål) (no)
- Polish (pl)
- Portuguese (pt)
- Romanian (ro)
- Russian (ru)
- Slovak (sk)
- Slovene (sl)
- Spanish (es)
- Swedish (sv)
- Turkish (tr)

## Current Status

The implementation is functional but has some known issues:

- Handling of uppercase acronyms needs improvement
- Sentence detection within brackets requires fixes
- All core language support tests passing except for specific edge cases

## License

This project is licensed under the GNU Lesser General Public License v3.0 or later.

Copyright notices:
- Original algorithm: © 2005 Philip Koehn and Josh Schroeder
- Python implementation: © 2010 Digital Silk Road, © 2017 Linas Valiukas
- Rust implementation: 2025 [mrorigo](mrorigo+githubspam@gmail.com)

## Contributing

Contributions are welcome! Current areas that need attention:
1. Fix uppercase acronym handling
2. Improve sentence detection within brackets
3. Add more test cases
4. Enhance language support

Please ensure tests are added for any new features or bug fixes.
