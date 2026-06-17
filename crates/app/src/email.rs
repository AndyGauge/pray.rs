#[cfg(feature = "ssr")]
pub mod mailer {
    use serde_json::json;

    pub async fn send_group_invite(
        to_email: &str,
        invited_by_name: &str,
        group_name: &str,
        invite_url: &str,
    ) -> Result<(), String> {
        let api_key = std::env::var("RESEND_API_KEY")
            .map_err(|_| "RESEND_API_KEY not set".to_string())?;
        let from = std::env::var("RESEND_FROM")
            .unwrap_or_else(|_| "Thanksgivings <noreply@pray.rs>".to_string());

        let html = format!(
            r#"<div style="font-family:Georgia,serif;max-width:520px;margin:0 auto;color:#2c1810">
                <h2 style="color:#8b4513">{invited_by_name} invited you to join<br><em>{group_name}</em></h2>
                <p>Thanksgivings is a quiet prayer book — a place to write prayers and praises,
                   away from the noise of social media.</p>
                <p style="margin:2rem 0">
                  <a href="{invite_url}"
                     style="background:#8b4513;color:#fdf8f0;padding:0.75rem 1.5rem;
                            text-decoration:none;border-radius:2px;font-size:1rem">
                    Accept invitation
                  </a>
                </p>
                <p style="font-size:0.85rem;color:#9e8a7a">
                  Or copy this link:<br>
                  <a href="{invite_url}" style="color:#8b4513">{invite_url}</a>
                </p>
                <hr style="border:none;border-top:1px solid #d4c5b0;margin:2rem 0">
                <p style="font-size:0.75rem;color:#9e8a7a">
                  You received this because {invited_by_name} entered your email address.
                  If you don't want to join, simply ignore this message.
                </p>
              </div>"#
        );

        let body = json!({
            "from":    from,
            "to":      [to_email],
            "subject": format!("{invited_by_name} invited you to a prayer group"),
            "html":    html,
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| e.to_string())?;

        let resp = client
            .post("https://api.resend.com/emails")
            .bearer_auth(&api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            // Extract human-readable message from Resend's JSON if present
            let msg = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v["message"].as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| format!("delivery failed ({})", status.as_u16()));
            Err(msg)
        }
    }
}
