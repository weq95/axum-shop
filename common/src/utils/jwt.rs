use chrono::{Duration, Utc};
use jsonwebtoken::{
    decode as jwt_decode, encode as jwt_encode, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};

use crate::error::{ApiError, ApiResult};

const SECRET: &str = "weq_B03ln1h9))R+BU!TbnsX@qTf862TgAbhAbj#lwAK6q+GBaEI0Ui&D~GZ~O8cUnRoQw_hoLa&CFB#8h%q+YUt9%WP@~EW$_dyl";

const ISS: &str = "湖北大学[University Test]";

const EXP: i64 = 600;

/// 用户来源
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UserSource {
    //管理端
    Admin,
    //微信小程序
    WxApp,
    //微信公众号
    Wechat,
    //PC端
    PC,
    //手机端
    Mobile,
}

/// 用户类型
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UserType {
    //管理员
    Admin,
    //超级管理员
    SuperAdmin,
    //普通用户
    User,
}

impl Default for UserType {
    fn default() -> Self {
        UserType::User
    }
}

impl Default for UserSource {
    fn default() -> Self {
        UserSource::Mobile
    }
}

pub struct JWT {
    pub secret: String,
    pub exp: i64,
    pub iss: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Claims {
    pub id: i64,
    // 用户类型
    pub user_type: UserType,
    //昵称
    pub username: String,
    //账号
    pub email: String,
    //来源
    pub from: UserSource,
    //租户编码
    pub agency_code: String,
    //过期时间
    pub exp: i64,
    //签发机构
    iss: String,
    //token的类型
    token_type: String,
}

impl Claims {
    pub fn token_type(&self) -> String {
        self.token_type.clone()
    }
}

impl Default for JWT {
    fn default() -> Self {
        Self {
            secret: SECRET.to_string(),
            exp: EXP,
            iss: ISS.to_string(),
        }
    }
}

impl JWT {
    pub fn new(secret: String, exp: i64, iss: String) -> Self {
        Self { secret, exp, iss }
    }

    pub fn new_claims(
        &self,
        id: i64,
        email: String,
        username: String,
        agency_code: String,
        source: UserSource,
        user_type: UserType,
    ) -> Claims {
        Claims {
            id,
            email,
            agency_code,
            username,
            user_type,
            from: source,
            iss: self.iss.clone(),
            exp: self.calc_claim_exp(),
            token_type: "".to_string(),
        }
    }

    fn calc_claim_exp(&self) -> i64 {
        (Utc::now() + Duration::seconds(self.exp)).timestamp()
    }

    fn secret_bytes(&self) -> &[u8] {
        (&self.secret).as_bytes()
    }

    /// 获取签名token
    pub fn token(&self, claims: &Claims) -> ApiResult<String> {
        jwt_encode(
            &Header::default(),
            claims,
            &EncodingKey::from_secret(self.secret_bytes()),
        )
        .map_err(ApiError::from)
    }

    pub fn token_info(&self, claims: &mut Claims) -> ApiResult<(String, String)> {
        Ok((self.access_token(claims)?, self.refresh_token(claims)?))
    }

    /// access_token 有效期: 20分钟
    pub fn access_token(&self, claims: &mut Claims) -> ApiResult<String> {
        claims.token_type = "access_token".to_string();
        claims.exp = self.calc_claim_exp();

        self.token(claims)
    }

    /// refresh_token 有效期: 15天
    pub fn refresh_token(&self, claims: &mut Claims) -> ApiResult<String> {
        claims.token_type = "refresh_token".to_string();
        claims.exp = (Utc::now() + Duration::days(15i64)).timestamp();

        self.token(claims)
    }

    /// 验证token, 并返回claims
    pub fn verify(&self, token: &str) -> ApiResult<Claims> {
        let mut validate = Validation::new(jsonwebtoken::Algorithm::HS256);
        validate.set_issuer(&[self.iss.clone()]);

        Ok(jwt_decode(
            token,
            &DecodingKey::from_secret(self.secret_bytes()),
            &validate,
        )
        .map_err(ApiError::from)?
        .claims)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_gen_token() {
        let jwt = JWT::default();
        let claims = jwt.new_claims(
            1i64,
            "1842618766@qq.com".to_string(),
            "weq".to_string(),
            "AFC".to_string(),
            UserSource::Admin,
            UserType::SuperAdmin,
        );
        let token = jwt.token(&claims).unwrap();
        println!("success. \r\n{:?}", token);
    }

    #[test]
    fn test_gen_claims() {
        let token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpZCI6MSwidXNlcl90eXBlIjoiVXNlciIsInVzZXJuYW1lIjoid2VxIiwiZW1haWwiOiIxODQyNjE4NzY2QHFxLmNvbSIsImZyb20iOiJQQyIsImFnZW5jeV9jb2RlIjoiQUZDIiwiZXhwIjoxNjcyMjkyNDkxLCJpc3MiOiLmuZbljJflpKflraZbVW5pdmVyc2l0eSBUZXN0XSJ9.TE_6dHMaYh2muyBugs50xfCd2zSf9_rKHpaOH6gIQfs";

        let claims = JWT::default().verify(token).unwrap();
        println!("success. \r\n{:?}", claims);
    }
}
