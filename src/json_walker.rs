use crate::*;
#[cfg(feature = "deserialize")]
use crate::deserializer::deserialize_mod::Deserializer;
pub use crate::Error;
use crate::parser_core::{extract_current_value, get_stack_top_index, Parser, walk_forward, get_current_level, get_path, get_recent_piece, seek_by_level_offset};
pub use crate::parser_core::{Content, Item, Parser as JsonWalker, PathItem, TextItem, ValueType};
pub use crate::readers::*;

impl Parser {
    /// return the level of current position in json string.
    /// for more information check out next_item_by_level() doc
    pub fn get_current_level(&mut self) -> f32 {
        get_current_level(self)
    }

    /// json has tree structure. this function returns that path to the current position with some details
    pub fn get_path(&mut self) -> Vec<PathItem> {
        get_path(self)
    }

    /// if mem_size is set in new() function, this function will return the latest piece of json, so you can apply a regex operation for example
    pub fn get_recent_piece(&mut self) -> String {
        get_recent_piece(self)
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
    pub fn seek_by_level_offset(&mut self, target_level_offset: f32) -> bool {
        seek_by_level_offset(self, target_level_offset)
    }

    /// Return current path string.
    /// - default root is "#"
    /// - objects are surrounded between "{" and "}"
    /// - arrays are surrounded between "[" and "]"
    /// - each item is formatted like (latest_key_name, index_of_child_in_its_parent)
    /// #### Consider below json with detailed path for different positions. The **Green** ones are the positions which you can access via **next_item()** function.
    /// <pre>
    /// { "key1" : 1 , ..., "key8" : [ "value1" , ..., "value6" , { "key81" : ... } ]}
    ///  ⎟      ⎟ ⎟ ⎟      ⎟      ⎟ ⎟ ⎟        ⎟ ⎟    ⎟        ⎟           ⎟       ↓
    ///  ⎟      ⎟ ⎟ ⎟      ⎟      ⎟ ⎟ ⎟        ⎟ ⎟    ⎟        ⎟           ↓       #/{key8,7}/[key8,6]/
    ///  ⎟      ⎟ ⎟ ⎟      ⎟      ⎟ ⎟ ⎟        ⎟ ⎟    ⎟        ↓           <span style="color:green">#/{key8,7}/[key8,6]/{key81,0}/</span>
    ///  ⎟      ⎟ ⎟ ⎟      ⎟      ⎟ ⎟ ⎟        ⎟ ⎟    ↓        <span style="color:green">#/{key8,7}/[key8,6]/</span>
    ///  ⎟      ⎟ ⎟ ⎟      ⎟      ⎟ ⎟ ⎟        ⎟ ↓    #/{key8,7}/[key8,5]/
    ///  ⎟      ⎟ ⎟ ⎟      ⎟      ⎟ ⎟ ⎟        ↓ #/{key8,7}/[key8,1]/
    ///  ⎟      ⎟ ⎟ ⎟      ⎟      ⎟ ⎟ ↓        <span style="color:green">#/{key8,7}/[key8,0]/</span>
    ///  ⎟      ⎟ ⎟ ⎟      ⎟      ⎟ ↓ #/{key8,7}/[key8,0]/
    ///  ⎟      ⎟ ⎟ ⎟      ⎟      ↓ #/{key8,7}/
    ///  ⎟      ⎟ ⎟ ⎟      ↓      <span style="color:green">#/{key8,7}/</span>
    ///  ⎟      ⎟ ⎟ ↓      #/{key7,7}/
    ///  ⎟      ⎟ ↓ <span style="color:green">#/{key1,0}/</span>
    ///  ⎟      ↓ #/{key1,0}/
    ///  ↓      <span style="color:green">#/{key1,0}/</span>
    ///  #/{#,0}/
    /// </pre>
    pub fn get_path_string(&mut self) -> String {
        let p = self.get_path();
        let mut s = String::with_capacity(p.len() * 10);
        for x in p.iter() {
            s.push_str(&x.to_string());
            s.push('/');
        }
        s
    }

    /// Return next key or value in json. No matter if the item belongs to the child node or parent. If  no item exists, None will be returned
    pub fn next_item(&mut self) -> Result<Item, Error> {
        while self.next_byte != NIL {
            match walk_forward(self) {
                TextItem::Key(t) | TextItem::Value(t) => {
                    return Ok(t);
                }
                _ => {
                    continue;
                }
            }
        }
        Err(Error::new_eos())
    }

    /// Next key will be returned and values will be ignored. No matter if it belongs to child or parent node. If there is no more key, None would be the result
    pub fn next_key(&mut self) -> Result<Item, Error> {
        while self.next_byte != NIL {
            match walk_forward(self) {
                TextItem::Key(t) => {
                    return Ok(t);
                }
                _ => {
                    continue;
                }
            }
        }
        Err(Error::new_eos())
    }

    /// The json will be parsed till the mentioned key. If key does not exist or it is already passed,
    /// parsing will continue to the end of stream.
    pub fn next_key_by_name(&mut self, name: &str) -> Result<Item, Error> {
        let mut key;
        loop {
            key = self.next_key();
            match key {
                Ok(t) if !t.1.eq(name) => {
                    continue;
                }
                _ => {}
            }
            break;
        }
        key
    }

    /// The json will be parsed till the next sibling key.
    /// At the end of current element (object or array), None will be returned and cursor will not move any further by this function
    pub fn next_sibling_key(&mut self) -> Result<Item, Error> {
        if self.next_byte != NIL {
            let top_index = get_stack_top_index(self);
            let top_stack_level = self.stack[top_index].level;
            let diff = top_stack_level - top_stack_level.floor();
            if seek_by_level_offset(self, diff) {
                return self.next_key();
            }
        }
        Err(Error::new_eos())
    }

    /// Return next child key.
    /// The key must be only one level lower than the current node, so grand children will not count in.
    pub fn next_child_key(&mut self) -> Result<Item, Error> {
        if self.next_byte != NIL {
            let top_index = get_stack_top_index(self);
            let top_stack_level = self.stack[top_index].level;
            let diff = (top_stack_level + 1.0).floor() - top_stack_level;
            if seek_by_level_offset(self, diff) {
                return self.next_key();
            }
        }
        Err(Error::new_eos())
    }

    /// Return next key of parent (1 level up) or None if parent has no more key
    pub fn next_key_from_parent(&mut self) -> Result<Item, Error> {
        if self.next_byte != NIL {
            let top_index = get_stack_top_index(self);
            let top_stack_level = self.stack[top_index].level;
            let diff = (top_stack_level - 1.0).ceil() - top_stack_level;
            if seek_by_level_offset(self, diff) {
                return self.next_key();
            }
        }
        Err(Error::new_eos())
    }

    /// Parse json until the position in which, node level reaches the target_level.
    /// - For short, consider each "[" and "{" one level increase and "]" and "}" one level decrease
    /// - ":" is used for accessing simple values
    /// ## Sample json with node level in different positions:
    /// <pre>
    /// 0 1       1.5   1      1.5 2     2 3        3.5    2 1 0
    /// <span style="color:yellow">
    /// ↓ ↓        ↓    ↓        ↓ ↓     ↓ ↓         ↓     ↓ ↓ ↓
    /// </span>
    ///  { "key1" : 123, "key2" : [ true, { "key21" : 2.5 } ] }
    /// </pre>
    pub fn next_item_by_level(&mut self, target_level: f32) -> Result<Item, Error> {
        let mut ti;
        let mut stack_top;
        while self.next_byte != NIL {
            ti = walk_forward(self);
            stack_top = self.stack.last().unwrap();
            if stack_top.level == target_level {
                match ti {
                    TextItem::Key(t) | TextItem::Value(t) => {
                        return Ok(t);
                    }
                    _ => {
                        continue;
                    }
                }
            }
        }
        Err(Error::new_eos())
    }

    /// To jump to the desired item (key or value), use this function.
    /// This function executes the provided patterns from last to first on any found key or value.
    /// When ever a pattern fails, it doesn't check the others and scans for another item in json,
    /// so it is recommended to minimize number of patterns to reach higher performance.
    /// - This function works by path, so please check out docs of get_path_string() and next_item_by_level() functions.
    /// - Notice that pattern is not something like **regex**. If you need **regex** to find the item, you can use get_recent_piece() function.
    ///
    /// # Example
    ///```
    /// fn main(){
    ///    use json_walker::json_walker::{CurrentState, Item, JsonWalker, StringReader, ValueType};
    ///
    ///    let json = r#"[{"key1":{"key4":100},"key2":10},[{"key1":{"key4":300}, "key3":100}],"key1"]"#;
    ///    let mut walker = JsonWalker::new(StringReader::new(json.to_string()), 50);
    ///    let patterns = vec![
    ///       |cs: &CurrentState| -> bool{ cs.level == 2.0 && cs.nth_occurrence == 0 },
    ///       |cs: &CurrentState| -> bool{ cs.latest_key.eq("key1") && cs.level == 3.0 },
    ///       |cs: &CurrentState| -> bool{cs.latest_key.eq("key4") },
    ///    ];
    ///
    ///    let item = walker.next_item_by_pattern(&patterns);
    ///    assert_eq!(item, Ok((ValueType::Str, String::from("key4"))));
    /// }
    /// ```
    /// In the above example 3 patterns are hired p0, p1 & p2 and we want to find the second key4.
    ///
    /// In marked positions, the provided patterns get called from last to first:
    /// <pre>
    /// [{"key1" :{"key4" :100 },"key2" :10 },[{"key1" :{"key4" :300}, "key3":100}],"key1"]
    /// <span style="color:red">
    ///        1        2    3        4   5          6        7
    /// </span>
    ///</pre>
    ///
    /// | pos | path | pattern|
    /// |-----|---------------------------------------|----------|
    /// |   | _ | p0: level == 2.0 && nth_occurrence == 0
    /// | 1 | _ | p1: latest_key.eq("key1") && level == 3.0
    /// |   | #/[#, 0]/<span style="color:teal">{key1, 0}</span>/ | p2: **latest_key.eq("key4")** 🔴 ↩️ <span style="color:red">->(key is "key1")</span>
    /// |-----|---------------------------------------|-----------------------------/---------------------|
    /// |   | _ | p0:  level == 2.0 && nth_occurrence == 0
    /// | 2 | #/[#, 0]/<span style="color:teal">{key1, 0}</span>/{key4, 0}/ | p1: latest_key.eq("key1") && **level == 3.0** 🔴 ↩️ <span style="color:red">->(level of key1 is "2")</span>
    /// |   | #/[#, 0]/{key1, 0}/<span style="color:teal">{key4, 0}</span>/ | p2: latest_key.eq("key4") 🟢
    /// |-----|---------------------------------------|--------------------------------------------------|
    /// |   | _ | p0:  level == 2.0 && nth_occurrence == 0
    /// | 3 | #/[#, 0]/<span style="color:teal">{key1, 0}</span>/{key4, 0}/ | p1: latest_key.eq("key1") && **level == 3.0** 🔴 ↩️ <span style="color:red">->(level of key1 is "2")</span>
    /// |   | #/[#, 0]/{key1, 0}/<span style="color:teal">{key4, 0}</span>/ | p2: latest_key.eq("key4") 🟢
    /// |-----|---------------------------------------|--------------------------------------------------|
    /// |   | _ | p0:  level == 2.0 && nth_occurrence == 0
    /// | 4 | _ | p1: latest_key.eq("key1") && level == 3.0
    /// |   | #/[#, 0]/<span style="color:teal">{key2, 1}</span>/ | p2: **latest_key.eq("key4")** 🔴 ↩️ <span style="color:red">->(key is "key2")</span>
    /// |-----|---------------------------------------|--------------------------------------------------|
    /// |   | _ | p0:  level == 2.0 && nth_occurrence == 0
    /// | 5 | _ | p1: latest_key.eq("key1") && level == 3.0
    /// |   | #/[#, 0]/<span style="color:teal">{key2, 1}</span>/ | p2: **latest_key.eq("key4")** 🔴 ↩️ <span style="color:red">->(key is "key2")</span>
    /// |-----|---------------------------------------|--------------------------------------------------|
    /// |   | _ | p0:  level == 2.0 && nth_occurrence == 0
    /// | 6 | _ | p1: latest_key.eq("key1") && level == 3.0
    /// |   | #/[#, 1]/<span style="color:teal">{key1, 0}</span>/ | p2: **latest_key.eq("key4")** 🔴 ↩️ <span style="color:red">->(key is "key1")</span>
    /// |-----|---------------------------------------|--------------------------------------------------|
    /// |   | #/[#, 1]/<span style="color:teal">[#, 0]</span>/{key1, 0}/{key4, 0}/ | p0:  level == 2.0 && nth_occurrence == 0 🟢
    /// | 7 | #/[#, 1]/[#, 0]/<span style="color:teal">{key1, 0}</span>/{key4, 0}/ | p1: latest_key.eq("key1") && level == 3.0 🟢
    /// |   | #/[#, 1]/[#, 0]/{key1, 0}/<span style="color:teal">{key4, 0}</span>/ | p2: latest_key.eq("key4") 🟢
    pub fn next_item_by_pattern(&mut self, pattern: &Vec<impl Fn(&CurrentState) -> bool>) -> Result<Item, Error> {
        let pat_top = pattern.len() - 1;
        let mut pat_index;
        let mut stack_item;

        let mut is_key;
        let mut item;
        'next_item: while self.next_byte != NIL {
            match walk_forward(self) {
                TextItem::Key(m) => {
                    item = m;
                    is_key = true;
                }
                TextItem::Value(m) => {
                    item = m;
                    is_key = false;
                }
                _ => {
                    continue;
                }
            }
            pat_index = pat_top;
            for si in (1..=self.stack.len() - 1).rev() {
                stack_item = &self.stack[si];
                if stack_item.symbol != ':' {
                    if !pattern[pat_index](&CurrentState {
                        latest_key: &stack_item.key,
                        nth_occurrence: stack_item.nth,
                        level: stack_item.level,
                        current_item: &item,
                        is_key,
                    }) {
                        continue 'next_item;
                    }
                    if pat_index == 0 {
                        return Ok(item);
                    }
                    pat_index -= 1;
                }
            }
        }
        Err(Error::new_eos())
    }

