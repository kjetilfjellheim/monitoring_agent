use actix_web::{get, web, HttpResponse, Responder};

use crate::api::StateApi;
use crate::api::response::MeminfoResponse;

/**
 * Get current memory information.
 * 
 * `state`: The state object.
 * 
 * Returns the current memory information or an error.
 */
#[get("/meminfo/current")]
pub async fn get_current_meminfo(state: web::Data<StateApi>) -> impl Responder {
    let procsmeminfo = state.monitoring_service.get_current_meminfo();
    match procsmeminfo {
        Ok(procsmeminfo) => HttpResponse::Ok().json(MeminfoResponse::from_meminfo(&procsmeminfo)),
        Err(err) => HttpResponse::InternalServerError().body(format!("Error occured: {err:?}")),
    }
}
