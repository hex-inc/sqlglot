use crate::trie::{Trie, TrieResult};
use crate::{Token, TokenType, TokenizerSettings};
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use std::panic::catch_unwind;

#[derive(Debug)]
#[pyclass]
pub struct Tokenizer {
    settings: TokenizerSettings,
    keyword_trie: Trie,
}

#[pymethods]
impl Tokenizer {
    #[new]
    pub fn new(settings: TokenizerSettings) -> Tokenizer {
        let mut keyword_trie = Trie::new();
        let single_token_strs: Vec<String> = settings
            .single_tokens
            .keys()
            .map(|s| s.to_string())
            .collect();
        let trie_filter =
            |key: &&String| key.contains(" ") || single_token_strs.iter().any(|t| key.contains(t));

        keyword_trie.add(settings.keywords.keys().filter(trie_filter));
        keyword_trie.add(settings.comments.keys().filter(trie_filter));
        keyword_trie.add(settings.quotes.keys().filter(trie_filter));
        keyword_trie.add(settings.format_strings.keys().filter(trie_filter));

        Tokenizer {
            settings,
            keyword_trie,
        }
    }

    pub fn tokenize(&self, sql: &str) -> Result<Vec<Token>, PyErr> {
        catch_unwind(|| {
            let mut state = TokenizerState::new(sql, &self.settings, &self.keyword_trie);
            state.tokenize()
        })
        .map_err(|e| PyException::new_err(e.downcast_ref::<&str>().unwrap_or(&"").to_string()))
    }
}

#[derive(Debug)]
struct TokenizerState<'a> {
    sql: Vec<char>,
    size: usize,
    tokens: Vec<Token>,
    start: usize,
    current: usize,
    line: usize,
    column: usize,
    comments: Vec<String>,
    is_end: bool,
    current_char: char,
    peek_char: char,
    previous_token_line: Option<usize>,
    keyword_trie: &'a Trie,
    settings: &'a TokenizerSettings,
}

