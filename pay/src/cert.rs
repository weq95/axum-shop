use openssl::{
    hash::{hash, MessageDigest},
    nid::Nid,
    x509::{X509NameEntries, X509},
};

use crate::PayResult;

pub struct CertX509;

impl CertX509 {
    pub fn new() -> Self {
        CertX509
    }

    pub fn cert_sn(&self, cert_content: &str) -> PayResult<String> {
        let ssl = X509::from_pem(cert_content.as_bytes())?;
        let issuer = self.to_string(ssl.issuer_name().entries())?;
        let serial_number = ssl.serial_number().to_bn()?.to_dec_str()?;
        let result = issuer + &serial_number;

        Ok(hex::encode(hash(MessageDigest::md5(), result.as_ref())?))
    }

    pub fn root_cert_sn(&self, root_cert_content: &str) -> PayResult<String> {
        Ok(root_cert_content
            .split_inclusive("-----END CERTIFICATE-----")
            .filter(|cert| {
                let ssl = X509::from_pem(cert.as_ref()).unwrap();
                let algorithm = ssl.signature_algorithm().object().nid();

                algorithm == Nid::SHA256WITHRSAENCRYPTION || algorithm == Nid::SHA1WITHRSAENCRYPTION
            })
            .filter_map(|cert| self.cert_sn(cert.as_ref()).ok())
            .collect::<Vec<String>>()
            .join("_"))
    }

    pub fn to_string(&self, entries: X509NameEntries) -> PayResult<String> {
        let mut value = String::new();
        for i in entries {
            let key = i.object().nid().short_name()?.to_owned();
            let val = i.data().as_utf8()?.to_owned();

            value.insert_str(0, &(key + "=" + &val + ","));
        }

        value.pop();

        Ok(value)
    }
}
