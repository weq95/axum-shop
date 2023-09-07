use std::collections::HashMap;
use std::error::Error;
use std::io::Error as IoError;

pub use alipay::AliPay;
#[cfg(not(target_os = "windows"))]
use openssl::{
    base64,
    error::ErrorStack,
    hash::MessageDigest,
    pkey::PKey,
    rsa::Rsa,
    sign::{Signer, Verifier},
};

mod alipay;
mod cert;

pub type PayResult<T> = Result<T, PayError>;

#[derive(Debug)]
pub enum PayError {
    Err(String),
    ErrArr(Vec<String>),
    ErrMap(HashMap<String, String>),
    ErrArrMap(Vec<HashMap<String, String>>),
}

impl PayError {
    pub fn new(err: &str) -> Self {
        PayError::Err(err.to_string())
    }
}

pub trait Sign {
    fn private_key(&self) -> &str;
    fn public_key(&self) -> &str;
    fn sign(&self, param: &str) -> PayResult<String> {
        #[cfg(not(target_os = "windows"))]
        {
            let content = base64::decode_block(self.private_key())?;
            let key = PKey::from_rsa(Rsa::private_key_from_der(&content)?)?;
            let mut signer = Signer::new(MessageDigest::sha256(), &key)?;
            signer.update(param.as_bytes())?;

            Ok(base64::encode_block(signer.sign_to_vec()?.as_ref()))
        }

        #[cfg(target_os = "windows")]
        Ok(String::new())
    }

    fn verify(&self, source: &str, signature: &str) -> PayResult<bool> {
        #[cfg(not(target_os = "windows"))]
        {
            let content = base64::decode_block(self.public_key())?;
            let key = PKey::from_rsa(Rsa::public_key_from_der(&content)?)?;

            let sign = base64::decode_block(signature)?;
            let mut verifier = Verifier::new(MessageDigest::sha256(), &key)?;
            verifier.update(source.as_bytes())?;

            Ok(verifier.verify(sign.as_slice())?)
        }

        #[cfg(target_os = "windows")]
        Ok(false)
    }
}

impl From<IoError> for PayError {
    fn from(value: IoError) -> Self {
        PayError::Err(value.to_string())
    }
}

#[cfg(not(target_os = "windows"))]
impl From<ErrorStack> for PayError {
    fn from(value: ErrorStack) -> Self {
        PayError::Err(value.to_string())
    }
}

impl From<reqwest::Error> for PayError {
    fn from(value: reqwest::Error) -> Self {
        PayError::Err(value.to_string())
    }
}
