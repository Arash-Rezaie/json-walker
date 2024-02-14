#[cfg(feature = "deserialize")]
pub mod deserialize_mod {
    use std::fmt::Display;
    use std::num::{ParseFloatError, ParseIntError};
    use std::str::ParseBoolError;

    use serde::de;

    use crate::{Error, ErrorKind, NIL};
    use crate::parser_core::{get_current_level, Item, Parser, TextItem, ValueType, walk_forward};

    //region error
    impl de::StdError for Error {}

    impl de::Error for Error {
        fn custom<T>(msg: T) -> Self where T: Display {
            Error { kind: ErrorKind::Serde, msg: msg.to_string() }
        }
    }

    impl From<ParseBoolError> for Error {
        fn from(value: ParseBoolError) -> Self {
            Error { kind: ErrorKind::ParseBoolError, msg: value.to_string() }
        }
    }

    impl From<ParseIntError> for Error {
        fn from(value: ParseIntError) -> Self {
            Error { kind: ErrorKind::ParseIntError, msg: value.to_string() }
        }
    }

    impl From<ParseFloatError> for Error {
        fn from(value: ParseFloatError) -> Self {
            Error { kind: ErrorKind::ParseFloatError, msg: value.to_string() }
        }
    }
    //endregion

    //region Deserializer
    pub struct Deserializer<'md> {
        parser: &'md mut Parser,
    }

    impl<'md> Deserializer<'md> {
        pub fn new(parser: &'md mut Parser) -> Self {
            Deserializer { parser }
        }

        fn move_forward(&mut self) {
            if self.parser.next_byte != NIL {
                walk_forward(&mut self.parser);
            }
            // println!("{}", get_current_status(self.parser));
        }

        fn next_item(&mut self) -> Result<Item, Error> {
            while self.parser.next_byte != NIL {
                match walk_forward(&mut self.parser) {
                    TextItem::Key(i) | TextItem::Value(i) => {
                        // println!("{}", get_current_status(self.parser));
                        return Ok(i);
                    }
                    _ => {
                        // println!("{}", get_current_status(self.parser));
                    }
                }
            }
            Err(Error::new_eos())
        }
    }

    // Read de::Deserializer own doc. It has a lot of explanation and a link to a sample. At writing this code, it was https://serde.rs/impl-deserializer.html
    impl<'de: 'md, 'md> de::Deserializer<'de> for &'md mut Deserializer<'de> {
        type Error = Error;

        fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            loop {
                match walk_forward(&mut self.parser) {
                    TextItem::Key(i) | TextItem::Value(i) => {
                        return match i.0 {
                            ValueType::Null => { visitor.visit_none() }
                            ValueType::Bool => { visitor.visit_bool(i.1.parse()?) }
                            ValueType::Int => {
                                if i.1.starts_with('-') {
                                    visitor.visit_i128(i.1.parse()?)
                                } else {
                                    visitor.visit_u128(i.1.parse()?)
                                }
                            }
                            ValueType::Float => { visitor.visit_f64(i.1.parse()?) }
                            ValueType::Str => { visitor.visit_string(i.1) }
                            ValueType::Arr => { visitor.visit_seq(SeqAccessor::new(self)) }
                            ValueType::Obj => { visitor.visit_map(MapAccessor::new(self)) }
                        };
                    }
                    _ => {}
                }
            }
        }

        fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_bool(self.next_item()?.1.parse()?)
        }

        fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_i8(self.next_item()?.1.parse()?)
        }

        fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_i16(self.next_item()?.1.parse()?)
        }

        fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_i32(self.next_item()?.1.parse()?)
        }

        fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_i64(self.next_item()?.1.parse()?)
        }

        fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_u8(self.next_item()?.1.parse()?)
        }

        fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_u16(self.next_item()?.1.parse()?)
        }

        fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_u32(self.next_item()?.1.parse()?)
        }

        fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_u64(self.next_item()?.1.parse()?)
        }

        fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_f32(self.next_item()?.1.parse()?)
        }

        fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_f64(self.next_item()?.1.parse()?)
        }

        fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_char(self.next_item()?.1.chars().next().ok_or(Error { kind: ErrorKind::WrongDataType, msg: "Expecting a string or a char".into() })?)
        }

        fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_str(&self.next_item()?.1)
        }

        fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_string(self.next_item()?.1)
        }

        fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_bytes(self.next_item()?.1.as_bytes())
        }

        fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_byte_buf(self.next_item()?.1.into_bytes())
        }

        fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            if self.parser.next_byte == b':' || self.parser.next_byte == b',' {
                self.move_forward();
            }
            if self.parser.next_byte == b'n' {
                _ = self.next_item();
                visitor.visit_none()
            } else {
                visitor.visit_some(self)
            }
        }

        fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_unit()
        }

        fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_unit()
        }

        fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_newtype_struct(self)
        }

        fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_seq(SeqAccessor::new(self))
        }

        fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            let result = self.deserialize_seq(visitor);
            self.move_forward();
            result
        }

        fn deserialize_tuple_struct<V>(self, _name: &'static str, _len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            let result = self.deserialize_seq(visitor);
            self.move_forward();
            result
        }

        fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_map(MapAccessor::new(self))
        }

        fn deserialize_struct<V>(self, _name: &'static str, _fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            self.deserialize_map(visitor)
        }

        fn deserialize_enum<V>(self, _name: &'static str, _variants: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            visitor.visit_enum(VariantAccessor { de: self })
        }

        fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            self.deserialize_string(visitor)
        }

        fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            self.deserialize_any(visitor)
        }
    }
