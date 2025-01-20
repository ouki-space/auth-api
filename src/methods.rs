use actix_web::{post, web, HttpResponse, Responder};
use sqlx::{self, PgPool};

use bcrypt::{hash, verify, DEFAULT_COST};

fn generate_token(username: &str) -> String {
    use rand::{distributions::Alphanumeric, Rng};
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(username);

    let hash_result = hasher.finalize();
    let hash_string = format!("{:x}", hash_result);

    let random_parts: Vec<String> = (0..3)
        .map(|_| {
            rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(4)
                .map(char::from)
                .collect()
        })
        .collect();

    format!(
        "{}-{}-{}-{}",
        hash_string, random_parts[0], random_parts[1], random_parts[2]
    )
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct User {
    name: String,
    password: String,
}

#[post("/register")]
async fn register(data: web::Json<User>, pool: web::Data<PgPool>) -> impl Responder {
    let user = data.0;

    let block_symbols = vec![
        ' ', '*', '|', '\\', '/', ':', '"', '\'', '<', '>', '?', '{', '}',
    ];
    for s in block_symbols {
        if user.name.contains(s) || user.password.contains(s) {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "status": "Err",
                "message": format!("Symbol '{}' is not allowed", s)
            }));
        }
    }

    let selector = sqlx::query("SELECT * FROM users WHERE name = ($1)")
        .bind(&user.name)
        .fetch_one(pool.get_ref())
        .await;

    if selector.is_ok() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "status": "Err",
            "message": format!("User '{}' exists", user.name)
        }));
    }

    let hashed_password = hash(&user.password, DEFAULT_COST);

    if hashed_password.is_err() {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "Err",
            "message": format!("Failed hashed the password")
        }));
    }
    let hashed_password = hashed_password.unwrap();

    let result = sqlx::query("INSERT INTO users (name, password, token) VALUES ($1, $2, $3)")
        .bind(&user.name)
        .bind(&hashed_password)
        .bind(&generate_token(&user.name))
        .execute(pool.get_ref())
        .await;

    match result {
        Ok(_) => HttpResponse::Created().json(serde_json::json!({
            "status": "Ok",
            "message": format!("User '{}' created", &user.name)
        })),
        Err(err) => HttpResponse::BadRequest().json(serde_json::json!({
            "status": "Err",
            "message": err.to_string()
        })),
    }
}

#[post("/login")]
async fn login(data: web::Json<User>, pool: web::Data<PgPool>) -> impl Responder {
    let user = data.0;

    let selector = sqlx::query_as("SELECT password, token FROM users WHERE name = $1")
        .bind(&user.name)
        .fetch_one(pool.get_ref())
        .await;

    if let Err(ref e) = selector {
        println!("{}", e.to_string());
    }

    if selector.is_err() {
        return HttpResponse::NotFound().json(serde_json::json!({
            "status": "Err",
            "message": format!("User '{}' not found", &user.name)
        }));
    }
    let (password, token): (String, String) = selector.unwrap();

    let verify_result = verify(&user.password, &password);

    if verify_result.is_err() {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "Err",
            "message": format!("Failed verify the password")
        }));
    }
    let verify_result = verify_result.unwrap();

    if verify_result {
        HttpResponse::Ok().json(serde_json::json!({
            "status": "Ok",
            "message": token
        }))
    } else {
        HttpResponse::BadRequest().json(serde_json::json!({
            "status": "Err",
            "message": "Wrong password"
        }))
    }
}
