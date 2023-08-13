use std::collections::HashMap;

use serde_json::{Map, Number, Value};

pub use crate::macros::*;

macro_rules! alipay_val_from_number {
    ($($ty: ty), *) => {
        $(
        impl From<$ty> for AlipayVal{
            fn from(value: $ty) -> Self{
                AlipayVal::Number(Number::from(value))
            }
        }
        )*
    };
}

alipay_val_from_number!(u8, u16, u32, u64);
alipay_val_from_number!(i8, i16, i32, i64);
alipay_val_from_number!(usize, isize);

macro_rules! alipay_param_impl_number {
    ($($ty: ty), *) => {
        $(
        impl AlipayParam for $ty{
            fn to_value(self) -> AlipayVal{
                AlipayVal::Number(Number::from(self))
            }
        }
        )*
    };
}

alipay_param_impl_number!(u8, u16, u32, u64);
alipay_param_impl_number!(i8, i16, i32, i64);
alipay_param_impl_number!(usize, isize);
pub trait AlipayParam {
    fn to_value(self) -> AlipayVal;
}

pub enum AlipayVal {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Tuple((String, Value)),
    TupleArray(Vec<(String, Value)>),
    Array(Vec<AlipayVal>),
    Object(HashMap<String, AlipayVal>),
}

impl From<bool> for AlipayVal {
    fn from(value: bool) -> Self {
        AlipayVal::Bool(value)
    }
}

impl From<()> for AlipayVal {
    fn from(_: ()) -> Self {
        AlipayVal::Null
    }
}

impl From<f64> for AlipayVal {
    fn from(value: f64) -> Self {
        if let Some(val) = Number::from_f64(value) {
            return AlipayVal::Number(val);
        }

        AlipayVal::Null
    }
}

impl From<String> for AlipayVal {
    fn from(value: String) -> Self {
        AlipayVal::String(value)
    }
}

impl<'a> From<&'a str> for AlipayVal {
    fn from(value: &'a str) -> Self {
        AlipayVal::String(value.to_string())
    }
}

impl<T: Clone + Into<Value>> From<(String, T)> for AlipayVal {
    fn from(value: (String, T)) -> Self {
        AlipayVal::Tuple((value.0, value.1.into()))
    }
}

impl From<Vec<AlipayVal>> for AlipayVal {
    fn from(value: Vec<AlipayVal>) -> Self {
        AlipayVal::Array(value)
    }
}

impl<T: Clone + Into<Value>> From<Vec<(String, T)>> for AlipayVal {
    fn from(value: Vec<(String, T)>) -> Self {
        let mut result: Vec<(String, Value)> = Vec::new();
        for (key, val) in value {
            result.push((key, val.into()));
        }

        AlipayVal::TupleArray(result)
    }
}

impl<T: Into<AlipayVal>> From<HashMap<String, T>> for AlipayVal {
    fn from(value: HashMap<String, T>) -> Self {
        let mut result: HashMap<String, AlipayVal> = HashMap::new();
        for (key, value) in value {
            result.insert(key, value.into());
        }

        AlipayVal::Object(result)
    }
}

impl<'a, T: Into<AlipayVal>> From<HashMap<&'a str, T>> for AlipayVal {
    fn from(value: HashMap<&'a str, T>) -> Self {
        let mut result: HashMap<String, AlipayVal> = HashMap::new();
        for (key, val) in value {
            result.insert(key.to_string(), val.into());
        }

        AlipayVal::Object(result)
    }
}

fn json_alipay_val(val: Value) -> AlipayVal {
    match val {
        Value::Null => AlipayVal::Null,
        Value::Bool(val) => AlipayVal::Bool(val),
        Value::Number(val) => AlipayVal::Number(val),
        Value::String(val) => AlipayVal::String(val),
        Value::Array(val) => {
            let mut array: Vec<AlipayVal> = Vec::new();
            for i in val {
                array.push(json_alipay_val(i));
            }
            AlipayVal::Array(array)
        }
        Value::Object(val) => {
            let mut object: HashMap<String, AlipayVal> = HashMap::new();

            for (key, val) in val {
                object.insert(key, json_alipay_val(val));
            }

            AlipayVal::Object(object)
        }
    }
}

