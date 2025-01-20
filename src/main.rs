use actix_web::{web, App, HttpServer};
use sqlx::{self, PgPool};

mod methods;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let pool =
        PgPool::connect(&std::env::var("POSTGRESS_CON").expect("Failed to get POSTGRESS_CON env"))
            .await
            .map(|x| web::Data::new(x))
            .expect("Failed connect to Postgres");

    sqlx::migrate!()
        .run(pool.get_ref())
        .await
        .expect("Failed to run migration");

    HttpServer::new(move || {
        App::new()
            .app_data(pool.clone())
            .service(methods::register)
            .service(methods::login)
    })
    .bind(("127.0.0.1", 5131))?
    .run()
    .await
}
