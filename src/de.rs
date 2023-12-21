use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{de, forward_to_deserialize_any, Deserialize};

use crate::error::{Error, Result};
use crate::naive::{iter_from_str, Iter};

pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(s);
    let t = T::deserialize(&mut deserializer)?;
    // TODO: check if all data are consumed
    assert!(dbg!(deserializer.it.next()).is_none());
    Ok(t)
}

pub struct Deserializer<'de> {
    it: Iter<'de>,
}

impl<'de> Deserializer<'de> {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &'de str) -> Self {
        Deserializer {
            it: iter_from_str(input),
        }
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> { todo!("any 1") }

    fn deserialize_unit<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> { todo!("unit") }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value> {
        todo!("unit_struct")
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value> {
        todo!("newtype_struct")
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        println!(" - seq");
        let value = visitor.visit_seq(&mut *self)?;

        match self.it.next() {
            None => Ok(value),
            Some(Ok(None)) => Ok(value),
            _ => Err(Error::ExpectedStanzaEnd),
        }
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        println!(" - map");
        let value = visitor.visit_map(&mut *self)?;

        match self.it.next() {
            None => Ok(value),
            Some(Ok(None)) => Ok(value),
            _ => Err(Error::ExpectedStanzaEnd),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option
        // unit unit_struct newtype_struct seq
        tuple tuple_struct
        // map struct
        enum identifier ignored_any
    }
}

impl<'de, 'a> SeqAccess<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        match self.it.peek() {
            None => Ok(None),
            Some(Ok(None)) => Ok(None),
            _ => seed.deserialize(&mut **self).map(Some),
        }
    }
}

impl<'de, 'a> MapAccess<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.it.peek() {
            None | Some(Ok(None)) => Ok(None),
            Some(Err(_err)) => Err(self.it.next().unwrap().unwrap_err()),
            Some(Ok(Some((_, key, _)))) => seed
                .deserialize(DeserializerKey {
                    key: key.to_lowercase(),
                })
                .map(Some),
        }
    }
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        let value = self
            .it
            .next()
            .expect("next after peek need to be valid")
            .expect("Error already returned if present")
            .expect("end string already returned if present")
            .2;
        seed.deserialize(&mut DeserializerValue {
            value,
            mode: ValueMod::None,
        })
    }
}

pub struct DeserializerKey {
    key: String,
}
impl<'de> de::Deserializer<'de> for DeserializerKey {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> { todo!() }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        println!(" - visit key {}", &self.key);
        visitor.visit_string(self.key)
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        println!(" - visit identifier {}", &self.key);
        visitor.visit_string(self.key)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str
        // string
        bytes byte_buf option unit unit_struct newtype_struct
        seq tuple tuple_struct map struct enum
        // identifier
        ignored_any
    }
}

enum ValueMod {
    None,
    Seq,
    SeqOfTupple(usize),
}
pub struct DeserializerValue<'de> {
    value: Vec<&'de str>,
    mode:  ValueMod,
}
impl<'de> DeserializerValue<'de> {
    fn trim_start(&mut self) {
        while !self.value.is_empty() && self.value[0].trim().is_empty() {
            self.value.remove(0);
        }
    }
    fn is_empty(&self) -> bool { self.value.is_empty() }
    fn next_token(&mut self) -> &'de str {
        match self.mode {
            ValueMod::None => panic!(),
            ValueMod::SeqOfTupple(1) => {
                if self.value.is_empty() {
                    panic!();
                }
                let ret = self.value[0].trim();
                self.value[0] = "";
                ret
            }
            ValueMod::Seq | ValueMod::SeqOfTupple(_) => {
                if self.value.is_empty() {
                    panic!();
                }
                self.value[0] = self.value[0].trim_start();
                let ret;
                (ret, self.value[0]) = self.value[0]
                    .split_once(char::is_whitespace)
                    .unwrap_or((self.value[0], ""));
                ret
            }
        }
    }
}
impl<'de, 'a> de::Deserializer<'de> for &'a mut DeserializerValue<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        //self.deserialize_string(visitor)
        todo!("any")
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let i = self.next_token().parse::<u64>().unwrap();
        println!(" - visit value u64 {i:?}");
        visitor.visit_u64(i)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.mode {
            ValueMod::None => {
                println!(" - visit value string {:?}", &self.value.join("\n"));
                visitor.visit_string(self.value.join("\n"))
            }
            ValueMod::Seq | ValueMod::SeqOfTupple(_) => {
                let ret = self.next_token();
                println!(" - visit value string {ret:?}");
                visitor.visit_str(ret)
            }
        }
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        println!(" - value seq");
        assert!(matches!(self.mode, ValueMod::None));

        self.mode = ValueMod::Seq;
        let r = visitor.visit_seq(&mut *self);
        self.mode = ValueMod::None;
        r
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        match self.mode {
            ValueMod::Seq => {
                self.mode = ValueMod::SeqOfTupple(len);
                let r = visitor.visit_seq(&mut *self);
                self.mode = ValueMod::Seq;
                r
            }
            _ => panic!(),
        }
    }

    fn deserialize_map<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> { todo!("map") }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        println!(" - ignored any");
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32
        // u64
        u128 f32 f64 char str
        // string
        bytes byte_buf option unit unit_struct newtype_struct
        // seq tuple
        tuple_struct
        // map
        struct enum identifier
        // ignored_any
    }
}
impl<'de, 'a> SeqAccess<'de> for &'a mut DeserializerValue<'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        match self.mode {
            ValueMod::Seq => {
                self.trim_start();
                if self.is_empty() {
                    Ok(None)
                } else {
                    seed.deserialize(&mut **self).map(Some)
                }
            }
            ValueMod::SeqOfTupple(0) => Ok(None),
            ValueMod::SeqOfTupple(rest) => {
                let r = seed.deserialize(&mut **self).map(Some);
                self.mode = ValueMod::SeqOfTupple(rest - 1);
                r
            }
            ValueMod::None => panic!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde::Deserialize;

    use super::from_str;
    const S: &str = r#"



