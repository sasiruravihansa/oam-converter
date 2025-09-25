use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const SYSTEM_DIRECTIVE: &str = "You are an expert platform engineer. Given an OAM Application spec, generate a minimal yet production-ready IaC project for the requested tool and provider. You are allowed to choose the file/folder structure dynamically. Output a JSON mapping of file paths to file contents. Only include files that are necessary to deploy the described resources. Prefer least-privilege, tags/labels, and outputs. CRITICAL: Do NOT create separate variables.tf or outputs.tf files if variables/outputs are already defined in main.tf. Avoid duplicate declarations - use either main.tf only OR separate files, not both.";

// The request body for the local AI service
#[derive(Serialize)]
struct LocalAiRequest {
    requirements: String,
    programming_language: String,
}

// The response body from the local AI service
#[derive(Deserialize)]
struct LocalAiResponse {
    code: String,
}

// This is the structure we expect to be inside the `code` field of the response
#[derive(Deserialize, Serialize)]
pub struct GeneratedFiles {
    pub files: HashMap<String, String>,
}

/// Builds the prompt for the LLM based on the Python reference.
pub fn build_prompt(oam_yaml: &str, provider: &str, tool: &str) -> String {
    let requirements = if tool == "gcloud" {
        vec![
            "Generate a complete and runnable shell script named 'deploy.sh' that uses gcloud commands to deploy the application described in the OAM specification.",
            "The script must not be a template. It should be a fully functional script that can be executed directly.",
            "Use the component name from the OAM spec as the service name.",
            "Use the image from the OAM spec as the container image.",
            "Expose environment variables for key parameters like region and project.",
        ]
    } else {
        vec![
            "Project must be runnable by 'init/plan/apply/destroy' for the given tool.",
            "Include backend/state config if required; default to local if unspecified.",
            "Expose variables/inputs for key parameters (region, project, name, image, scaling, env).",
        ]
    };

    let requirements_json = serde_json::json!({
        "instructions": {
            "tool": tool,
            "provider": provider,
            "requirements": requirements,
        },
        "oam": oam_yaml,
        "output_format": {
            "type": "json_object",
            "schema": {
                "files": {
                    "type": "object",
                    "description": "A map of file paths to their string content.",
                    "additionalProperties": {
                        "type": "string"
                    }
                }
            }
        },
    });

    serde_json::to_string(&requirements_json).unwrap_or_default()
}

/// Calls the local AI service to generate files.
pub async fn generate_files(
    client: &reqwest::Client,
    user_prompt: &str,
) -> Result<GeneratedFiles, String> {
    // Combine the system directive with the user prompt
    let combined_requirements = format!("{}\n\nUser Prompt:\n{}", SYSTEM_DIRECTIVE, user_prompt);

    let ai_request = LocalAiRequest {
        requirements: combined_requirements,
        programming_language: "JSON".to_string(),
    };

    let res = client
        .post("http://localhost:3000/ai/generate-code")
        .json(&ai_request)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let error_body = res
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read error body>".to_string());
        return Err(format!("Local AI service request failed: {}", error_body));
    }

    let ai_response = res
        .json::<LocalAiResponse>()
        .await
        .map_err(|e| e.to_string())?;

    // The AI service often wraps the JSON in a markdown block. We need to remove it.
    let cleaned_code = ai_response.code
        .trim()
        .strip_prefix("```json")
        .unwrap_or(&ai_response.code)
        .strip_suffix("```")
        .unwrap_or(&ai_response.code)
        .trim();

    serde_json::from_str::<GeneratedFiles>(cleaned_code).map_err(|e| {
        format!(
            "Failed to parse file map from AI response `code` field: {}. Content was: {}",
            e, ai_response.code
        )
    })
}