impl AlipayVal {
    pub fn is_null(&self) -> bool {
        matches!(self, AlipayVal::Null)
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, AlipayVal::Bool(_))
    }

    pub fn is_number(&self) -> bool {
        matches!(self, AlipayVal::Number(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, AlipayVal::String(_))
    }

    pub fn is_tuple(&self) -> bool {
        matches!(self, AlipayVal::Tuple(_))
    }

    pub fn is_tuple_array(&self) -> bool {
        matches!(self, AlipayVal::TupleArray(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, AlipayVal::Array(_))
    }

    pub fn is_object(&self) -> bool {
        matches!(self, AlipayVal::Object(_))
    }

    pub fn to_json_val(self) -> Value {
        match self {
            AlipayVal::Null => Value::Null,
            AlipayVal::Bool(val) => Value::Bool(val),
            AlipayVal::Number(val) => Value::Number(val),
            AlipayVal::String(val) => Value::String(val),
            AlipayVal::Tuple((key, val)) => {
                let mut object = Map::new();
                object.insert(key, val);
                Value::Object(object)
            }
            AlipayVal::TupleArray(val) => {
                let mut object = Map::new();
                for (key, val) in val {
                    object.insert(key, val);
                }

                Value::Object(object)
            }
            AlipayVal::Array(val) => {
                let mut array = Vec::new();
                for i in val {
                    array.push(i.to_json_val());
                }

                Value::Array(array)
            }
            AlipayVal::Object(val) => {
                let mut object = Map::new();
                for (key, val) in val {
                    object.insert(key, val.to_json_val());
                }

                Value::Object(object)
            }
        }
    }
}

impl AlipayParam for f32 {
    fn to_value(self) -> AlipayVal {
        if let Some(val) = Number::from_f64(self as f64) {
            return AlipayVal::Number(val);
        }

        AlipayVal::Null
    }
}

impl AlipayParam for f64 {
    fn to_value(self) -> AlipayVal {
        if let Some(val) = Number::from_f64(self) {
            return AlipayVal::Number(val);
        }

        AlipayVal::Null
    }
}

impl AlipayParam for bool {
    fn to_value(self) -> AlipayVal {
        AlipayVal::Bool(self)
    }
}

impl AlipayParam for String {
    fn to_value(self) -> AlipayVal {
        AlipayVal::String(self)
    }
}

impl<'a> AlipayParam for &'a str {
    fn to_value(self) -> AlipayVal {
        AlipayVal::String(self.to_owned())
    }
}

impl<T: AlipayParam> AlipayParam for Option<T> {
    fn to_value(self) -> AlipayVal {
        if let Some(val) = self {
            return val.to_value();
        }

        AlipayVal::Null
    }
}

impl<T1, T2> AlipayParam for (T1, T2)
where
    T1: Into<String>,
    T2: AlipayParam,
{
    fn to_value(self) -> AlipayVal {
        AlipayVal::Tuple((self.0.into(), self.1.to_value().to_json_val()))
    }
}

impl<T, const N: usize> AlipayParam for [T; N]
where
    T: AlipayParam + Clone,
{
    fn to_value(self) -> AlipayVal {
        if self.is_empty() {
            return AlipayVal::Null;
        }

        let mut i = 0;
        let len = self.len();
        let temp = self[i].clone().to_value();
        if temp.is_tuple() {
            i += 1;
            let mut array = Vec::new();
            if let AlipayVal::Tuple((key, val)) = temp {
                array.push((key, val));
            }

            while len > i {
                let temp = self[i].clone().to_value();
                if temp.is_tuple() {
                    if let AlipayVal::Tuple((key, val)) = temp {
                        array.push((key, val));
                    }
                }

                i += 1;
            }

            return AlipayVal::TupleArray(array);
        }

        i += 1;
        let mut array = Vec::new();
        array.push(temp);
        while len > i {
            let temp = self[i].clone().to_value();
            array.push(temp);
            i += 1;
        }

        AlipayVal::Array(array)
    }
}

impl<T1, T2> AlipayParam for HashMap<T1, T2>
where
    T1: Into<String>,
    T2: AlipayParam,
{
    fn to_value(self) -> AlipayVal {
        let mut result = HashMap::new();
        for (key, val) in self {
            result.insert(key.into(), val.to_value());
        }

        AlipayVal::Object(result)
    }
}

impl AlipayParam for Value {
    fn to_value(self) -> AlipayVal {
        json_alipay_val(self)
    }
}

impl<T: AlipayParam> AlipayParam for Vec<T> {
    fn to_value(mut self) -> AlipayVal {
        if self.is_empty() {
            return AlipayVal::Null;
        }

        let mut i = 0;
        let len = self.len();
        let temp = self.remove(0).to_value();
        if temp.is_tuple() {
            i += 1;
            let mut array = Vec::new();
            if let AlipayVal::Tuple((key, val)) = temp {
                array.push((key, val));
            }

            while len > i {
                let temp = self.remove(0).to_value();
                if temp.is_tuple() {
                    if let AlipayVal::Tuple((key, val)) = temp {
                        array.push((key, val));
                    }
                }

                i += 1;
            }

            return AlipayVal::TupleArray(array);
        }

        i += 1;
        let mut array: Vec<AlipayVal> = Vec::new();
        array.push(temp);
        while len > i {
            let temp = self.remove(0).to_value();
            array.push(temp);
            i += 1;
        }

        AlipayVal::Array(array)
    }
}

impl AlipayParam for () {
    fn to_value(self) -> AlipayVal {
        AlipayVal::Null
    }
}

impl AlipayParam for AlipayVal {
    fn to_value(self) -> AlipayVal {
        self
    }
}
