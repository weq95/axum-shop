pub struct Md5Encoder;

pub trait PassWordAnalyze {
    /// 字符加密
    fn encode(raw_pwd: &str) -> String;

    /// 检验密码
    /// new_pwd 新密码
    /// old_pwd 旧密码
    fn verify(new_pwd: &str, old_pwd: &str) -> bool;
}

impl PassWordAnalyze for Md5Encoder {
    fn encode(raw_pwd: &str) -> String {
        format!("{:x}", md5::compute(raw_pwd))
    }

    fn verify(new_pwd: &str, old_pwd: &str) -> bool {
        new_pwd.eq(old_pwd) ||
            new_pwd.eq(&Md5Encoder::encode(old_pwd))
    }
}