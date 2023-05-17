use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::error::{ApiError, ApiResult};

#[derive(Clone)]
pub struct SnowflakeIdWorker(Arc<Mutex<SnowflakeIdWorkerInner>>);

impl SnowflakeIdWorker {
    pub fn new(worker_id: u128, data_center_id: u128) -> ApiResult<SnowflakeIdWorker> {
        Ok(Self(Arc::new(Mutex::new(SnowflakeIdWorkerInner::new(
            worker_id,
            data_center_id,
        )?))))
    }

    pub fn next_id(&self) -> ApiResult<u128> {
        let mut inner = match self.0.lock() {
            Ok(result) => result,
            Err(_e) => return Err(ApiError::Error(_e.to_string())),
        };

        inner.next_id()
    }
}

struct SnowflakeIdWorkerInner {
    // 工作节点id
    worker_id: u128,
    // 数据id
    data_center_id: u128,
    // 序列号
    sequence: u128,
    // 上一次时间戳
    last_timestamp: u128,
}

impl SnowflakeIdWorkerInner {
    // 开始时间戳（2023-03-16）
    const TWEPOCH: u128 = 1678955490000;
    // 机器id所占的位数
    const WORKER_ID_BITS: u128 = 5;
    // 数据节点所占的位数
    const DATA_CENTER_ID_BITS: u128 = 5;
    // 支持最大的机器ID，最大是31
    const MAX_WORKER_ID: u128 = (-1 ^ (-1 << Self::WORKER_ID_BITS)) as u128;
    // 支持的最大数据节点ID，结果是31
    const MAX_DATA_CENTER_ID: u128 = (-1 ^ (-1 << Self::DATA_CENTER_ID_BITS)) as u128;
    // 序列号所占的位数
    const SEQUENCE_BITS: u128 = 12;
    // 工作节点标识ID向左移12位
    const WORKER_ID_SHIFT: u128 = Self::SEQUENCE_BITS;
    // 数据节点标识ID向左移动17位（12位序列号+5位工作节点）
    const DATA_CENTER_ID_SHIFT: u128 = Self::SEQUENCE_BITS + Self::WORKER_ID_BITS;
    // 时间戳向左移动22位（12位序列号+5位工作节点+5位数据节点）
    const TIMESTAMP_LEFT_SHIFT: u128 =
        Self::SEQUENCE_BITS + Self::WORKER_ID_BITS + Self::DATA_CENTER_ID_BITS;
    // 生成的序列掩码，这里是4095
    const SEQUENCE_MASK: u128 = (-1 ^ (-1 << Self::SEQUENCE_BITS)) as u128;

    fn new(worker_id: u128, data_center_id: u128) -> ApiResult<Self> {
        // 校验worker_id合法性
        if worker_id > Self::MAX_WORKER_ID {
            return Err(ApiError::Error(format!(
                "workerId:{} must be less than {}",
                worker_id,
                Self::MAX_WORKER_ID
            )));
        }

        // 校验data_center_id合法性
        if data_center_id > Self::MAX_DATA_CENTER_ID {
            return Err(ApiError::Error(format!(
                "datacenterId:{} must be less than {}",
                data_center_id,
                Self::MAX_DATA_CENTER_ID
            )));
        }

        Ok(Self {
            worker_id,
            data_center_id,
            sequence: 0,
            last_timestamp: 0,
        })
    }

    fn next_id(&mut self) -> ApiResult<u128> {
        let mut timestamp = Self::get_time()?;
        if timestamp < self.last_timestamp {
            return Err(ApiError::Error(format!(
                "Clock moved backwards.  Refusing to generate id for {} milliseconds",
                self.last_timestamp - timestamp
            )));
        }

        // 如果当前时间戳等于上一次的时间戳，那么计算出序列号目前是第几位
        if timestamp == self.last_timestamp {
            self.sequence = (self.sequence + 1) & Self::SEQUENCE_MASK;
            if self.sequence == 0 {
                timestamp = Self::til_next_mills(self.last_timestamp)?;
            }
        } else {
            // 如果当前时间戳大于上一次的时间戳，序列号置为0。因为又开始了新的毫秒，所以序列号要从0开始。
            self.sequence = 0;
        }

        // 把当前时间戳赋值给last_timestamp，以便下一次计算next_id
        self.last_timestamp = timestamp;

        Ok(((timestamp - Self::TWEPOCH) << Self::TIMESTAMP_LEFT_SHIFT)
            | (self.data_center_id << Self::DATA_CENTER_ID_SHIFT)
            | (self.worker_id << Self::WORKER_ID_SHIFT)
            | self.sequence)
    }

    // 计算一个大于上一次时间戳的时间戳
    fn til_next_mills(last_timestamp: u128) -> ApiResult<u128> {
        Ok(loop {
            let timestamp = Self::get_time()?;

            if timestamp > last_timestamp {
                break timestamp;
            }
        })
    }

    // 获取当前时间戳
    fn get_time() -> ApiResult<u128> {
        match SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => Ok(duration.as_millis()),
            Err(e) => Err(ApiError::Error(e.to_string())),
        }
    }
}

#[cfg(test)]
mod test {
    use tokio::spawn;

    use crate::utils::snowflake::SnowflakeIdWorker;
    

    const WORKER_ID: u128 = 1;
    const DATA_CENTER_ID: u128 = 1;

    #[tokio::test]
    async fn create_uuid() {
        let worker = SnowflakeIdWorker::new(WORKER_ID, DATA_CENTER_ID).unwrap();
        let mut handlers = vec![];
        for _ in 0..100 {
            let worker = worker.clone();
            handlers.push(spawn(async move {
                println!("{}", worker.next_id().unwrap());
            }));
        }

        for i in handlers {
            i.await.unwrap()
        }
    }
}
