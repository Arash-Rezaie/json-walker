# Rust Json_Walker

Parse json stream or text by this library. The main idea is to maintain a cursor and move it forward till the desired element be
found, so json can be partially visible.

Verification happens during parsing. If json is malformed, it will panic. Deserialization will return Error

- **Please notice that backward walking is not supported.**
- async operation is not supported by default, but there is a way to do that through channels. Check out the following examples. 
- "null", "true" & "false" are not supported in **capital** mode, If you need to have them, modify parser_core.rs file
  and
  uncomment lines which are marked with "**support_capital_word**" phrase (Both are placed in *extract_word()*
  function).
- To deserialize a part of json, you need to enable "**deserialize**" feature

### Features
**deserialize** -> enable deserialization via current_value() function 

### Some provided methods

> <span style="color:teal">**get_current_level**</span> -> if we consider json as a tree, nodes can have level</br>
> <span style="color:teal">**get_path**</span> -> the path of a node from tree root</br>
> <span style="color:teal">**seek_by_level_offset**</span> -> jump to the node by its level</br>
> <span style="color:teal">**next_item**</span> -> next key or value </br>
> <span style="color:teal">**next_key**</span> -> just get next key, no matter if it is in children nodes, parent node
> or siblings</br>
> <span style="color:teal">**next_key_by_name**</span> -> parse json till a specific key</br>
> <span style="color:teal">**next_sibling_key**</span> -> jump to the next sibling key </br>
> <span style="color:teal">**next_item_by_pattern**</span> -> if you are looking for a specific item, jump to it</br>
> <span style="color:teal">**current_value_content**</span> -> where ever the cursor is, the related value will be
> return as content </br>
> <span style="color:teal">**current_value**</span> -> where ever the cursor is, the related value will be returned as
> deserialized (enable "deserialize" feature for this one) </br>

# Example

```rust
use json_walker::json_walker::{Item, JsonWalker, ValueType, StringReader};

fn main() {
    // we are looking for "key2"
    let json = r#"{"key1": null, "key2": true, "key3": false, "key4": 111}"#;
    let mut walker = JsonWalker::from_string(StringReader::new(json.to_string()), 0);
    let result = walker.next_key_by_name("key2");
    assert_eq!(
        Some((ValueType::Str, String::from("key2"))),
        result
    );
}
```

<br/>
Finding the second "key4" by patterns. Pattern is an array of comparators.
In this example, some comparators are provided, but notice that the final performance is up to you.

```rust
use json_walker::json_walker::{CurrentState, Item, JsonWalker, ValueType};

fn main() {
    let json = r#"[{"key1":{"key4":100},"key2":10},[{"key1":{"key4":300}, "key3":100}],"key1"]"#;
    let mut walker = JsonWalker::from_string(StringReader::new(json.to_string()), 0);
    let pattern = vec![
        |cs: &CurrentState| -> bool{
            cs.level == 2.0 && cs.nth_occurrence == 0
        },
        |cs: &CurrentState| -> bool{
            cs.latest_key.eq("key1") && cs.level == 3.0
        },
        |cs: &CurrentState| -> bool{
            cs.latest_key.eq("key4")
        },
    ];

    let item = walker.next_item_by_pattern(&pattern);
    assert_eq!(item, Some((ValueType::Str, String::from("key4"))));
}
```

This library does not support async by default. It must be handled by Reader. The reader is a Box<Iterator<Item=u8>>.

The following is a sample of handling async reader:

```rust
use json_walker::json_walker::JsonWalker;

use crate::stream_reader::StreamReader;

#[tokio::main]
async fn main() {
  let reader = StreamReader::new("path to json file".into(), 10);
  let mut walker = JsonWalker::new(Box::new(reader), 0);
  loop {
    match walker.next_item() {
      Ok(t) => { println!("{:?}", t) }
      Err(_) => { break; }
    }
  }
}

mod stream_reader {
  use std::fs::File;
  use std::io::{BufRead, BufReader};
  use std::sync::mpsc::{Receiver, sync_channel, SyncSender};

  // read target file line by line asynchronously
  async fn read_line(reader: &mut BufReader<File>) -> Result<String, ()> {
    let mut buffer = String::new();
    return match reader.read_line(&mut buffer) {
      Ok(_) => { Ok(buffer) }
      _ => { Err(()) }
    };
  }

  pub struct StreamReader {
    buffer_slice: Vec<u8>,
    buffer_slice_len: usize,
    index: usize,
    consumer: Receiver<Vec<u8>>,
  }

  impl StreamReader {
    pub fn new(file_path: String, queue_size: usize) -> Self {
      // std::..::sync_channel makes a queue and that queue will be filled to the queue_size.
      // receiver will wait till there is some data ready to use, otherwise no blocking happens
      let (producer, consumer) = sync_channel(queue_size);
      StreamReader::start_thread(file_path, producer);
      StreamReader {
        buffer_slice: vec![],
        buffer_slice_len: 0,
        index: 0,
        consumer,
      }
    }

    // fill sender queue in this thread
    fn start_thread(file_path: String, producer: SyncSender<Vec<u8>>) {
      tokio::spawn(async move {
        let file = File::open(file_path.clone()).expect(&format!("file: {} does not exist", file_path));
        let mut reader = BufReader::new(file);
        loop {
          // read_line needs await mechanism
          match read_line(&mut reader).await {
            Ok(line) => {
              match producer.send(line.into_bytes()) {
                Err(_) => break, // maybe receiver is dismissed
                _ => {}
              }
            }
            Err(_) => {
              panic!("reading file failed");
            }
          };
        }
      });
    }
  }

  impl Iterator for StreamReader {
    type Item = u8;

    // provide bytes
    fn next(&mut self) -> Option<Self::Item> {
      // if current piece of data is finished, catch another one
      if self.index == self.buffer_slice_len {
        match self.consumer.recv() {
          Ok(d) if d.len() > 0 => {
            self.buffer_slice = d;
            self.buffer_slice_len = self.buffer_slice.len();
            self.index = 0;
          }
          _ => { return None; }
        }
      }
      let i = self.index;
      self.index += 1;
      Some(self.buffer_slice[i])
    }
  }
}
```

- Other examples are provided in function docs and tests