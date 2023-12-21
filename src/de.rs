use std::collections::HashMap as Map;

use icu_casemap::CaseMapper;
use serde::de::{DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess,
                VariantAccess, Visitor};
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
    Ok(t)
}

pub struct Deserializer<'de> {
    it:          Iter<'de>,
    case_mapper: CaseMapper,
    known_keys:  Option<Map<String, &'static str>>,
}

impl<'de> Deserializer<'de> {
    pub fn from_str(input: &'de str) -> Self {
        Deserializer {
            it:          iter_from_str(input),
            case_mapper: CaseMapper::new(),
            known_keys:  None,
        }
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> { todo!("any 1") }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> { todo!("unit") }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        todo!("unit_struct")
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
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
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.known_keys = Some(
            fields
                .iter()
                .map(|&s| (self.case_mapper.fold_string(s), s))
                .collect(),
        );
        let r = self.deserialize_map(visitor);
        self.known_keys = None;
        r
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
            Some(Err(err)) => return Err(self.it.next().unwrap().unwrap_err()),
            Some(Ok(Some((_, key, _)))) => seed
                .deserialize(DeserializerKey { key: key.clone() })
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
        seed.deserialize(&mut DeserializerValue { value })
    }
}

pub struct DeserializerKey {
    key: String,
}
impl<'de, 'a> de::Deserializer<'de> for DeserializerKey {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> { todo!() }

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

pub struct DeserializerValue<'de> {
    value: Vec<&'de str>,
}
impl<'de, 'a> de::Deserializer<'de> for &'a mut DeserializerValue<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> { todo!() }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        println!(" - visit value string {:?}", &self.value.join("\n"));
        visitor.visit_string(self.value.join("\n"))
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        println!(" - ignored any");
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str
        // string
        bytes byte_buf option unit unit_struct newtype_struct seq
        tuple tuple_struct map struct enum identifier
        // ignored_any
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

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
        struct Test {
            origin: String,
            #[serde(flatten)]
            other:  BTreeMap<String, String>,
        }
        let test: Test = from_str(S).unwrap();
        dbg!(test);
        panic!();
    }
}