Origin: Debian
Architectures: all amd64 arm64 armel armhf i386 mips64el ppc64el riscv64 s390x
Components: main contrib non-free-firmware non-free
Description: Experimental packages - not released; use at your own risk.
MD5Sum:
 3cc222d6694b2de9734c081122a17cb3  3030586 contrib/Contents-all
 1f7d9d3e63b59533f6f5dadc83e71cc7    63339 contrib/Contents-all.diff/Index
 aa5dc8f6f4ab68b4e5b76df04a0532c4   291019 contrib/Contents-all.gz
 55a5553654b03c6a75cd61f79a31257e   271634 contrib/Contents-amd64
 ed5005daa6257830e623e78691c29475    63339 contrib/Contents-amd64.diff Index


Origin: Debian
Architectures: all amd64 arm64 armel armhf i386 mips64el ppc64el riscv64 s390x
Components: main contrib non-free-firmware non-free
Description: Experimental packages - not released; use at your own risk.
MD5Sum:
 3cc222d6694b2de9734c081122a17cb3  3030586 contrib/Contents-all
 1f7d9d3e63b59533f6f5dadc83e71cc7    63339 contrib/Contents-all.diff/Index
 aa5dc8f6f4ab68b4e5b76df04a0532c4   291019 contrib/Contents-all.gz
 55a5553654b03c6a75cd61f79a31257e   271634 contrib/Contents-amd64
 ed5005daa6257830e623e78691c29475    63339 contrib/Contents-amd64.diff/Index

Origin: Debian
Architectures: all amd64 arm64 armel armhf i386 mips64el ppc64el riscv64 s390x
Components: main contrib non-free-firmware non-free
Description: Experimental packages - not released; use at your own risk.
MD5Sum:
 3cc222d6694b2de9734c081122a17cb3  3030586 contrib/Contents-all
 1f7d9d3e63b59533f6f5dadc83e71cc7    63339 contrib/Contents-all.diff/Index
 aa5dc8f6f4ab68b4e5b76df04a0532c4   291019 contrib/Contents-all.gz
 55a5553654b03c6a75cd61f79a31257e   271634 contrib/Contents-amd64
 ed5005daa6257830e623e78691c29475    63339 contrib/Contents-amd64.diff/Index

"#;

    #[test]
    fn test_file() {
        let data: Vec<BTreeMap<String, String>> = from_str(S).unwrap();
        dbg!(data);
    }

    #[test]
    fn test_struct() {
        #[derive(Deserialize, Debug)]
        #[serde(rename_all = "kebab-case")]
        #[allow(dead_code)]
        struct Test {
            origin:        String,
            description:   String,
            architectures: Vec<String>,
            components:    Vec<String>,
            md5sum:        Vec<(String, u64, String)>,
            //#[serde(flatten)]
            //other:         BTreeMap<String, String>,
        }
        let _test: Vec<Test> = from_str(S).unwrap();
    }
}
