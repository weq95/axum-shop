use std::sync::Arc;

use async_once::AsyncOnce;
use elasticsearch::auth::Credentials;
use elasticsearch::cat::CatIndicesParts;
use elasticsearch::http::transport::{SingleNodeConnectionPool, Transport, TransportBuilder};
use elasticsearch::Elasticsearch;
use lazy_static::lazy_static;
use url::Url;

use crate::application_config;

lazy_static! {
    pub static ref ELASTICSEARCH_CLIENT: AsyncOnce<Arc<Elasticsearch>> = AsyncOnce::new(async {
        let cfg = &application_config().await.elasticsearch;
        if let Some(scheme) = &cfg.scheme {
            let conn_pool = SingleNodeConnectionPool::new(Url::parse(scheme).unwrap());
            let transport = TransportBuilder::new(conn_pool)
                .disable_proxy()
                .build()
                .unwrap();
            return Arc::new(Elasticsearch::new(transport));
        }

        let credentials =
            Credentials::Basic(cfg.username.clone().into(), cfg.password.clone().into());
        let transport = Transport::cloud(&cfg.cloud_id, credentials).unwrap();
        Arc::new(Elasticsearch::new(transport))
    });
}

pub async fn client() {
    let c = ELASTICSEARCH_CLIENT.get().await.clone();

    let result = c.cat().indices(CatIndicesParts::Index(&["*"])).send().await;

    println!("elasticsearch info: {result:#?}");
}
