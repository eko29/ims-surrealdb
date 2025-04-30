use lettre::{Message, SmtpTransport, Transport};
use std::env;
use dotenv::dotenv;
use anyhow::{Result, anyhow};

pub async fn send_verification_email(email: &str, title: &str, body: &str) -> Result<()> {
    dotenv().ok();
    let smtp_server = env::var("SMTP_SERVER").unwrap();
    let smtp_username = env::var("SMTP_USERNAME").unwrap();
    let smtp_password = env::var("SMTP_PASSWORD").unwrap();

    println!("smtp_server : {}", smtp_server);
    println!("smtp_username : {}", smtp_username);
    println!("smtp_password : {}", smtp_password);
    let subject = title;

    let html_body = format!(
        r#"
            <div marginwidth='0' marginheight='0' bgcolor='#fff' style='margin:0;padding:0'>
            <table cellpadding='0' cellspacing='0' border='0' height='100%' width='100%' bgcolor='#fff'>
            <tbody><tr><td valign='top'>
                <table border='0' width='600' cellpadding='0' cellspacing='0' align='center'>
                <tbody><tr><td>
                    <table border='0' width='100%' cellpadding='0' cellspacing='0' bgcolor='#ffffff'>
                    <tbody>
                        <tr>
                        <td valign='middle' align='center'>
                            <img src='https://smartta.smartelco.co.id/templates/sst/img/logo-1.png' alt='banner' width="1018" height="228" align='center' border='0' style='margin:auto'>
                        </td>
                        </tr>
                        <tr>
                        <td style='padding:4%;font-size:15px;line-height:1.3;color:#383a40;border-width:1px;border-color:#f1f1f1;border-style:ridge'>
                            {}
                        </td>
                        </tr>
                    </tbody>
                    </table>
                </td></tr>
                </tbody></table>
            </td></tr></tbody>
            </table>
            </div>
            "#,
            body,
    );

    let email = Message::builder()
        .from("Admin <no-reply@example.com>".parse().unwrap())
        .to(email.parse().unwrap())
        .subject(subject)
        .header(lettre::message::header::ContentType::TEXT_HTML)
        .body(html_body)
        .unwrap();

    let creds = lettre::transport::smtp::authentication::Credentials::new(smtp_username, smtp_password);
    // let mailer = SmtpTransport::relay(&smtp_server)
    //     .unwrap()
    //     .credentials(creds)
    //     .build();

    let mailer = SmtpTransport::relay(&smtp_server)
        .map_err(|e| anyhow!("Gagal menghubungi server SMTP: {}", e))?
        .credentials(creds)
        .build();

    // match mailer.send(&email) {
    //     Ok(_) => println!("Email verifikasi terkirim ke {:?}", email),
    //     Err(e) => eprintln!("Gagal mengirim email: {:?}", e),
    // }

    mailer.send(&email).map_err(|e| anyhow!("Gagal mengirim email: {}", e))?;

    println!("Email verifikasi terkirim ke {:?}", email);
    Ok(())
}