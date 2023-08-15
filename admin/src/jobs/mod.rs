extern crate cron_job;

use chrono::FixedOffset;
use cron_job::CronJob;

pub use crate::jobs::calculate_fine::calculate_installment_fine;
use crate::jobs::calculate_fine::OverdueRate;

pub mod calculate_fine;

pub fn start_jobs() {
    let mut cron = CronJob::new(FixedOffset::west_opt(-8), 50);
    cron.new_job("0 * * * * *", OverdueRate);


    cron.start();
}