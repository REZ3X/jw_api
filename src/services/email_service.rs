use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

use crate::{config::BrevoConfig, error::{AppError, Result}};

#[derive(Clone)]
pub struct EmailService {
    smtp_host: String,
    smtp_port: u16,
    smtp_user: String,
    smtp_pass: String,
    from_email: String,
    from_name: String,
    app_name: String,
    frontend_url: String,
}

impl EmailService {
    pub fn new(brevo: &BrevoConfig, app_name: &str, frontend_url: &str) -> Self {
        Self {
            smtp_host: brevo.smtp_host.clone(),
            smtp_port: brevo.smtp_port,
            smtp_user: brevo.smtp_user.clone(),
            smtp_pass: brevo.smtp_pass.clone(),
            from_email: brevo.from_email.clone(),
            from_name: brevo.from_name.clone(),
            app_name: app_name.to_string(),
            frontend_url: frontend_url.to_string(),
        }
    }

    fn build_mailer(&self) -> Result<AsyncSmtpTransport<Tokio1Executor>> {
        let creds = Credentials::new(self.smtp_user.clone(), self.smtp_pass.clone());
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.smtp_host)
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("SMTP relay error: {}", e)))?
            .port(self.smtp_port)
            .credentials(creds)
            .build();
        Ok(mailer)
    }

    async fn send(&self, email: Message) -> Result<()> {
        let mailer = self.build_mailer()?;
        mailer.send(email).await.map_err(|e| {
            tracing::error!("Failed to send email: {}", e);
            AppError::InternalError(anyhow::anyhow!("Failed to send email: {}", e))
        })?;
        Ok(())
    }

    pub async fn send_verification_email(
        &self,
        to_email: &str,
        to_name: &str,
        token: &str,
    ) -> Result<()> {
        let verification_url = format!("{}/auth/verify-email?token={}", self.frontend_url, token);

        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0"/>
<title>Verify Your Email</title>
<style>
  body {{ font-family: 'Segoe UI', Tahoma, sans-serif; background:#f0f4ff; margin:0; padding:0; }}
  .wrap {{ max-width:600px; margin:40px auto; }}
  .card {{ background:#fff; border-radius:24px; padding:48px 40px; box-shadow:0 8px 40px rgba(100,120,255,0.10); }}
  h1 {{ color:#2d2d2d; font-size:26px; margin-top:0; }}
  p {{ color:#555; line-height:1.7; font-size:15px; }}
  .btn {{ display:inline-block; background:linear-gradient(135deg,#059669,#10b981); color:#fff!important;
          padding:14px 36px; border-radius:50px; font-weight:700; text-decoration:none;
          font-size:15px; margin:24px 0; box-shadow:0 4px 16px rgba(5,150,105,0.35); }}
  .link {{ color:#059669; word-break:break-all; font-size:13px; }}
  .footer {{ text-align:center; color:#aaa; font-size:12px; margin-top:32px; }}
</style>
</head>
<body>
<div class="wrap">
  <div class="card">
    <h1>Welcome to {app}</h1>
    <p>Hi <strong>{name}</strong>,</p>
    <p>Thank you for joining us. Click the button below to verify your email and start reporting civic issues.</p>
    <div style="text-align:center">
      <a href="{url}" class="btn">Verify Email Address</a>
    </div>
    <p>Or copy this link into your browser:</p>
    <p><a href="{url}" class="link">{url}</a></p>
    <hr style="border:none;border-top:1px solid #eee;margin:28px 0"/>
    <p style="font-size:13px;color:#888">This link expires in 24 hours.<br>
    If you didn't create an account, you can ignore this email.</p>
  </div>
  <div class="footer">&copy; 2026 {app}. All rights reserved.</div>
</div>
</body>
</html>"#,
            app = self.app_name,
            name = to_name,
            url = verification_url
        );

        let email = Message::builder()
            .from(format!("{} <{}>", self.from_name, self.from_email).parse().unwrap())
            .to(format!("{} <{}>", to_name, to_email).parse().unwrap())
            .subject(format!("Verify your {} account", self.app_name))
            .header(ContentType::TEXT_HTML)
            .body(html)
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("Email build error: {}", e)))?;

        self.send(email).await?;
        tracing::info!("Verification email sent to {}", to_email);
        Ok(())
    }

    pub async fn send_department_response_notification(
        &self,
        to_email: &str,
        to_name: &str,
        department: &str,
        post_caption: &str,
    ) -> Result<()> {
        let dashboard_url = format!("{}/posts", self.frontend_url);

        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8"/>
<title>Department Response</title>
<style>
  body {{ font-family: 'Segoe UI', sans-serif; background:#f0fdf4; margin:0; padding:0; }}
  .wrap {{ max-width:600px; margin:40px auto; }}
  .card {{ background:#fff; border-radius:24px; padding:48px 40px; box-shadow:0 8px 40px rgba(5,150,105,.10); }}
  h1 {{ color:#065f46; font-size:24px; margin-top:0; }}
  p {{ color:#555; line-height:1.7; font-size:15px; }}
  .badge {{ display:inline-block; background:#d1fae5; color:#065f46; padding:4px 14px; border-radius:50px; font-size:13px; font-weight:700; }}
  .btn {{ display:inline-block; background:linear-gradient(135deg,#059669,#10b981); color:#fff!important;
          padding:14px 36px; border-radius:50px; font-weight:700; text-decoration:none; font-size:15px; margin:24px 0; }}
  .footer {{ text-align:center; color:#aaa; font-size:12px; margin-top:32px; }}
</style>
</head>
<body>
<div class="wrap">
<div class="card">
  <h1>Your Report Got a Response!</h1>
  <p>Hi <strong>{name}</strong>,</p>
  <p>The <span class="badge">{dept}</span> department has responded to your report:</p>
  <div style="background:#f8fafc;border-radius:12px;padding:16px;margin:16px 0;">
    <p style="margin:0;color:#374151;font-style:italic;">"{caption}"</p>
  </div>
  <div style="text-align:center">
    <a href="{url}" class="btn">View Response</a>
  </div>
</div>
<div class="footer">&copy; 2026 {app}. All rights reserved.</div>
</div>
</body>
</html>"#,
            app = self.app_name,
            name = to_name,
            dept = department.replace('_', " "),
            caption = if post_caption.len() > 100 {
                format!("{}...", &post_caption[..100])
            } else {
                post_caption.to_string()
            },
            url = dashboard_url
        );

        let email = Message::builder()
            .from(format!("{} <{}>", self.from_name, self.from_email).parse().unwrap())
            .to(format!("{} <{}>", to_name, to_email).parse().unwrap())
            .subject(format!("{} — Department responded to your report", self.app_name))
            .header(ContentType::TEXT_HTML)
            .body(html)
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("Email build error: {}", e)))?;

        self.send(email).await?;
        tracing::info!("Department response notification sent to {}", to_email);
        Ok(())
    }
}