impl<'a> TokenizerState<'a> {
    fn new(
        sql: &str,
        settings: &'a TokenizerSettings,
        keyword_trie: &'a Trie,
    ) -> TokenizerState<'a> {
        let sql_vec = sql.chars().collect::<Vec<char>>();
        let sql_vec_len = sql_vec.len();
        TokenizerState {
            sql: sql_vec,
            size: sql_vec_len,
            tokens: Vec::new(),
            start: 0,
            current: 0,
            line: 1,
            column: 0,
            comments: Vec::new(),
            is_end: false,
            current_char: '\0',
            peek_char: '\0',
            previous_token_line: None,
            keyword_trie,
            settings,
        }
    }

    fn tokenize(&mut self) -> Vec<Token> {
        self.scan(None);
        std::mem::replace(&mut self.tokens, Vec::new())
    }

    fn scan(&mut self, until_peek_char: Option<char>) {
        while self.size > 0 && !self.is_end {
            self.start = self.current;
            self.advance(1, false);

            if self.current_char == '\0' {
                break;
            }

            if !self.settings.white_space.contains_key(&self.current_char) {
                if self.current_char.is_digit(10) {
                    self.scan_number();
                } else if let Some(identifier_end) =
                    self.settings.identifiers.get(&self.current_char)
                {
                    self.scan_identifier(&identifier_end.to_string());
                } else {
                    self.scan_keyword();
                }
            }

            if let Some(c) = until_peek_char {
                if self.peek_char == c {
                    break;
                }
            }
        }
        if !self.tokens.is_empty() && !self.comments.is_empty() {
            self.tokens
                .last_mut()
                .unwrap()
                .append_comments(&mut self.comments);
        }
    }

    fn advance(&mut self, i: isize, alnum: bool) {
        let mut i = i;
        if let Some(TokenType::BREAK) = self.settings.white_space.get(&self.current_char) {
            // Ensures we don't count an extra line if we get a \r\n line break sequence.
            if self.current_char == '\r' && self.peek_char == '\n' {
                i = 2;
            }

            self.column = 1;
            self.line += 1;
        } else {
            self.column = self.column.wrapping_add_signed(i);
        }

        self.current = self.current.wrapping_add_signed(i);
        self.is_end = self.current >= self.size;
        self.current_char = self.char_at(self.current - 1);
        self.peek_char = if self.is_end {
            '\0'
        } else {
            self.char_at(self.current)
        };

        if alnum && self.current_char.is_alphanumeric() {
            while self.peek_char.is_alphanumeric() {
                self.column += 1;
                self.current += 1;
                self.is_end = self.current >= self.size;
                self.peek_char = if self.is_end {
                    '\0'
                } else {
                    self.char_at(self.current)
                };
            }
            self.current_char = self.char_at(self.current - 1);
        }
    }

    fn peek(&self, i: usize) -> char {
        let index = self.current + i;
        if index < self.size {
            self.char_at(index)
        } else {
            '\0'
        }
    }

    fn chars(&self, size: usize) -> String {
        let start = self.current - 1;
        let end = start + size;
        if end <= self.size {
            self.sql[start..end].iter().collect()
        } else {
            String::from("")
        }
    }

    fn char_at(&self, index: usize) -> char {
        *self.sql.get(index).unwrap()
    }

    fn text(&self) -> String {
        self.sql[self.start..self.current].iter().collect()
    }

    fn add(&mut self, token_type: TokenType, text: Option<String>) {
        self.previous_token_line = Some(self.line);

        if !self.comments.is_empty()
            && !self.tokens.is_empty()
            && token_type == TokenType::SEMICOLON
        {
            self.tokens
                .last_mut()
                .unwrap()
                .append_comments(&mut self.comments);
        }

        self.tokens.push(Token::new(
            token_type,
            text.unwrap_or(self.text()),
            self.line,
            self.column,
            self.start,
            self.current - 1,
            std::mem::replace(&mut self.comments, Vec::new()),
        ));

        // If we have either a semicolon or a begin token before the command's token, we'll parse
        // whatever follows the command's token as a string.
        if self.settings.commands.contains(&token_type)
            && self.peek_char != ';'
            && (self.tokens.len() == 1
                || self
                    .settings
                    .command_prefix_tokens
                    .contains(&self.tokens[self.tokens.len() - 2].token_type))
        {
            let start = self.current;
            let tokens_len = self.tokens.len();
            self.scan(Some(';'));
            self.tokens.truncate(tokens_len);
            let text = self.sql[start..self.current]
                .iter()
                .collect::<String>()
                .trim()
                .to_string();
            if !text.is_empty() {
                self.add(TokenType::STRING, Some(text));
            }
        }
    }

    fn scan_keyword(&mut self) {
        let mut size: usize = 0;
        let mut word: Option<String> = None;
        let mut chars = self.text();
        let mut current_char = '\0';
        let mut prev_space = false;
        let mut skip = false;
        let mut is_single_token = chars.len() == 1
            && self
                .settings
                .single_tokens
                .contains_key(&chars.chars().next().unwrap());

        let (mut trie_result, mut trie_node) =
            self.keyword_trie.root.contains(&chars.to_uppercase());

        while !chars.is_empty() {
            match trie_result {
                TrieResult::Failed => break,
                TrieResult::Exists => word = Some(chars.clone()),
                _ => {}
            }

            let end = self.current + size;
            size += 1;

            if end < self.size {
                current_char = self.char_at(end);
                is_single_token =
                    is_single_token || self.settings.single_tokens.contains_key(&current_char);
                let is_space = self.settings.white_space.contains_key(&current_char);

                if !is_space || !prev_space {
                    if is_space {
                        current_char = ' ';
                    }
                    chars.push(current_char);
                    prev_space = is_space;
                    skip = false;
                } else {
                    skip = true;
                }
            } else {
                current_char = '\0';
                chars = String::from(" ");
            }

            if skip {
                trie_result = TrieResult::Prefix;
            } else {
                (trie_result, trie_node) =
                    trie_node.contains(&current_char.to_uppercase().collect::<String>());
            }
        }

        if word.is_some() {
            let unwrapped_word = word.unwrap();
            if self.scan_string(&unwrapped_word) {
                return;
            }
            if self.scan_comment(&unwrapped_word) {
                return;
            }
            if prev_space || is_single_token || current_char == '\0' {
                self.advance((size - 1) as isize, false);
                let normalized_word = unwrapped_word.to_uppercase();
                self.add(
                    *self.settings.keywords.get(&normalized_word).unwrap(),
                    Some(unwrapped_word),
                );
                return;
            }
        }

        match self.settings.single_tokens.get(&self.current_char) {
            Some(token_type) => self.add(*token_type, Some(self.current_char.to_string())),
            None => self.scan_var(),
        }
    }

    fn scan_comment(&mut self, comment_start: &str) -> bool {
        if !self.settings.comments.contains_key(comment_start) {
            return false;
        }

        let comment_start_line = self.line;
        let comment_start_size = comment_start.len();

        if let Some(comment_end) = self.settings.comments.get(comment_start).unwrap() {
            // Skip the comment's start delimiter.
            self.advance(comment_start_size as isize, false);

            let comment_end_size = comment_end.len();

            while !self.is_end && self.chars(comment_end_size) != *comment_end {
                self.advance(1, true);
            }

            let text = self.text();
            self.comments
                .push(text[comment_start_size..text.len() - comment_end_size + 1].to_string());
            self.advance((comment_end_size - 1) as isize, false);
        } else {
            while !self.is_end
                && self.settings.white_space.get(&self.peek_char) != Some(&TokenType::BREAK)
            {
                self.advance(1, true);
            }
            self.comments
                .push(self.text()[comment_start_size..].to_string());
        }

        // Leading comment is attached to the succeeding token, whilst trailing comment to the preceding.
        // Multiple consecutive comments are preserved by appending them to the current comments list.
        if Some(comment_start_line) == self.previous_token_line {
            self.tokens
                .last_mut()
                .unwrap()
                .append_comments(&mut self.comments);
            self.previous_token_line = Some(self.line);
        }

        true
    }

    fn scan_string(&mut self, start: &String) -> bool {
        let (base, token_type, end) = if let Some(end) = self.settings.quotes.get(start) {
            (None, TokenType::STRING, end.clone())
        } else if self.settings.format_strings.contains_key(start) {
            let (ref end, token_type) = self.settings.format_strings.get(start).unwrap();

            if *token_type == TokenType::HEX_STRING {
                (Some(16), *token_type, end.clone())
            } else if *token_type == TokenType::BIT_STRING {
                (Some(2), *token_type, end.clone())
            } else if *token_type == TokenType::HEREDOC_STRING {
                self.advance(1, false);
                let tag = if self.current_char.to_string() == *end {
                    String::from("")
                } else {
                    self.extract_string(end, false)
                };
                (None, *token_type, format!("{}{}{}", start, tag, end))
            } else {
                (None, *token_type, end.clone())
            }
        } else {
            return false;
        };

        self.advance(start.len() as isize, false);
        let text = self.extract_string(&end, false);

        if let Some(b) = base {
            if u64::from_str_radix(&text, b).is_err() {
                // FIXME: return Result instead.
                panic!(
                    "Numeric string contains invalid characters from {}:{}",
                    self.line, self.start
                );
            }
        } else {
            // FIXME: Encode / decode
        }

        self.add(token_type, Some(text));
        true
    }

    fn scan_number(&mut self) {
        if self.current_char == '0' {
            let peek_char = self.peek_char.to_ascii_uppercase();
            if peek_char == 'B' {
                if self.settings.has_bit_strings {
                    self.scan_bits();
                } else {
                    self.add(TokenType::NUMBER, None);
                }
                return;
            } else if peek_char == 'X' {
                if self.settings.has_hex_strings {
                    self.scan_hex();
                } else {
                    self.add(TokenType::NUMBER, None);
                }
                return;
            }
        }

        let mut decimal = false;
        let mut scientific = 0;

        loop {
            if self.peek_char.is_digit(10) {
                self.advance(1, false);
            } else if self.peek_char == '.' && !decimal {
                let after = self.peek(1);
                if after.is_digit(10) || !after.is_alphabetic() {
                    decimal = true;
                    self.advance(1, false);
                } else {
                    self.add(TokenType::VAR, None);
                    return;
                }
            } else if (self.peek_char == '-' || self.peek_char == '+') && scientific == 1 {
                scientific += 1;
                self.advance(1, false);
            } else if self.peek_char.to_ascii_uppercase() == 'E' && scientific == 0 {
                scientific += 1;
                self.advance(1, false);
            } else if self.peek_char.is_alphabetic() || self.peek_char == '_' {
                let number_text = self.text();
                let mut literal = String::from("");

                while !self.peek_char.is_whitespace()
                    && !self.is_end
                    && !self.settings.white_space.contains_key(&self.peek_char)
                {
                    literal.push(self.peek_char);
                    self.advance(1, false);
                }

                let token_type = self
                    .settings
                    .keywords
                    .get(
                        self.settings
                            .numeric_literals
                            .get(&literal.to_uppercase())
                            .unwrap_or(&String::from("")),
                    )
                    .map(|x| *x);

                if let Some(unwrapped_token_type) = token_type {
                    self.add(TokenType::NUMBER, Some(number_text));
                    self.add(TokenType::DCOLON, Some("::".to_string()));
                    self.add(unwrapped_token_type, Some(literal));
                } else if self.settings.identifiers_can_start_with_digit {
                    self.add(TokenType::VAR, None);
                } else {
                    self.advance(-(literal.len() as isize), false);
                    self.add(TokenType::NUMBER, Some(number_text));
                }
                return;
            } else {
                self.add(TokenType::NUMBER, None);
                return;
            }
        }
    }

    fn scan_bits(&mut self) {
        self.scan_radix_string(2, TokenType::BIT_STRING);
    }

    fn scan_hex(&mut self) {
        self.scan_radix_string(16, TokenType::HEX_STRING);
    }

    fn scan_radix_string(&mut self, radix: u32, radix_token_type: TokenType) {
        self.advance(1, false);
        let value = self.extract_value()[2..].to_string();
        match u32::from_str_radix(&value, radix) {
            Ok(_) => self.add(radix_token_type, Some(value)),
            Err(_) => self.add(TokenType::IDENTIFIER, None),
        }
    }

    fn scan_var(&mut self) {
        loop {
            let peek_char = if !self.peek_char.is_whitespace() {
                self.peek_char
            } else {
                '\0'
            };
            if peek_char != '\0'
                && (self.settings.var_single_tokens.contains(&peek_char)
                    || !self.settings.single_tokens.contains_key(&peek_char))
            {
                self.advance(1, true);
            } else {
                break;
            }
        }

        let token_type = if self.tokens.last().map(|t| t.token_type) == Some(TokenType::PARAMETER) {
            TokenType::VAR
        } else {
            self.settings
                .keywords
                .get(&self.text().to_uppercase())
                .map(|x| *x)
                .unwrap_or(TokenType::VAR)
        };
        self.add(token_type, None);
    }

    fn scan_identifier(&mut self, identifier_end: &str) {
        self.advance(1, false);
        let text = self.extract_string(identifier_end, true);
        self.add(TokenType::IDENTIFIER, Some(text));
    }

    fn extract_string(&mut self, delimiter: &str, use_identifier_escapes: bool) -> String {
        let mut text = String::from("");

        loop {
            let escapes = if use_identifier_escapes {
                &self.settings.identifier_escapes
            } else {
                &self.settings.string_escapes
            };

            let peek_char_str = self.peek_char.to_string();
            if escapes.contains(&self.current_char)
                && (peek_char_str == delimiter || escapes.contains(&self.peek_char))
                && (self.current_char == self.peek_char
                    || !self
                        .settings
                        .quotes
                        .contains_key(&self.current_char.to_string()))
            {
                if peek_char_str == delimiter {
                    text.push(self.peek_char);
                } else {
                    text.push(self.current_char);
                    text.push(self.peek_char);
                }
                if self.current + 1 < self.size {
                    self.advance(2, false);
                } else {
                    // FIXME: use Result instead of panic
                    panic!("Missing {} from {}:{}", delimiter, self.line, self.current);
                }
            } else {
                if self.chars(delimiter.len()) == delimiter {
                    if delimiter.len() > 1 {
                        self.advance((delimiter.len() - 1) as isize, false);
                    }
                    break;
                }
                if self.is_end {
                    // FIXME: use Result instead of panic
                    panic!("Missing {} from {}:{}", delimiter, self.line, self.current);
                }

                if !self.settings.escape_sequences.is_empty()
                    && !self.peek_char.is_whitespace()
                    && self.settings.string_escapes.contains(&self.current_char)
                {
                    let sequence_key = format!("{}{}", self.current_char, self.peek_char);
                    if let Some(escaped_sequence) =
                        self.settings.escape_sequences.get(&sequence_key)
                    {
                        self.advance(2, false);
                        text.push_str(escaped_sequence);
                        continue;
                    }
                }

                let current = self.current - 1;
                self.advance(1, true);
                text.push_str(
                    &self.sql[current..self.current - 1]
                        .iter()
                        .collect::<String>(),
                );
            }
        }
        return text;
    }

    fn extract_value(&mut self) -> String {
        loop {
            if !self.peek_char.is_whitespace()
                && !self.is_end
                && !self.settings.single_tokens.contains_key(&self.peek_char)
            {
                self.advance(1, true);
            } else {
                break;
            }
        }
        self.text()
    }
}