    fn walk_before_value(&mut self) {
        while self.next_byte == b':' || self.next_byte == b',' || self.stack.last().is_some_and(|s| s.symbol == '{') {
            walk_forward(self);
        }
    }

    /// Based on cursor location, the value of current key will be returned.
    /// Value can be a single string, integer, float, boolean, null, object or array.
    /// If there is no progress, the whole object will be returned
    pub fn current_value_content(&mut self) -> Result<Content, Error> {
        self.walk_before_value();
        if self.next_byte != NIL {
            let top_index = get_stack_top_index(self);
            return Ok(extract_current_value(self, top_index));
        }
        Err(Error::new_eos())
    }

    /// Based on cursor location, the value of current key will be deserialize.
    #[cfg(feature = "deserialize")]
    pub fn current_value<V>(&mut self) -> Result<V, Error> where V: for<'a> serde::de::Deserialize<'a>, {
        self.walk_before_value();
        if self.next_byte != NIL {
            let mut de = Deserializer::new(self);
            return V::deserialize(&mut de);
        }
        Err(Error::new_eos())
    }

    /// move n item including key, value or other none white space char such as "{", "[", "}", "]", ":" or ","
    pub fn move_n_element_forward(&mut self, n: usize) {
        for _ in 0..n {
            walk_forward(self);
        }
    }
}

