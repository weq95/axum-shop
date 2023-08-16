extern crate cron_job;

use std::ops::Add;

use chrono::Local;
use cron_job::Job;
use sqlx::postgres::types::PgMoney;
use sqlx::Row;

use common::error::ApiResult;

use crate::models::installments::Installments;

struct CalculateFineItems {
    id: i64,
    installment_id: i64,
    base: PgMoney,
    fee: PgMoney,
    fine: PgMoney,
    due_time: chrono::DateTime<Local>,
}

pub async fn calculate_installment_fine() -> ApiResult<()> {
    let mut id = 0i64;
    loop {
        let mut ids: Vec<i64> = Vec::with_capacity(100);
        let items = sqlx::query(
            "select id,installment_id,base,fee,fine,due_date from installment_items where id > $1\
           and due_date <= now() and paid_at is null order by id ASC limit 100",
        )
        .bind(id)
        .fetch_all(&*common::postgres().await)
        .await?
        .iter()
        .map(|row| {
            let item_id = row.get::<i64, _>("id");
            let installment_id = row.get::<i64, _>("installment_id");
            ids.push(installment_id);
            id = item_id;
            CalculateFineItems {
                installment_id,
                id: item_id,
                base: row.get::<PgMoney, _>("base"),
                fee: row.get::<PgMoney, _>("fee"),
                fine: row.get::<PgMoney, _>("fine"),
                due_time: row.get::<chrono::DateTime<Local>, _>("due_date"),
            }
        })
        .collect::<Vec<CalculateFineItems>>();

        if items.is_empty() {
            break;
        }
        let installments = Installments::overdue_items(ids).await?;

        for item in items {
            let days = Local::now().signed_duration_since(item.due_time).num_days();
            let base = item.base.add(item.fee);
            let fine =
                PgMoney::from(base.0 * days * installments.get(&item.installment_id).unwrap().0);
            let fine = if fine.0 > base.0 { base } else { fine };

            sqlx::query("update installment_items set fine = $1 where id = $2")
                .bind(fine)
                .bind(item.id)
                .execute(&*common::postgres().await)
                .await?;
        }
    }

    Ok(())
}

pub struct OverdueRate;

impl Job for OverdueRate {
    fn run(&mut self) {
        println!("定时任务开始执行...");
    }
}
