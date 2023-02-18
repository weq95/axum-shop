use std::path::PathBuf;

pub mod pwd;
pub mod jwt;
pub mod redis;
pub mod casbin;
pub mod pgsql;


/// 读取系统配置文件
pub fn init_read_config() {
    dotenv::from_path(PathBuf::from("./config/.env")).unwrap();
    dotenv::dotenv().ok();
}