pub struct CurrentState<'a> {
    /// **latest_key** is the latest key seen in the current position
    pub latest_key: &'a str,

    /// **nth_occurrence** is the index of the current item (key or value) in json
    pub nth_occurrence: usize,

    /// **level: f32** is the level of the current position. Please check out next_item_by_level() docs
    pub level: f32,

    /// **current_item** is the current scanned which is Item(ValueType, value as string)
    pub current_item: &'a Item,

    /// **is_key: bool**, determines if the current item is a key or value
    pub is_key: bool,
}

#[cfg(test)]
mod walker_tests {
    use std::collections::BTreeMap;

    use crate::Error;
    use crate::json_walker::{CurrentState, JsonWalker};
    use crate::parser_core::{Content, ValueType};
    use crate::readers::StringReader;

    const CORRECT_JSON: &str = r#" {"key1":null,"key2":true,"key3":false,"key4":111,"key5":111.111,"key6":"str1 \":{}[],","key7":{  "key71" : null ,  "key72" : true ,  "key73" : false ,  "key74" : 222 ,  "key75" : 222.222 ,  "key76" : "str2 \":{}[]," ,  "key78" : [    null ,    true ,    false ,    333 ,    333.333 ,    "str3 \":{}[]," ,    {  } ,    [  ]  ] ,  "key79" : {} ,  "key710": [  ] } , "key8" : [  null ,  true ,  false ,  444 ,  444.444 ,  "str4 \":{}[]," ,  {    "key81" : null ,    "key82" : true ,    "key83" : false ,    "key84" : 555 ,
      "key85" : 555.555 ,
      "key86" : "str5 \":{}[]," ,    "key89" : {} ,    "key810" : [ ]  } ,  { } ,  [ ]  ] , "key9" : { } , "key10" : [ ]
} "#;

    #[test]
    fn test_next_item() {
        // keys and values must be retrieved in order
        let words = [
            "key1",
            "null",
            "key2",
            "true",
            "key3",
            "false",
            "key4",
            "111",
            "key5",
            "111.111",
            "key6",
            "str1 \":{}[],",
            "key7",
            "key71",
            "null",
            "key72",
            "true",
            "key73",
            "false",
            "key74",
            "222",
            "key75",
            "222.222",
            "key76",
            "str2 \":{}[],",
            "key78",
            "null",
            "true",
            "false",
            "333",
            "333.333",
            "str3 \":{}[],",
            "key79",
            "key710",
            "key8",
            "null",
            "true",
            "false",
            "444",
            "444.444",
            "str4 \":{}[],",
            "key81",
            "null",
            "key82",
            "true",
            "key83",
            "false",
            "key84",
            "555",
            "key85",
            "555.555",
            "key86",
            "str5 \":{}[],",
            "key89",
            "key810",
            "key9",
            "key10",
        ];
        let mut word_index = 0;
        let mut walker = JsonWalker::new(StringReader::new(CORRECT_JSON.to_string()), 50);
        loop {
            match walker.next_item() {
                Err(_) => {
                    break;
                }
                Ok(t) => {
                    assert!(t.1.eq(words[word_index]));
                    word_index += 1;
                }
            }
        }
    }

    #[test]
    fn test_next_key() {
        // only keys must be retrieved in order, no matter if the key belongs to a child or parent node
        let words = [
            "key1", "key2", "key3", "key4", "key5", "key6", "key7", "key71", "key72", "key73",
            "key74", "key75", "key76", "key78", "key79", "key710", "key8", "key81", "key82",
            "key83", "key84", "key85", "key86", "key89", "key810", "key9", "key10",
        ];
        let mut word_index = 0;
        let mut walker = JsonWalker::new(StringReader::new(CORRECT_JSON.to_string()), 50);
        loop {
            match walker.next_key() {
                Err(_) => {
                    break;
                }
                Ok(t) => {
                    assert!(t.1.eq(words[word_index]));
                    word_index += 1;
                }
            }
        }
    }

    #[test]
    fn test_next_key_by_name() {
        // key must be retrieved by its name, no matter if the key belongs to a child or parent node
        let mut walker = JsonWalker::new(StringReader::new(CORRECT_JSON.to_string()), 50);
        let result = walker.next_key_by_name("key2");
        assert_eq!(
            Ok((ValueType::Str, String::from("key2"))),
            result,
            r#"next_key_by_name("key2") != "key2" "#
        );

        let result = walker.next_key_by_name("key71");
        assert_eq!(
            Ok((ValueType::Str, String::from("key71"))),
            result,
            r#"next_key_by_name("key71") != "key71" "#
        );

        let result = walker.next_key_by_name("key82");
        assert_eq!(
            Ok((ValueType::Str, String::from("key82"))),
            result,
            r#"next_key_by_name("key82") != "key82" "#
        );

        let result = walker.next_key_by_name("key");
        assert_eq!(Err(Error::new_eos()), result, r#"next_key_by_name("key") != "key" "#);
    }

    #[test]
    fn test_get_path_and_get_path_string() {
        let mut walker = JsonWalker::new(StringReader::new(CORRECT_JSON.to_string()), 50);
        let _ = walker.next_key_by_name("key81");
        let path = walker.get_path_string();
        assert_eq!(path, "#/{key8,7}/[key8,6]/{key81,0}/");
    }

    #[test]
    fn test_next_sibling_key_for_level0() {
        let mut walker = JsonWalker::new(StringReader::new(CORRECT_JSON.to_string()), 50);
        let _ = walker.next_key();
        let keys = [
            "key1", "key2", "key3", "key4", "key5", "key6", "key7", "key8", "key9", "key10",
        ];
        let mut i = 1;
        loop {
            match walker.next_sibling_key() {
                Err(_) => {
                    assert_eq!(i, keys.len());
                    break;
                }
                Ok(k) => {
                    assert_eq!(k.1, String::from(keys[i]));
                    i += 1;
                }
            }
        }
    }

    #[test]
    fn test_next_sibling_key_for_level1() {
        let mut walker = JsonWalker::new(StringReader::new(CORRECT_JSON.to_string()), 50);
        let _ = walker.next_key_by_name("key71");
        let keys = [
            "key71", "key72", "key73", "key74", "key75", "key76", "key78", "key79", "key710",
        ];
        let mut i = 1;
        loop {
            match walker.next_sibling_key() {
                Err(_) => {
                    assert_eq!(i, keys.len());
                    break;
                }
                Ok(k) => {
                    assert_eq!(k.1, String::from(keys[i]));
                    i += 1;
                }
            }
        }
    }

    #[test]
    fn test_next_sibling_key_for_level2() {
        let mut walker = JsonWalker::new(StringReader::new(CORRECT_JSON.to_string()), 50);
        let _ = walker.next_key_by_name("key81");
        let keys = [
            "key81", "key82", "key83", "key84", "key85", "key86", "key89", "key810",
        ];
        let mut i = 1;
        loop {
            match walker.next_sibling_key() {
                Err(_) => {
                    assert_eq!(i, keys.len());
                    break;
                }
                Ok(k) => {
                    assert_eq!(k.1, String::from(keys[i]));
                    i += 1;
                }
            }
        }
    }

    #[test]
    fn test_next_child_key() {
        let mut walker = JsonWalker::new(StringReader::new(CORRECT_JSON.to_string()), 50);
        let _ = walker.next_key_by_name("key1");// seek key1
        let item = walker.next_child_key();
        assert_eq!(item, Ok((ValueType::Str, "key71".to_string())))
    }

    #[test]
    fn test_next_key_from_parent() {
        let mut walker = JsonWalker::new(StringReader::new(CORRECT_JSON.to_string()), 50);
        let _ = walker.next_key_by_name("key71");// seek key1
        let item = walker.next_key_from_parent();
        assert_eq!(item, Ok((ValueType::Str, "key8".to_string())))
    }

    #[test]
    fn test_next_item_by_pattern_some_items_in_middle() {
        let json = r#"[{"key1":"key1","key2":10},[{"key1":null, "key3":100}],"key1"]"#;
        let mut walker = JsonWalker::new(StringReader::new(json.to_string()), 50);
        let pattern = vec![|cs: &CurrentState| -> bool { cs.current_item.1.eq("key1") }];

        for _ in 0..4 {
            let item = walker.next_item_by_pattern(&pattern);
            assert_eq!(item, Ok((ValueType::Str, String::from("key1"))));
        }

        let item = walker.next_item_by_pattern(&pattern);
        assert_eq!(item, Err(Error::new_eos()));
    }

    #[test]
    fn test_next_item_by_pattern_item_in_path() {
        let json = r#"[{"key1":{"key4":100},"key2":10},[{"key1":{"key4":300}, "key3":100}],"key1"]"#;
        let mut walker = JsonWalker::new(StringReader::new(json.to_string()), 50);
        let pattern = vec![
            |cs: &CurrentState| -> bool { cs.level == 2.0 && cs.nth_occurrence == 0 },
            |cs: &CurrentState| -> bool { cs.latest_key.eq("key1") && cs.level == 3.0 },
            |cs: &CurrentState| -> bool { cs.latest_key.eq("key4") },
        ];

        let item = walker.next_item_by_pattern(&pattern);
        assert_eq!(item, Ok((ValueType::Str, String::from("key4"))));
    }

    #[test]
    fn test_next_item_by_level() {
        let json = r#"[{"key1":{"key4":100},"key2":10},[{"key1":{"key4":300}, "key3":100}],"key1"]"#;
        let mut walker = JsonWalker::new(StringReader::new(json.to_string()), 50);
        let item = walker.next_item_by_level(2.0);
        assert_eq!(item, Ok((ValueType::Str, String::from("key1"))));

        let item = walker.next_item_by_level(4.0);
        assert_eq!(item, Ok((ValueType::Str, String::from("key4"))));
    }

    #[test]
    fn test_current_value() {
        let item = |v: &str, is_str: bool| -> Content {
            Content::Simple((
                if is_str {
                    ValueType::Str
                } else {
                    ValueType::Int
                },
                String::from(v),
            ))
        };

        let object = |d: Vec<(&str, Content)>| -> Content {
            let mut o = BTreeMap::new();
            for x in d {
                o.insert(String::from(x.0), x.1);
            }
            Content::Object(o)
        };

        let array = |d: Vec<Content>| -> Content { Content::Array(d) };

        let s = r#"[{"key1" :{"key4" :100 },"key2" :10 },[{"key1" :{"key4" :300}, "key3":100}],"key1"]"#;

        // fetch all
        let mut walker = JsonWalker::new(StringReader::new(s.to_string()), 50);
        let a = walker.current_value_content();
        assert_eq!(
            a,
            Ok(array(vec![
                object(vec![
                    ("key1", object(vec![("key4", item("100", false))])),
                    ("key2", item("10", false)),
                ]),
                array(vec![object(vec![
                    ("key1", object(vec![("key4", item("300", false))])),
                    ("key3", item("100", false)),
                ])]),
                item("key1", true),
            ]))
        );

        // fetch only first item
        let mut walker = JsonWalker::new(StringReader::new(s.to_string()), 50);
        walker.move_n_element_forward(1);
        let a = walker.current_value_content();
        assert_eq!(
            a,
            Ok(object(vec![
                ("key1", object(vec![("key4", item("100", false))])),
                ("key2", item("10", false)),
            ]))
        );

        // fetch only a simple item
        let mut walker = JsonWalker::new(StringReader::new(s.to_string()), 50);
        let _ = walker.next_key_by_name("key2");
        let a = walker.current_value_content();
        assert_eq!(a, Ok(item("10", false)));
    }

    #[test]
    fn test_json_file() {}
}

