use serde::{Deserialize, Serialize};

const FEEDBACK_WEBHOOK_URL: &str =
    "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=3689d449-fc69-484a-905c-47b5aa6f5bf5";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFeedbackInput {
    pub issue: String,
    pub description: String,
    pub contact: String,
}

#[derive(Debug, Serialize)]
struct WecomMarkdownMessage<'a> {
    msgtype: &'a str,
    markdown: WecomMarkdownBody<'a>,
}

#[derive(Debug, Serialize)]
struct WecomMarkdownBody<'a> {
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct WecomResponse {
    errcode: i64,
    errmsg: String,
}

fn validate_feedback_input(input: &UserFeedbackInput) -> Result<(), String> {
    if input.issue.trim().is_empty() {
        return Err("问题不能为空".to_string());
    }
    if input.description.trim().is_empty() {
        return Err("描述不能为空".to_string());
    }
    if input.issue.chars().count() > 80 {
        return Err("问题长度不能超过 80 字".to_string());
    }
    if input.description.chars().count() > 1000 {
        return Err("描述长度不能超过 1000 字".to_string());
    }
    if input.contact.chars().count() > 120 {
        return Err("联系方式长度不能超过 120 字".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn submit_user_feedback(
    app: tauri::AppHandle,
    payload: UserFeedbackInput,
) -> Result<(), String> {
    validate_feedback_input(&payload)?;

    let app_name = app.package_info().name.clone();
    let app_version = app.package_info().version.to_string();
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let contact = input_contact_or_default(&payload.contact);
    let content = format!(
        "## 用户反馈\n\
         **应用**: {}\n\
         **版本**: {}\n\
         **系统**: {} ({})\n\
         **问题**: {}\n\
         **描述**:\n{}\n\
         **联系方式**: {}",
        app_name,
        app_version,
        os,
        arch,
        payload.issue.trim(),
        payload.description.trim(),
        contact,
    );
    let message = WecomMarkdownMessage {
        msgtype: "markdown",
        markdown: WecomMarkdownBody { content: &content },
    };

    let response = reqwest::Client::new()
        .post(FEEDBACK_WEBHOOK_URL)
        .json(&message)
        .send()
        .await
        .map_err(|err| format!("发送反馈请求失败: {err}"))?;

    let status = response.status();
    let body: WecomResponse = response
        .json()
        .await
        .map_err(|err| format!("解析反馈响应失败: {err}"))?;

    if !status.is_success() {
        return Err(format!(
            "反馈发送失败，HTTP 状态码: {}, errmsg: {}",
            status, body.errmsg
        ));
    }
    if body.errcode != 0 {
        return Err(format!(
            "反馈发送失败，errcode: {}, errmsg: {}",
            body.errcode, body.errmsg
        ));
    }

    Ok(())
}

fn input_contact_or_default(contact: &str) -> &str {
    let trimmed = contact.trim();
    if trimmed.is_empty() {
        "未填写"
    } else {
        trimmed
    }
}
