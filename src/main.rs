use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use serde::{Deserialize, Serialize};
use reqwest::Client;

#[derive(Deserialize)]
struct OamRequest {
    url: String,
}

#[derive(Serialize)]
struct AiRequest {
    requirements: String,
    programming_language: String,
}

#[derive(Deserialize)]
struct AiResponse {
    code: String,
    explanation: String,
    model: String,
    programming_language: String,
}

async fn generate_backstage(oam_req: web::Json<OamRequest>) -> impl Responder {
    let client = Client::new();
    let oam_yaml = match client.get(&oam_req.url).send().await {
        Ok(res) => match res.text().await {
            Ok(text) => text,
            Err(_) => return HttpResponse::InternalServerError().body("Failed to read OAM file content"),
        },
        Err(_) => return HttpResponse::InternalServerError().body("Failed to fetch OAM file URL"),
    };

    let ai_req = AiRequest {
        requirements: format!("Generate a Backstage component manifest from this OAM YAML:\n\n{}", oam_yaml),
        programming_language: "YAML".to_string(),
    };

    let ai_res = match client.post("http://localhost:3000/ai/generate-code").json(&ai_req).send().await {
        Ok(res) => match res.json::<AiResponse>().await {
            Ok(json) => json,
            Err(_) => return HttpResponse::InternalServerError().body("Failed to deserialize AI response"),
        },
        Err(_) => return HttpResponse::InternalServerError().body("Failed to call AI service"),
    };

    HttpResponse::Ok().body(ai_res.code)
}

async fn generate_iac(oam_req: web::Json<OamRequest>) -> impl Responder {
    let client = Client::new();
    let oam_yaml = match client.get(&oam_req.url).send().await {
        Ok(res) => match res.text().await {
            Ok(text) => text,
            Err(_) => return HttpResponse::InternalServerError().body("Failed to read OAM file content"),
        },
        Err(_) => return HttpResponse::InternalServerError().body("Failed to fetch OAM file URL"),
    };

    let ai_req = AiRequest {
        requirements: format!("Generate gcloud commands from this OAM YAML:\n\n{}", oam_yaml),
        programming_language: "Bash".to_string(),
    };

    let ai_res = match client.post("http://localhost:3000/ai/generate-code").json(&ai_req).send().await {
        Ok(res) => match res.json::<AiResponse>().await {
            Ok(json) => json,
            Err(_) => return HttpResponse::InternalServerError().body("Failed to deserialize AI response"),
        },
        Err(_) => return HttpResponse::InternalServerError().body("Failed to call AI service"),
    };

    HttpResponse::Ok().body(ai_res.code)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/generate-backstage", web::post().to(generate_backstage))
            .route("/generate-iac", web::post().to(generate_iac))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