#[cfg(test)]
#[cfg(feature = "deserialize")]
mod walker_test_de {
    use crate::json_walker::JsonWalker;
    use crate::json_walker::walker_test_de::data1::MixedDataTypes;
    use crate::json_walker::walker_test_de::data2::Person;
    use crate::readers::StringReader;

    mod data1 {
        use serde::{Deserialize, Serialize};

        pub fn create_data() -> MixedDataTypes {
            MixedDataTypes {
                null: None,
                unsigned: Some(1),
                int: [2, -2],
                float1: [3.3, -3.3],
                character: 'g',
                boolean: false,
                string: "Hello".into(),
                bytes: vec![4, b'h'],
                tuple: (5, 6.7, 'b', None, vec![8], Color::Red, Point { x: 10, y: -10 }),
                array1: vec![('l', "world".into()), ('x', "oops".into())],
                array2: vec![Point { x: 11, y: -11 }, Point { x: 12, y: -12 }],
                array3: vec![Color::Green, Color::Blue],
                enum1: Message::Quit,
                enum2: Message::Move { x: 13, y: -13 },
                enum3: Message::Write("This is a test".into()),
                enum4: Message::ChangeColor(Color::Green, Point { x: 14, y: -14 }),
            }
        }

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        pub struct MixedDataTypes {
            pub null: Option<usize>,
            pub unsigned: Option<usize>,
            pub int: [i32; 2],
            pub float1: [f32; 2],
            pub character: char,
            pub boolean: bool,
            pub string: String,
            pub bytes: Vec<u8>,
            pub tuple: (i32, f32, char, Option<String>, Vec<i32>, Color, Point),
            pub array1: Vec<(char, String)>,
            pub array2: Vec<Point>,
            pub array3: Vec<Color>,
            pub enum1: Message,
            pub enum2: Message,
            pub enum3: Message,
            pub enum4: Message,
        }

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        pub enum Message {
            Quit,
            Move { x: i32, y: i32 },
            Write(String),
            ChangeColor(Color, Point),
        }

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        pub enum Color {
            Red,
            Green,
            Blue,
        }

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        pub struct Point {
            pub x: i32,
            pub y: i32,
        }
    }

