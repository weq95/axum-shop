use serde::Deserialize;
use validator::Validate;

/// 用户列表查询条件
#[derive(Deserialize, Clone)]
pub struct ReqQueryUser {
    pub name: Option<String>,
    pub nickname: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub page_num: Option<u32>,
    pub page_size: Option<u32>,
}

/// 获取单个用户查询条件
#[derive(Validate, Deserialize, Clone)]
pub struct ReqGetUser {
    pub id: Option<i64>,
    pub name: Option<String>,
    #[validate(range(min = 1, max = 255, message = "年龄不合法"))]
    pub age: Option<u8>,
    pub nickname: Option<String>,
    #[validate(phone)]
    pub phone: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
}

/// 创建用户请求体
#[derive(Validate, Deserialize, Debug, Clone)]
pub struct ReqCrateUser {
    #[validate(length(min = 4, max = 100, message = "名称至少4个字符"))]
    pub name: Option<String>,
    #[validate(range(min = 1, max = 125, message = "年龄必须在1-125之间"))]
    pub age: Option<i16>,
    #[validate(length(min = 6, message = "密码至少6个字符"))]
    pub password: Option<String>,
    #[validate(required)]
    pub password_confirm: Option<String>,
    pub nickname: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    #[validate(phone)]
    pub phone: Option<String>,
}

/// 用户更新信息
#[derive(Deserialize, Validate, Clone)]
pub struct ReqUpdateUser {
    #[validate(required)]
    pub id: Option<i64>,
    #[validate(range(min = 1, max = 125, message = "年龄必须在1-125之间"))]
    pub age: Option<i16>,
    #[validate(length(min = 4, max = 100, message = "名称至少4个字符"))]
    pub name: Option<String>,
    #[validate(length(min = 4, max = 100, message = "昵称至少4个字符"))]
    pub nickname: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ReqLogin {
    #[validate(email)]
    pub email: Option<String>,
    #[validate(required)]
    pub password: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ReqRegister {
    #[validate(email)]
    pub email: Option<String>,
    #[validate(length(min = 8, max = 255, message = "密码长度超过最大限制"))]
    pub password: Option<String>,
    #[validate(required)]
    pub confirm_password: Option<String>,
    #[validate(phone)]
    pub phone: Option<String>,
    #[validate(length(min = 6, max = 6, message = "验证码错误"))]
    pub code: Option<String>,
}