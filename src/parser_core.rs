use std::collections::BTreeMap;
use std::fmt::{Display, Formatter, Write};
use std::rc::Rc;

use crate::*;

const NULL: &[u8] = "null".as_bytes();
const TRUE: &[u8] = "true".as_bytes();
const FALSE: &[u8] = "false".as_bytes();

//region FixedSizeArray
struct FixedSizeArray {
    capacity: usize,
    pos: usize,
    arr: Vec<u8>,
}

impl FixedSizeArray {
    fn new(capacity: usize) -> Self {
        let mut a = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            a.push(b' ');
        }
        FixedSizeArray {
            pos: 0,
            capacity,
            arr: a,
        }
    }

    fn push(&mut self, byte: u8) {
        self.pos = (self.pos + 1) % self.capacity;
        self.arr[self.pos] = byte;
    }
}

impl Display for FixedSizeArray {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut container = Vec::with_capacity(self.capacity);
        let s = self.pos + 1;
        for i in s..self.capacity { container.push(self.arr[i]); }
        for i in 0..s { container.push(self.arr[i]); }
        f.write_str(&String::from_utf8_lossy(&container))
    }
}
//endregion

//region pubs including Parser, Content, PathItem, ValueType
pub struct Parser {
    reader: Box<dyn Iterator<Item=u8>>,
    pub next_byte: u8,
    txt: FixedSizeArray,
    next_fn: fn(&mut Parser) -> u8,
    pub stack: Vec<StackItem>,
}

impl Parser {
    /// To read
    /// **reader** is the mechanism to access bytes of the json
    /// **mem_size** determines the size of a buffer which is responsible to hold a copy of most recent bytes, so there would be an extra u8 copy operation.
    /// If you need to preview the most recent piece of json, set mem_size as you wish. Then you can access that by calling get_recent_piece()
    /// ### Example
    /// ```
    /// use json_walker::json_walker::{JsonWalker, StringReader};
    ///
    /// fn main() {
    ///     let mut walker = JsonWalker::new(StringReader::new(r#"{"key":"value"}"#.to_string()), 50);
    ///     loop {
    ///         match walker.next_item() {
    ///             Ok(t) => { println!("{t:?}") }
    ///             Err(_) => { break; }
    ///         }
    ///     }
    /// }
    /// ```
    pub fn new(reader: Box<dyn Iterator<Item=u8>>, mem_size: usize) -> Self {
        let mut stack = Vec::with_capacity(30);
        stack.push(new_colon_stack_item(Rc::new(String::from(ROOT)), -0.5));

        let txt: FixedSizeArray;
        let next_fn: fn(&mut Parser) -> u8;

        if mem_size > 0 {
            txt = FixedSizeArray::new(mem_size);
            next_fn = next_byte_with_memory;
        } else {
            txt = FixedSizeArray::new(1);
            next_fn = next_byte;
        }

        let mut h = Parser {
            reader,
            next_byte: NIL,
            txt,
            next_fn,
            stack,
        };
        next_no_white_space(&mut h);
        h
    }
}

pub type Item = (ValueType, String);

#[derive(Debug, PartialEq)]
pub enum Content {
    Simple(Item),
    Array(Vec<Content>),
    Object(BTreeMap<String, Content>),
}

#[derive(Debug)]
pub enum PathItem {
    Start,
    Object(Rc<String>, usize),
    Array(Rc<String>, usize),
}

impl Display for PathItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PathItem::Start => f.write_char(ROOT),
            PathItem::Object(s, i) => f.write_str(&format!("{}{},{}{}", '{', s, i, '}')),
            PathItem::Array(s, i) => f.write_str(&format!("{}{},{}{}", '[', s, i, ']')),
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum ValueType {
    Null,
    Bool,
    Int,
    Float,
    Str,
    Arr,
    Obj,
}
//endregion

//region Parser controller methods such as next(), error report builder,...