    mod data2 {
        use serde::{Deserialize, Serialize};

        pub fn create_data() -> Vec<Person> {
            vec![
                Person {
                    name: "John Doe".to_string(),
                    age: -30,
                    unsigned_age: 25,
                    address: Address {
                        street: "123 Main St".to_string(),
                        city: "New York".to_string(),
                        country: "USA".to_string(),
                    },
                    hobbies: vec!["reading", "painting", "hiking"].iter().map(|s| s.to_string()).collect(),
                    favorite_color: Color::Blue,
                    height: 1.75,
                    weight: -65.5,
                    friends: Some(vec![
                        (Friend { name: "Alice".to_string(), age: -28 }, true, Address {
                            street: "oh fuck".to_string(),
                            city: "Laas".to_string(),
                            country: "USA".to_string(),
                        },
                         vec!["0123456".to_string()]
                        ),
                        (Friend { name: "Bob".to_string(), age: -32 }, false, Address {
                            street: "shit".to_string(),
                            city: "goh".to_string(),
                            country: "USA".to_string(),
                        },
                         vec!["0123459846".to_string(), "54654032".to_string()]),
                    ]),
                    is_iranian: false,
                },
                Person {
                    name: "Arash".to_string(),
                    age: 36,
                    unsigned_age: 10,
                    address: Address {
                        street: "123 Main St".to_string(),
                        city: "Hamedan-Hamedan".to_string(),
                        country: "Iran".to_string(),
                    },
                    hobbies: vec!["gaming", "mount climbing", "bicycle"].iter().map(|s| s.to_string()).collect(),
                    favorite_color: Color::Green,
                    height: 164.1,
                    weight: -82.3,
                    friends: None,
                    is_iranian: true,
                },
            ]
        }

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        pub enum Color {
            Red,
            Green,
            Blue,
        }

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        pub struct Person {
            name: String,
            age: i32,
            unsigned_age: u32,
            address: Address,
            hobbies: Vec<String>,
            favorite_color: Color,
            height: f64,
            weight: f64,
            // Option<Vec<(Friend, is_close, Address, Vec<phones>)>>
            friends: Option<Vec<(Friend, bool, Address, Vec<String>)>>,
            is_iranian: bool,

        }

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        pub struct Address {
            street: String,
            city: String,
            country: String,
        }

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        pub struct Friend {
            name: String,
            age: i32,
        }
    }

    #[test]
    fn test_data1_de() {
        let data = data1::create_data();
        let json = serde_json::to_string(&data).unwrap();
        let mut walker = JsonWalker::new(StringReader::new(json), 50);
        let de = walker.current_value::<MixedDataTypes>().unwrap();
        assert_eq!(de, data);
    }

    #[test]
    fn test_data2_de() {
        let data = data2::create_data();
        let json = serde_json::to_string(&data).unwrap();
        let mut walker = JsonWalker::new(StringReader::new(json), 50);
        let de = walker.current_value::<Vec<Person>>().unwrap();
        assert_eq!(de, data);
    }
}
