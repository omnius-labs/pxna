use async_trait::async_trait;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use omnius_pxna_interface::{apis, models};

use crate::interface::ApiServer;

#[allow(unused_variables)]
#[async_trait]
impl omnius_pxna_interface::apis::health::Health for ApiServer {
    async fn get_health(&self, method: &Method, host: &Host, cookies: &CookieJar) -> std::result::Result<omnius_pxna_interface::apis::health::GetHealthResponse, ()> {
        Ok(apis::health::GetHealthResponse::Status200_TheDaemonIsRunning(models::HealthResponse::new(true)))
    }
}
