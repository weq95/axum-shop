use std::collections::HashMap;

use crate::{PayResult, Sign};

pub struct AliPay<'a> {
    public_key: &'a str,
    private_key: &'a str,
    request: HashMap<&'a str, &'a str>,
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
        }
    }

    pub fn sandbox(&mut self) -> &mut Self {
        self.sandbox = true;

        self
    }

    pub fn url(&self) -> &str {
        if self.sandbox {
            return "https://openapi.alipaydev.com/gateway.do";
        }

        "https://openapi.alipay.com/gateway.do"
    }
    pub fn request<'b: 'a>(&'a mut self, app_id: &'b str) -> &'a mut Self {
        let param = vec![
            ("app_id", app_id),
            ("charset", "utf-8"),
            ("sign_type", "RSA2"),
            ("format", "jFson"),
            ("version", "1.0"),
        ];

        for (key, val) in param {
            self.request.insert(key, val);
        }

        self
    }

    pub fn add_request<'b: 'a>(&'a mut self, param: Vec<(&'b str, &'b str)>) -> &'a mut Self {
        for (key, val) in param {
            self.request.insert(key, val);
        }

        self
    }

    pub fn add_cert<'b: 'a>(
        &'a mut self,
        cert: Option<&'b str>,
        root_cert: Option<&'b str>,
    ) -> &'a mut Self {
        if let Some(cert) = cert {
            self.request.insert("cert_sn", cert);
        }

        if let Some(root_cert) = root_cert {
            self.request.insert("root_cert_sn", root_cert);
        }

        self
    }

    pub fn get_param(&self, method: String) -> PayResult<Vec<(String, String)>> {
        let timestamp = chrono::Local::now().format("%F %T").to_string();
        let mut params: Vec<(String, String)> = Vec::with_capacity(self.request.len() + 3);

        for (key, val) in self.request.iter() {
            params.push((key.to_string(), val.to_string()));
        }
        params.push(("timestamp".to_string(), timestamp));
        params.push(("method".to_string(), method));
        params.sort_by(|a, b| a.0.cmp(&b.0));
        let mut tmp = String::new();
        for (key, val) in params.iter() {
            tmp.push_str(&format!("{key}={val}&"));
        }
        tmp.pop();

        params.push(("sign".to_string(), self.sign(&tmp)?));

        Ok(params)
    }

    fn param_to_string(&self, method: String) -> PayResult<String> {
        let mut content = String::new();

        for (key, val) in self.get_param(method)?.iter() {
            content.push_str(&format!("{key}={val}&"));
        }
        content.pop();

        Ok(content)
    }
    async fn post<S>(
        &mut self,
        method: S,
        biz_content: Option<String>,
    ) -> PayResult<reqwest::Response>
    where
        S: Into<String>,
    {
        let body = reqwest::Body::from(self.param_to_string(method.into())?);
        Ok(reqwest::ClientBuilder::new()
            .build()?
            .post(self.url())
            .send()
            .await?)
    }
}
