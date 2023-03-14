use std::sync::Arc;

use async_once::AsyncOnce;
use axum::async_trait;
use lazy_static::lazy_static;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_yaml::value::Value;
use tokio::sync::RwLock;
use tokio::time::Duration;

use crate::error::{ApiError, ApiResult};

lazy_static! {
    pub static ref APP_CONFIG: AsyncOnce<RwLock<Arc<Application>>> = AsyncOnce::new(async {
        match Application::init().await {
            Ok(application) => {
                let application = RwLock::new(Arc::new(application));
                let application_clone = application.read().await.clone();
                tokio::spawn(async move {
                    application_clone.update().await;
                });

                application
            }
            Err(e) => {
                println!("application load err: {}", e);
                std::process::exit(-1);
            }
        }
    });
}

/// 自动更新配置文件接口
#[async_trait]
pub trait UpdateAppCfg {
    async fn update(&self);
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Application {
    // 服务域名
    pub host: String,
    // 服务端口
    pub port: u16,
    // 配置文件更新频率
    pub update_frequency: u16,
    pub postgres: PostgresConfig,
    pub redis: RedisConfig,
}

#[async_trait]
impl UpdateAppCfg for Arc<Application> {
    async fn update(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(self.update_frequency as u64));

        loop {
            interval.tick().await;

            tokio::spawn(async {
                match Application::init().await {
                    Ok(result) => {
                        let mut lock = APP_CONFIG.get().await.write().await;
                        *lock = Arc::new(result)
                    }
                    Err(_e) => println!("interval.tick err: {}", _e),
                }
            });
        }
    }
}

/// 获取系统配置, 需要解锁才能读取
pub async fn application_config() -> Arc<Application> {
    APP_CONFIG.get().await.read().await.clone()
}

impl Application {
    pub async fn init() -> ApiResult<Self> {
        let cfg = Application::file_get_content().await?;

        Ok(Application {
            host: Self::analysis::<String>("host", &cfg)?,
            port: Self::analysis::<u16>("port", &cfg)?,
            update_frequency: Self::analysis::<u16>("profile_refresh_rate", &cfg)?,
            postgres: Self::analysis::<PostgresConfig>("postgres", &cfg)?,
            redis: Self::analysis::<RedisConfig>("redis", &cfg)?,
        })
    }

    fn analysis<T: DeserializeOwned>(key: &str, value: &Value) -> ApiResult<T> {
        let val = match value.get(key) {
            Some(value) => value,
            None => return Err(ApiError::Error(format!("{} 字段不存在", key))),
        };
        Ok(serde_yaml::from_value::<T>(val.clone())?)
    }

    /// 读取文件内容, 文件不存在时进行创建
    async fn file_get_content() -> ApiResult<Value> {
        let filename = "application.yaml";
        match tokio::fs::read(filename).await {
            Ok(file_content) => {
                let content = match serde_yaml::from_slice::<Value>(&file_content.as_slice()) {
                    Ok(value) => Ok(value),
                    Err(_e) => {
                        println!("\r\n请检查 ./{} 配置信息!\r\nerr: {}\r\n", filename, _e);

                        Err(ApiError::Error(_e.to_string()))
                    }
                };

                content
            }
            Err(_e) => {
                println!(" ./{} 不存在, 正在创建配置文件 ...", filename);
                let copy_filename = ".example.yaml";
                match tokio::fs::copy(copy_filename, filename).await {
                    Ok(_) => {
                        println!("配置文件 ./{} 创建成功, 请填写配置信息!", filename);
                        return Err(ApiError::Error("请重新填写配置信息".to_string()));
                    }
                    Err(_e) => {
                        println!("应用程序启动失败, 没有找到原始配置文件,err: {}", _e);
                        return Err(ApiError::Error(_e.to_string()));
                    }
                }
            }
        }
    }
}

/// postgres 数据库配置参数
#[derive(Serialize, Deserialize, Debug)]
pub struct PostgresConfig {
    pub host: String,
    pub username: String,
    pub password: String,
    pub port: u16,
    pub db_name: String,
    pub pool_size: u32,
}

/// redis 数据库配置参数
#[derive(Serialize, Deserialize, Debug)]
pub struct RedisConfig {
    pub host: String,
    pub username: String,
    pub password: String,
    pub port: u16,
    pub db: u8,
    pub pool_size: u32,
}

#[cfg(test)]
mod test {
    use crate::application::Application;
    use crate::*;

    #[tokio::test]
    async fn init() {
        Application::init();
    }
}