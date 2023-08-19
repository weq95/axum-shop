use std::collections::HashMap;

use serde_json::json;

use crate::cert::CertX509;
use crate::{PayResult, Sign};

pub struct AliPay<'a> {
    public_key: &'a str,
    private_key: &'a str,
    request: HashMap<&'a str, String>,
    biz_content: HashMap<&'a str, String>,
    sandbox: bool,
}

impl Sign for AliPay<'_> {
    fn private_key(&self) -> &str {
        self.private_key
    }

    fn public_key(&self) -> &str {
        self.public_key
    }
}

impl<'a> AliPay<'a> {
    pub fn new(public_key: &'a str, private_key: &'a str) -> AliPay<'a> {
        AliPay {
            public_key,
            private_key,
            sandbox: true,
            request: HashMap::new(),
            biz_content: HashMap::new(),
        }
    }

    pub fn sandbox(&mut self) -> &mut Self {
        self.sandbox = true;

        self
    }

    pub fn url(&self) -> &str {
        if self.sandbox {
            return "https://openapi-sandbox.dl.alipaydev.com/gateway.do";
        }

        "https://openapi.alipay.com/gateway.do"
    }

    pub fn request<'b: 'a>(&'a mut self, app_id: &'b str) -> &'a mut Self {
        let default = vec![
            ("app_id", app_id),
            ("charset", "utf-8"),
            ("sign_type", "RSA2"),
            ("format", "json"),
            ("version", "1.0"),
        ];

        for (key, val) in default {
            self.request.insert(key, val.to_string());
        }

        self
    }

    pub fn add_request<'b: 'a>(&'a mut self, param: Vec<(&'b str, &'b str)>) -> &'a mut Self {
        for (key, val) in param {
            self.request.insert(key, val.to_string());
        }

        self
    }

    pub fn add_cert<'b: 'a>(
        &'a mut self,
        cert: Option<&'b str>,
        root_cert: Option<&'b str>,
    ) -> &'a mut Self {
        let cert_x509 = CertX509::new();
        if let Some(cert) = cert {
            match cert_x509.cert_sn(cert) {
                Ok(vale) => {
                    self.request.insert("cert_sn", vale);
                }
                Err(e) => println!("cert: {:?}, content: {}", e, cert),
            }
        }

        if let Some(root_cert) = root_cert {
            match cert_x509.root_cert_sn(root_cert) {
                Ok(vale) => {
                    self.request.insert("root_cert_sn", vale);
                }
                Err(e) => println!("root_cert: {:?}, content:{}", e, root_cert),
            }
        }

        self
    }

    pub fn to_json(
        &self,
        method: String,
        biz_content: Option<&Vec<(&str, &str)>>,
    ) -> PayResult<serde_json::Value> {
        let timestamp = chrono::Local::now().format("%F %T").to_string();
        let mut params: Vec<(String, String)> = Vec::with_capacity(self.request.len() + 3);

        for (key, val) in self.request.iter() {
            params.push((key.to_string(), val.to_string()));
        }

        let mut biz_content_param = json!({});
        if let Some(biz_content) = biz_content {
            for (key, val) in biz_content.iter() {
                biz_content_param[key] = json!(val);
            }
        }

        params.push(("biz_content".to_string(), biz_content_param.to_string()));
        params.push(("timestamp".to_string(), timestamp));
        params.push(("method".to_string(), method));
        params.sort_by(|a, b| a.0.cmp(&b.0));
        let mut tmp = String::new();
        for (key, val) in params.iter() {
            tmp.push_str(&format!("{key}={val}&"));
        }
        tmp.pop();

        params.push(("sign".to_string(), self.sign(&tmp)?));

        let mut json_body = json!({});
        for (key, val) in params.iter() {
            json_body[key.as_str()] = json!(val.as_str());
        }

        Ok(json_body)
    }

    pub async fn post<S>(
        &mut self,
        method: S,
        biz_content: Option<&Vec<(&str, &str)>>,
    ) -> PayResult<reqwest::Response>
    where
        S: Into<String>,
    {
        let request = self.to_json(method.into(), biz_content)?;

        println!("{:#?}", request);
        Ok(reqwest::Client::new()
            .post(self.url())
            .header(
                "Content-Type",
                "application/x-www-form-urlencoded;charset=utf-8",
            )
            .query(&request)
            .send()
            .await?)
    }
}
