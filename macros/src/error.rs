use std::{
    error::Error,
    fmt::{self, Formatter},
};

#[derive(Debug)]
pub struct AlipayError<'a>(&'a str);

impl<'a> AlipayError<'a> {
    pub fn new<S: Into<&'a str>>(msg: S) -> Self {
        AlipayError(msg.into())
    }
}

impl fmt::Display for AlipayError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "alipay: {}", self.0)
    }
}

impl Error for AlipayError<'_> {}