//endregion

    //region Accessors
    fn handle_next_element_seed<'de, T>(de: &mut Deserializer<'de>, working_level: f32, seed: T) -> Result<Option<T::Value>, Error> where T: de::DeserializeSeed<'de> {
        let mut current_level;
        while de.parser.next_byte == b']' || de.parser.next_byte == b'}' {
            current_level = get_current_level(de.parser);
            if working_level == current_level {// cursor of parser is synced with the deserializer function calls
                de.move_forward();
                return Ok(None);
            } else if working_level < current_level {// some deserializer function have returned early without any cursor move
                loop {
                    de.move_forward();
                    current_level = get_current_level(de.parser);
                    if working_level == current_level {
                        break;
                    }
                }
            } else {// some deserializer function have not returned yet byt cursor has moved extra
                return Err(Error { kind: ErrorKind::OOPS, msg: "Strange situation".into() });
            }
        }
        seed.deserialize(&mut *de).map(Some)
    }

    fn move_to_scope(de: &mut Deserializer, desired_byte: u8) {
        while de.parser.next_byte != b'{' && de.parser.next_byte != b'[' {
            de.move_forward();
        }
        if de.parser.next_byte == desired_byte {
            de.move_forward();
        }
    }

    struct SeqAccessor<'md, 'de: 'md> {
        de: &'md mut Deserializer<'de>,
        level: f32,
    }

    impl<'md, 'de> SeqAccessor<'md, 'de> {
        fn new(de: &'md mut Deserializer<'de>) -> Self {
            move_to_scope(de, b'[');
            let level = get_current_level(de.parser);
            SeqAccessor { de, level }
        }
    }


    impl<'md, 'de> de::SeqAccess<'de> for SeqAccessor<'md, 'de> {
        type Error = Error;

        fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> where T: de::DeserializeSeed<'de> {
            handle_next_element_seed(self.de, self.level, seed)
        }
    }

    struct MapAccessor<'md, 'de: 'md> {
        de: &'md mut Deserializer<'de>,
        level: f32,
    }

    impl<'md, 'de> MapAccessor<'md, 'de> {
        fn new(de: &'md mut Deserializer<'de>) -> Self {
            move_to_scope(de, b'{');
            let level = get_current_level(de.parser);
            MapAccessor { de, level }
        }
    }

    impl<'md, 'de> de::MapAccess<'de> for MapAccessor<'md, 'de> {
        type Error = Error;

        fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error> where K: de::DeserializeSeed<'de> {
            handle_next_element_seed(self.de, self.level, seed)
        }

        fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error> where V: de::DeserializeSeed<'de> {
            seed.deserialize(&mut *self.de)
        }
    }

    struct VariantAccessor<'md, 'de: 'md> {
        de: &'md mut Deserializer<'de>,
    }

    impl<'md, 'de> de::EnumAccess<'de> for VariantAccessor<'md, 'de> {
        type Error = Error;
        type Variant = Self;

        fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error> where V: de::DeserializeSeed<'de> {
            Ok((seed.deserialize(&mut *self.de)?, self))
        }
    }


    impl<'md, 'de> de::VariantAccess<'de> for VariantAccessor<'md, 'de> {
        type Error = Error;

        fn unit_variant(self) -> Result<(), Self::Error> {
            de::Deserialize::deserialize(self.de)
        }

        fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error> where T: de::DeserializeSeed<'de> {
            seed.deserialize(self.de)
        }

        fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            de::Deserializer::deserialize_seq(self.de, visitor)
        }

        fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> where V: de::Visitor<'de> {
            de::Deserializer::deserialize_map(self.de, visitor)
        }
    }
    //endregion
}