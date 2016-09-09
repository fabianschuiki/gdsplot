// Copyright (c) 2016 Fabian Schuiki

//! A simple parser for command files.

use std;
use std::io;

pub type ByteIter = Iterator<Item = Result<u8, io::Error>>;

pub struct Parser {
	reader: Box<ByteIter>,
	cur: u8,
	line: u32,
	column: u32,
	token: Token,
	token_line: u32,
	token_column: u32,
}

impl Parser {
	pub fn new(reader: Box<ByteIter>) -> Result<Parser, Error> {
		let mut p = Parser {
			reader: reader,
			cur: 0,
			line: 0,
			column: 0,
			token: Token::Invalid,
			token_line: 0,
			token_column: 0,
		};
		try!(p.next_byte());
		try!(p.next());
		Ok(p)
	}

	/// Advance the parser to the next byte. Used internally by the next()
	/// function.
	fn next_byte(&mut self) -> Result<(), Error> {
		if self.cur == '\n' as u8 {
			self.line += 1;
			self.column = 0;
		} else {
			self.column += 1;
		}
		match self.reader.next() {
			Some(Ok(x)) => self.cur = x,
			Some(Err(e)) => return Err(Error::Io(e)),
			None => self.cur = 0,
		}
		Ok(())
	}

	/// Advance the parser to the next token.
	pub fn next(&mut self) -> Result<(), Error> {
		loop {
			// Skip whitespace.
			while self.cur != 0 && (self.cur as char).is_whitespace() {
				try!(self.next_byte());
			}

			// Skip comment lines.
			if self.cur == '#' as u8 {
				while self.cur != 0 && self.cur != '\n' as u8 {
					try!(self.next_byte());
				}
				continue;
			}

			self.token_line = self.line;
			self.token_column = self.column;
			if self.cur == 0 {
				self.token = Token::Eof;
				return Ok(());
			}

			// Quoted Text
			if self.cur == '"' as u8 {
				let mut buffer = Vec::new();
				try!(self.next_byte());
				while self.cur != 0 && self.cur != '\"' as u8 {
					if self.cur == '\\' as u8 {
						try!(self.next_byte());
					}
					buffer.push(self.cur);
					try!(self.next_byte());
				}
				if self.cur != '"' as u8 {
					return Err(Error::Syntax(String::from("expected closing `\"`"), self.line, self.column));
				}
				self.token = Token::Str(match String::from_utf8(buffer) {
					Ok(s) => s.into_boxed_str(),
					Err(e) => return Err(Error::Utf8(e, self.token_line, self.token_column)),
				});
				return Ok(());
			}

			// Identifiers
			let mut buffer = Vec::new();
			while self.cur != 0 && (self.cur as char).is_alphanumeric() {
				buffer.push(self.cur);
				try!(self.next_byte());
			}
			if buffer.len() > 0 {
				let s = match String::from_utf8(buffer) {
					Ok(s) => s.into_boxed_str(),
					Err(e) => return Err(Error::Utf8(e, self.token_line, self.token_column)),
				};

				if let Ok(v) = s.parse() as Result<f64,_> {
					self.token = Token::Real(v);
				} else if let Ok(v) = s.parse() as Result<i64,_> {
					self.token = Token::Int(v);
				} else {
					self.token = Token::Str(s);
				}

				return Ok(());
			}

			return Err(Error::Syntax(format!("unknown token `{}`", self.cur as char), self.line, self.column));
		}
	}

	pub fn eat(&mut self, s: &str) -> Result<bool, Error> {
		if self.check(s) {
			try!(self.next());
			Ok(true)
		} else {
			Ok(false)
		}
	}

	pub fn check(&self, s: &str) -> bool {
		match self.token {
			Token::Str(ref t) => (&**t == s),
			_ => false,
		}
	}
}


pub enum Error {
	Io(std::io::Error),
	Utf8(std::string::FromUtf8Error,u32,u32),
	Syntax(String,u32,u32),
}


pub enum Token {
	Invalid,
	Eof,
	Str(Box<str>),
	Int(i64),
	Real(f64),
}
