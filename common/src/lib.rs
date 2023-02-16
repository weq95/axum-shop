use serde::Serialize;

pub use request::*;
pub use response::*;
pub use utils::*;

pub mod request;
pub mod response;
pub mod error;
pub mod utils;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}


#[derive(Serialize)]
pub struct Paginate<T: Serialize> {
    pub page: u32,
    /// 每页条数
    pub page_size: u32,
    /// 总页数
    pub page_total: u64,
    /// 总记录数
    pub record_total: u64,
    /// 返回数据: 一定要可以序列化
    pub data: Vec<T>,
}

/// 检测是否为null
pub trait IsEmpty {
    fn is_empty(&self) -> bool;
}

impl IsEmpty for Option<String> {
    /// 检测字符串是否为空
    fn is_empty(&self) -> bool {
        return match self {
            Some(s) => s.is_empty(),
            _ => true,
        };
    }
}

/// qps 监控服务
pub trait QPS {
    fn qps(&self, total: u64);
    fn time(&self, total: u64);
    fn cost(&self);
}

impl QPS for std::time::Instant {
    fn qps(&self, total: u64) {
        let time = self.elapsed();
        let val = total as u128 * 1000000000;

        println!("use OPS: {} Ops/s", (val / time.as_nanos() as u128));
    }

    fn time(&self, total: u64) {
        let time = self.elapsed();
        let val = time.as_nanos() / (total as u128);

        println!("use Time: {:?}, each:{} ns/op", &time, val);
    }

    fn cost(&self) {
        println!("cont: {:?}", self.elapsed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