/// when reader returns None, this function gets called
fn on_none_input(parser: &mut Parser) -> u8 {
    if parser.stack.len() > 2 {
        panic!(r#"Unexpected end of stream"#)
    }
    NIL
}

/// call this function when memory size is zero
fn next_byte(parser: &mut Parser) -> u8 {
    parser
        .reader
        .next()
        .unwrap_or_else(|| on_none_input(parser))
}

/// call this function when memory is set
fn next_byte_with_memory(parser: &mut Parser) -> u8 {
    match parser
        .reader
        .next() {
        None => on_none_input(parser),
        Some(b) => {
            parser.txt.push(b);
            b
        }
    }
}

/// return next byte from the reader. No matter if that byte is white-space or not
fn next(parser: &mut Parser) -> u8 {
    // strings start with " and finish with ". so from the iterator view point, it is clear to find out the start and end point.
    // null, true & false are key words and the length of them are fix, so the start and end points are obvious
    // but when it comes to reading numbers, from the iterator aspect view,
    // there is no way to find out when digits finish and it always needs to read bytes till one none digit byte,
    // but that byte is going to be processed in the next loop and we have consume it early.
    // The way to solve that is to walk one byte backward, but iterator does not support that,
    // so it would be better to read a byte and save it for next loop, then process one byte ago.
    // this is why, I am using next_byte. also it comes handy when I want to find out if the stream is finished.
    let c = parser.next_byte;
    parser.next_byte = (parser.next_fn)(parser);
    c
}

/// return next none white-space byte
fn next_no_white_space(parser: &mut Parser) -> u8 {
    let c = parser.next_byte;
    loop {
        parser.next_byte = (parser.next_fn)(parser);
        if !parser.next_byte.is_ascii_whitespace() {
            break;
        }
    }
    c
}

/// get current stack status including latest_key, node level, stack top char, nth occurrence and recent piece of json if memory size is set
pub fn get_current_status(parser: &mut Parser) -> String {
    let t = parser.txt.to_string();
    let l = t.len() - 1;
    if let Some(a) = parser.stack.last() {
        let level = a.level;
        let stack_top = a.symbol;
        let key = format!(r#""{}""#, a.key.clone());
        let nth = a.nth;
        format!("level: {level:<5}, key: {key:<20}, top: '{stack_top}',  nth: {nth:<4},\t\t\x1b[91m{}\x1b[32m{}\x1b[0m{}...", &t[0..l], &t[l..l + 1], &t[l + 1..])
    } else {
        "stack is empty".to_string()
    }
}
/// convert utf8 bytes to string and panic if its not standard utf8 string
fn to_string(v: Vec<u8>) -> String {
    String::from_utf8(v).expect("This input is not utf8 formatted string")
}

/// return stock top index and check stack size and panic if necessary
pub fn get_stack_top_index(parser: &mut Parser) -> usize {
    let l = parser.stack.len();
    if l == 0 {
        panic!(
            "The json string is malformed. Json is closed while there are more data. {}",
            get_current_status(parser)
        )
    }
    l - 1
}

/// Parse json stream. Verification happens during parsing, so the stream can be incomplete.
/// The result may be a key, value or None
///
/// ### Sample result
/// <pre>
/// None({ as u8)    (String,"key")    None(: as u8)     (Integer,"123")    None({ as u8)
/// <span style="color:yellow">
/// ↓                    ↓              ↓                       ↓             ↓
/// </span>
/// {                  "key"            :                      123            }
/// </pre>
pub fn walk_forward(parser: &mut Parser) -> TextItem {
    let c = next_no_white_space(parser);
    let top_index = get_stack_top_index(parser);
    (parser.stack[top_index].next_executor)(parser, top_index, c)
}

/// return the level of current position in json string.
/// for more information check out next_item_by_level() doc
pub fn get_current_level(parser: &Parser) -> f32 {
    match parser.stack.last() {
        None => { -1.0 }
        Some(t) => { t.level }
    }
}

/// Parse json until the position at which, node level reaches the target_level_offset
/// ## Sample json with level in different positions after parsing each element:
/// <pre>
/// <span style="color:red">
/// 0 1       1 1.5   1  1       1 1.5 2     2  2  3        3 3.5   3  2  1  0
/// </span>
/// <span style="color:yellow">
/// ↓ ↓       ↓  ↓    ↓  ↓       ↓  ↓  ↓     ↓  ↓  ↓        ↓  ↓    ↓  ↓  ↓  ↓
/// </span>
///  {  "key1"  :  123  ,  "key2"  :  [  true  ,  {  "key21"  :  2.5  }  ]  }
/// </pre>
///
/// The result determines if there can be more data or not.
/// For example if cursor is the above json is after 2.5 and before "}", result will be false. It means that there is no more data for level 3.
pub fn seek_by_level_offset(parser: &mut Parser, target_level_offset: f32) -> bool {
    let mut top_index = get_stack_top_index(parser);
    let target_level = parser.stack[top_index].level + target_level_offset;

    // there is no item in level 0 (except root) and smaller than that
    if target_level < 1_f32 { return false; };

    while parser.next_byte != NIL {
        walk_forward(parser);
        top_index = parser.stack.len() - 1;
        if parser.stack[top_index].level == target_level /*&& parser.next_byte != b','*/ {
            return parser.next_byte != b'}' && parser.next_byte != b']';
        }
    }
    false
}

/// if mem_size is set in new() function, this function will return the latest piece of json, so you can apply a regex operation for example
pub fn get_recent_piece(parser: &mut Parser) -> String {
    parser.txt.to_string()
}
//endregion

//region extractors

/// extract data between two "
fn extract_string(parser: &mut Parser) -> Item {
    let mut result = Vec::with_capacity(50);
    let mut c: u8;
    loop {
        c = next(parser);
        if c == b'\\' {
            c = next(parser);
        } else if c == b'"' {
            break;
        }
        result.push(c);
    }
    if parser.next_byte.is_ascii_whitespace() {
        next_no_white_space(parser);
    }
    (ValueType::Str, to_string(result))
}

/// extract some data such as null, true, false and numbers
fn extract_word(parser: &mut Parser, mut c: u8) -> Item {
    let mut result = Vec::with_capacity(50);
    let value_type;
    let mut d: usize;
    if c == b'+' || c == b'-' || c.is_ascii_digit() {
        result.push(c);
        d = 0;
        let mut last_digit_index = if c != b'+' && c != b'-' { 1 } else { usize::MAX };
        loop {
            c = parser.next_byte;
            if c == b'.' {
                if d >= 1 {
                    panic!(
                        r#"It is not allowed to have more than one point in a number.{}"#,
                        get_current_status(parser)
                    );
                }
                d += 1;
                result.push(c);
                _ = next(parser);
            } else if c.is_ascii_digit() {
                result.push(c);
                last_digit_index = result.len();
                _ = next(parser);
            } else {
                if result.len() != last_digit_index || c == b'-' || c == b'+' {
                    panic!(
                        r#"Number format is wrong.{}"#,
                        get_current_status(parser)
                    );
                }
                value_type = match d {
                    0 => ValueType::Int,
                    _ => ValueType::Float,
                };
                break;
            }
        }
    } else {
        let expected_word;
        result.push(c);

        // to support_capital_word, uncomment below line
        // if c <= 90 { c += 32 }

        if c == NULL[0] {
            expected_word = NULL;
            value_type = ValueType::Null;
        } else if c == TRUE[0] {
            expected_word = TRUE;
            value_type = ValueType::Bool;
        } else if c == FALSE[0] {
            expected_word = FALSE;
            value_type = ValueType::Bool;
        } else {
            panic!(
                r#"Expecting "null | true | false" but found `{}`. {}"#,
                c,
                get_current_status(parser)
            );
        }
        let l = expected_word.len();
        d = 0;
        loop {
            d += 1;
            if d >= l {
                break;
            }
            c = next(parser);
            result.push(c);

            // to support_capital_word, uncomment below line
            // if c <= 90 { c += 32 }

            if c != expected_word[d] {
                panic!(
                    r#"Expecting "null, true, false" but found "{}". error info => {:?}"#,
                    to_string(result),
                    get_current_status(parser)
                );
            }
        }
    }
    if parser.next_byte.is_ascii_whitespace() {
        next_no_white_space(parser);
    }
    (value_type, to_string(result))
}
//endregion
//region logic

//region logic tools area
pub struct StackItem {
    next_executor: fn(parser: &mut Parser, top_index: usize, current_byte: u8) -> TextItem,
    pub key: Rc<String>,
    pub level: f32,
    pub nth: usize,
    pub symbol: char,
}

#[derive(Debug, PartialEq)]
pub enum TextItem {
    Key(Item),
    Value(Item),
    None(u8),
}

/// pop then execute top
fn pop_stack(parser: &mut Parser, top_index: usize) {
    parser.stack.remove(top_index);
    let i = top_index - 1;
    (parser.stack[i].next_executor)(parser, i, NIL);
}

/// panic with current status
fn panic(parser: &mut Parser, current_byte: u8) -> TextItem {
    panic!(
        r#"Unexpected char `{}`. {}"#,
        current_byte as char,
        get_current_status(parser)
    );
}

/// json has tree structure. this function returns that path to the current position with some details
pub fn get_path(parser: &mut Parser) -> Vec<PathItem> {
    let l = parser.stack.len();
    let mut path = Vec::with_capacity(l);
    path.push(PathItem::Start);
    for i in 0..l {
        if parser.stack[i].symbol == '{' {
            path.push(PathItem::Object(
                parser.stack[i].key.clone(),
                parser.stack[i].nth,
            ))
        } else if parser.stack[i].symbol == '[' {
            path.push(PathItem::Array(
                parser.stack[i].key.clone(),
                parser.stack[i].nth,
            ))
        }
    }
    path
}

fn new_open_brace_stack_item(last_level: f32) -> StackItem {
    StackItem {
        next_executor: open_brace_start_state,
        key: Rc::new(String::from("")),
        level: (last_level + 1_f32).floor(),
        nth: 0,
        symbol: '{',
    }
}

fn new_open_square_stack_item(key: Rc<String>, last_level: f32) -> StackItem {
    StackItem {
        next_executor: open_square_start_state,
        key,
        level: (last_level + 1_f32).floor(),
        nth: 0,
        symbol: '[',
    }
}

fn new_colon_stack_item(key: Rc<String>, last_level: f32) -> StackItem {
    StackItem {
        next_executor: colon_start_state,
        key,
        level: last_level + 0.5,
        nth: 0,
        symbol: ':',
    }
}
//endregion

//region stack top is colon
fn colon_start_state(parser: &mut Parser, top_index: usize, current_byte: u8) -> TextItem {
    let top = &mut parser.stack[top_index];
    match current_byte {
        b'"' => {
            parser.stack.pop();
            TextItem::Value(extract_string(parser))
        }
        b'{' => {
            top.next_executor = colon_after_return_state;
            let level = top.level;
            parser.stack.push(new_open_brace_stack_item(level));
            TextItem::None(current_byte)
        }
        b'[' => {
            let key = top.key.clone();
            top.next_executor = colon_after_return_state;
            let level = top.level;
            parser.stack.push(new_open_square_stack_item(key, level));
            TextItem::None(current_byte)
        }
        b'}' | b']' | b',' | b':' => panic(parser, current_byte),
        _ => {
            parser.stack.pop();
            TextItem::Value(extract_word(parser, current_byte))
        }
    }
}

fn colon_after_return_state(parser: &mut Parser, top_index: usize, current_byte: u8) -> TextItem {
    parser.stack.remove(top_index);
    TextItem::None(current_byte)
}
//endregion

//region stack top is open brace
fn open_brace_start_state(parser: &mut Parser, top_index: usize, current_byte: u8) -> TextItem {
    match current_byte {
        b'"' => {
            let txt = extract_string(parser);
            let top = &mut parser.stack[top_index];
            top.next_executor = open_brace_after_key_state;
            top.key = Rc::new(txt.1.clone());
            TextItem::Key(txt)
        }
        b'}' => {
            pop_stack(parser, top_index);
            TextItem::None(current_byte)
        }
        _ => panic(parser, current_byte),
    }
}

fn open_brace_after_key_state(parser: &mut Parser, top_index: usize, current_byte: u8) -> TextItem {
    let top = &mut parser.stack[top_index];
    match current_byte {
        b':' => {
            let key = top.key.clone();
            top.next_executor = open_brace_after_colon_state;
            let level = top.level;
            parser.stack.push(new_colon_stack_item(key, level));
            TextItem::None(current_byte)
        }
        _ => panic(parser, current_byte),
    }
}

fn open_brace_after_colon_state(parser: &mut Parser, top_index: usize, current_byte: u8) -> TextItem {
    let top = &mut parser.stack[top_index];
    match current_byte {
        b'}' => {
            pop_stack(parser, top_index);
            TextItem::None(current_byte)
        }
        b',' => {
            top.next_executor = open_brace_start_state;
            top.nth += 1;
            TextItem::None(current_byte)
        }
        _ => panic(parser, current_byte),
    }
}
//endregion

//region stack top is open square
fn open_square_start_state(parser: &mut Parser, top_index: usize, current_byte: u8) -> TextItem {
    let top = &mut parser.stack[top_index];
    match current_byte {
        b'"' => {
            top.next_executor = open_square_after_single_value_state;
            TextItem::Value(extract_string(parser))
        }
        b'{' => {
            top.next_executor = open_square_after_return;
            let level = top.level;
            parser.stack.push(new_open_brace_stack_item(level));
            TextItem::None(current_byte)
        }
        b'[' => {
            let key = top.key.clone();
            top.next_executor = open_square_after_return;
            let level = top.level;
            parser.stack.push(new_open_square_stack_item(key, level));
            TextItem::None(current_byte)
        }
        b']' => {
            pop_stack(parser, top_index);
            TextItem::None(current_byte)
        }
        b',' | b':' | b'}' => panic(parser, current_byte),
        _ => {
            top.next_executor = open_square_after_single_value_state;
            TextItem::Value(extract_word(parser, current_byte))
        }
    }
}

fn open_square_after_single_value_state(parser: &mut Parser, top_index: usize, current_byte: u8) -> TextItem {
    let top = &mut parser.stack[top_index];
    match current_byte {
        b']' => {
            pop_stack(parser, top_index);
            TextItem::None(current_byte)
        }
        b',' => {
            top.next_executor = open_square_start_state;
            top.nth += 1;
            TextItem::None(current_byte)
        }
        _ => panic(parser, current_byte),
    }
}

fn open_square_after_return(parser: &mut Parser, top_index: usize, current_byte: u8) -> TextItem {
    let top = &mut parser.stack[top_index];
    top.next_executor = open_square_after_single_value_state;
    TextItem::None(current_byte)
}
//endregion
//endregion

//region high-level extractors
fn extract_current_item(parser: &mut Parser) -> Item {
    match walk_forward(parser) {
        TextItem::Value(t) => t,
        TextItem::Key(t) => t,
        _ => panic!("Expected a value or key.{}", get_current_status(parser)),
    }
}

// to be run when top is :
pub fn extract_current_value(parser: &mut Parser, top_index: usize) -> Content {
    return match parser.next_byte {
        b'[' => {
            walk_forward(parser);
            extract_current_array(parser, top_index + 1)
        }
        b'{' => {
            walk_forward(parser);
            extract_current_object(parser, top_index + 1)
        }
        _ => match walk_forward(parser) {
            TextItem::Value(t) => Content::Simple(t),
            _ => {
                panic!("Expecting a value.{}", get_current_status(parser))
            }
        },
    };
}

// to be run when top is [
fn extract_current_array(parser: &mut Parser, top_index: usize) -> Content {
    let mut a: Vec<Content> = Vec::new();
    loop {
        match parser.next_byte {
            b',' => {
                walk_forward(parser);
            }
            b']' => {
                walk_forward(parser);
                break;
            }
            _ => {
                a.push(extract_current_value(parser, top_index));
            }
        }
    }
    Content::Array(a)
}

// to be run when top is { and cursor is before a key
fn extract_current_object(parser: &mut Parser, top_index: usize) -> Content {
    let mut a: BTreeMap<String, Content> = BTreeMap::new();
    let mut key;
    let mut val;
    loop {
        key = match parser.next_byte {
            b'}' => {
                walk_forward(parser);
                break;
            }
            _ => extract_current_item(parser),
        }
            .1;
        walk_forward(parser);
        val = extract_current_value(parser, top_index + 1);
        a.insert(key, val);
        match parser.next_byte {
            b',' => {
                walk_forward(parser);
                continue;
            }
            b'}' => {
                walk_forward(parser);
                break;
            }
            _ => panic!("Unexpected char.{}", get_current_status(parser)),
        }
    }
    Content::Object(a)
}
//endregion

#[cfg(test)]
mod parser_tests {
    use std::panic::*;

    use regex::Regex;

    use crate::NIL;
    use crate::parser_core::*;
    use crate::readers::StringReader;

    const CORRECT_JSON: &str = r#" {"key1":null,"key2":true,"key3":false,"key4":-111,"key5":+111.111,"key6":"str1 \":{}[],","key7":{  "key71" : null ,  "key72" : true ,  "key73" : false ,  "key74" : 222 ,  "key75" : 222.222 ,  "key76" : "str2 \":{}[]," ,  "key78" : [    null ,    true ,    false ,    -333 ,    +333.333 ,    "str3 \":{}[]," ,    {  } ,    [  ]  ] ,  "key79" : {} ,  "key710": [  ] } , "key8" : [  null ,  true ,  false ,  444 ,  444.444 ,  "str4 \":{}[]," ,  {    "key81" : null ,    "key82" : true ,    "key83" : false ,    "key84" : 555 ,
      "key85" : 555.555 ,
      "key86" : "str5 \":{}[]," ,    "key89" : {} ,    "key810" : [ ]  } ,  { } ,  [ ]  ] , "key9" : { } , "key10" : [ ]
} "#;

    #[ctor::ctor]
    fn initialize() {
        set_hook(Box::new(|_info| {
            // println!("{}",info)
        }));
    }

    fn execute_test(txt: &'static str, keys: &[&str], values: &[&str], chars: &[char]) {
        let mut keys_index = 0;
        let mut values_index = 0;
        let mut chars_index = 0;
        let result = catch_unwind(move || {
            let mut parser = Parser::new(StringReader::new(txt.into()), 50);
            while parser.next_byte != NIL {
                let r = walk_forward(&mut parser);
                match r {
                    TextItem::Key(k) => {
                        if k.1.ne(keys[keys_index]) {
                            // println!(">>>>> {} != {}", keys[keys_index], k.1);
                            panic!("expecting key: {}", k.1)
                        }
                        keys_index += 1;
                    }
                    TextItem::Value(v) => {
                        if v.1.ne(values[values_index]) {
                            // println!(">>>>> {} != {}", values[values_index], v.1);
                            panic!("expecting value: {}", v.1)
                        }
                        values_index += 1;
                    }
                    TextItem::None(_) => {
                        if chars_index >= chars.len() {
                            // println!(">>>>> {} >= {}", chars_index, chars.len());
                            panic!("expecting no more")
                        }
                        chars_index += 1;
                    }
                }
            }
        });
        assert_eq!(result.is_ok(), true);
    }

    fn execute_for_panic(txt: &'static str) -> String {
        let payload = catch_unwind(|| {
            let mut parser = Parser::new(StringReader::new(txt.into()), 50);
            while parser.next_byte != NIL {
                walk_forward(&mut parser);
            }
        })
            .unwrap_err();
        String::from(panic_message::panic_message(&payload))
    }

    fn does_error_msg_ends_with(error_msg: &str, expected_ending: &str) -> Result<bool, ()> {
        let raw_er;
        let re = Regex::new(" nth: \\d+.+?\\.\\.\\.").unwrap();
        match re.find(error_msg) {
            None => {
                raw_er = error_msg.to_owned();
            }
            Some(m) => {
                let mut temp = &error_msg[m.start()..m.end()];
                temp = &temp[temp.find(",").unwrap() + 1..];
                let color_regex = Regex::new(r#"\x1b\[\d+m"#).unwrap();
                raw_er = color_regex.replace_all(temp, "").trim().to_string();
            }
        }
        let expected_len = expected_ending.len();
        let end = if raw_er.ends_with("...") { raw_er.len() - 3 } else { raw_er.len() };
        let start = if end > expected_len { end - expected_len } else { 0 };
        Ok(expected_ending.eq(&raw_er[start..end]))
    }

    #[test]
    fn correct_input_starting_with_brace() {
        let keys = [
            "key1", "key2", "key3", "key4", "key5", "key6", "key7", "key71", "key72", "key73",
            "key74", "key75", "key76", "key78", "key79", "key710", "key8", "key81", "key82",
            "key83", "key84", "key85", "key86", "key89", "key810", "key9", "key10",
        ];
        let values = [
            "null",
            "true",
            "false",
            "-111",
            "+111.111",
            "str1 \":{}[],",
            "null",
            "true",
            "false",
            "222",
            "222.222",
            "str2 \":{}[],",
            "null",
            "true",
            "false",
            "-333",
            "+333.333",
            "str3 \":{}[],",
            "null",
            "true",
            "false",
            "444",
            "444.444",
            "str4 \":{}[],",
            "null",
            "true",
            "false",
            "555",
            "555.555",
            "str5 \":{}[],",
        ];
        let chars = [
            '{', ':', ',', ':', ',', ':', ',', ':', ',', ':', ',', ':', ',', ':', '{', ':', ',',
            ':', ',', ':', ',', ':', ',', ':', ',', ':', ',', ':', '[', ',', ',', ',', ',', ',',
            ',', '{', '}', ',', '[', ']', ']', ',', ':', '{', '}', ',', ':', '[', ']', '}', ',',
            ':', '[', ',', ',', ',', ',', ',', ',', '{', ':', ',', ':', ',', ':', ',', ':', ',',
            ':', ',', ':', ',', ':', '{', '}', ',', ':', '[', ']', '}', ',', '{', '}', ',', '[',
            ']', ']', ',', ':', '{', '}', ',', ':', '[', ']', '}',
        ];
        execute_test(CORRECT_JSON, &keys, &values, &chars);
    }

    #[test]
    fn correct_input_starting_with_square() {
        let txt = r#"[ null , true , false , 444 , 444.444 , "str4 \":{}[]," , {"key81": null,"key82": true,"key83": false,"key84": 555,"key85": 555.555 , "key86": "str5 \":{}[],"} ]"#;
        let keys = ["key81", "key82", "key83", "key84", "key85", "key86"];
        let values = [
            "null",
            "true",
            "false",
            "444",
            "444.444",
            "str4 \":{}[],",
            "null",
            "true",
            "false",
            "555",
            "555.555",
            "str5 \":{}[],",
        ];
        let chars = [
            '[', ',', ',', ',', ',', ',', ',', '{', ':', ',', ':', ',', ':', ',', ':', ',', ':',
            ',', ':', '}', ']', '}',
        ];
        execute_test(txt, &keys, &values, &chars);
    }

    #[test]
    fn incorrect_input_drop_key() {
        let txt = r#"{:123}"#;
        let result = execute_for_panic(txt);
        assert!(does_error_msg_ends_with(&result, "{:1").is_ok_and(|b| b));
    }

    #[test]
    fn incorrect_input_drop_colon() {
        let txt = r#"{"key"123}"#;
        let result = execute_for_panic(txt);
        assert!(does_error_msg_ends_with(&result, r#"{"key"12"#).is_ok_and(|b| b));
    }

    #[test]
    fn incorrect_input_drop_object_value() {
        let txt = r#"{"key":,}"#;
        let result = execute_for_panic(txt);
        assert!(does_error_msg_ends_with(&result, r#"{"key":,}"#).is_ok_and(|b| b));
    }

    #[test]
    fn incorrect_input_early_finish1() {
        let txt = r#"{"key":}"#;
        let result = execute_for_panic(txt);
        assert_eq!(result, "Unexpected end of stream");
    }

    #[test]
    fn incorrect_input_early_finish2() {
        let txt = r#"{"key1":123,"key2":[}"#;
        let result = execute_for_panic(txt);
        assert_eq!(result, "Unexpected end of stream");
    }

    #[test]
    fn incorrect_input_early_finish3() {
        let txt = r#"{"key1":123,"key2":{}"#;
        let result = execute_for_panic(txt);
        assert_eq!(result, "Unexpected end of stream");
    }

    #[test]
    fn incorrect_extra_input_start_with_brace() {
        let txt = r#"{"key1":123,"key2":null},"#;
        let result = execute_for_panic(txt);
        assert!(does_error_msg_ends_with(&result, r#"stack is empty"#).is_ok_and(|b| b));
    }

    #[test]
    fn incorrect_extra_input_start_with_square() {
        let txt = r#"[123,null],"#;
        let result = execute_for_panic(txt);
        assert!(does_error_msg_ends_with(&result, r#"stack is empty"#).is_ok_and(|b| b));
    }

    #[test]
    fn correct_input_start_with_single_value() {
        let txt = r#""val123""#;
        let mut parser = Parser::new(StringReader::new(txt.into()), 50);
        let result = walk_forward(&mut parser);
        match result {
            TextItem::Value(v) => {
                assert_eq!(v.1, "val123")
            }
            _ => {
                assert_eq!(1, 2)
            }
        }
    }

    #[test]
    fn walk_till_child_node() {
        let mut parser = Parser::new(StringReader::new(CORRECT_JSON.into()), 50);
        let result = seek_by_level_offset(&mut parser, 2.0);
        assert!(result);
        let item = walk_forward(&mut parser);
        assert_eq!(item, TextItem::Key((ValueType::Str, String::from("key71"))));
    }

    #[test]
    fn walk_till_parent_node() {
        let mut parser = Parser::new(StringReader::new(CORRECT_JSON.into()), 50);
        seek_by_level_offset(&mut parser, 2.0);
        seek_by_level_offset(&mut parser, -1.0);
        let item = walk_forward(&mut parser);
        assert_eq!(item, TextItem::None(b','));
        let item = walk_forward(&mut parser);
        assert_eq!(item, TextItem::Key((ValueType::Str, String::from("key8"))))
    }

    #[test]
    fn catch_sibling_nodes_of_object() {
        let items = ["key71", "key72", "key73", "key74", "key75", "key76", "key78", "key79", "key710"];
        let mut index = 0;
        let mut parser = Parser::new(StringReader::new(CORRECT_JSON.into()), 50);
        let mut result = seek_by_level_offset(&mut parser, 2.0);
        while result {
            let item = walk_forward(&mut parser);
            match item {
                TextItem::Key(m) => {
                    assert_eq!(m.1, items[index]);
                    index += 1;
                }
                TextItem::None(b',') => {
                    continue;
                }
                _ => {
                    assert!(false, "it is not supposed to get any item other than key")
                }
            }
            result = seek_by_level_offset(&mut parser, 0.0);
        }
    }

    #[test]
    fn catch_sibling_nodes_of_array() {
        let items = ["null", "true", "false", "444", "444.444", "str4 \":{}[],"];
        let mut index = 0;

        let mut parser = Parser::new(StringReader::new(CORRECT_JSON.into()), 50);

        loop {
            let item = walk_forward(&mut parser);
            match item {
                TextItem::Key(k) => { if k.1.eq("key8") { break; } }
                _ => {}
            }
        }

        let mut result = seek_by_level_offset(&mut parser, 1.0);
        let mut diff = 0.0;
        while result {
            let item = walk_forward(&mut parser);
            match item {
                TextItem::Value(m) => {
                    assert_eq!(m.1, items[index]);
                    index += 1;
                    diff = 0.0;
                }
                TextItem::None(b',') => {
                    diff = 0.0;
                }
                TextItem::None(b'{') | TextItem::None(b'[') => {
                    diff = -1.0;
                }
                _ => {
                    assert!(true, "It is not supposed to face any item other than value, comma, open brace or open square")
                }
            }
            result = seek_by_level_offset(&mut parser, diff);
        }
    }
}
