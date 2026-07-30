use serde::Serialize;

use crate::{
    ado::{
        pools::{Agent, AgentPool, JobRequest, JobRequestState},
        projects::Project,
        users::UserEntitlement,
    },
    cli::OutputArg,
    error::{AdoctlError, Result},
};

pub fn render_users(users: &[UserEntitlement], output: OutputArg) -> Result<String> {
    match output {
        OutputArg::Json => Ok(serde_json::to_string_pretty(users)?),
        OutputArg::Table => Ok(render_user_table(users)),
    }
}

pub fn render_projects(projects: &[Project], output: OutputArg) -> Result<String> {
    match output {
        OutputArg::Json => Ok(serde_json::to_string_pretty(projects)?),
        OutputArg::Table => Ok(render_project_table(projects)),
    }
}

pub fn render_agent_pools(pools: &[AgentPool], output: OutputArg) -> Result<String> {
    match output {
        OutputArg::Json => Ok(serde_json::to_string_pretty(pools)?),
        OutputArg::Table => Ok(render_agent_pool_table(pools)),
    }
}

pub fn render_agents(agents: &[Agent], output: OutputArg) -> Result<String> {
    match output {
        OutputArg::Json => Ok(serde_json::to_string_pretty(agents)?),
        OutputArg::Table => Ok(render_agent_table(agents)),
    }
}

pub fn render_jobs(jobs: &[JobRequest], output: OutputArg) -> Result<String> {
    match output {
        OutputArg::Json => Ok(serde_json::to_string_pretty(jobs)?),
        OutputArg::Table => Ok(render_job_table(jobs)),
    }
}

pub fn render_user(
    user: &UserEntitlement,
    output: OutputArg,
    include_projects: bool,
) -> Result<String> {
    match output {
        OutputArg::Json => Ok(serde_json::to_string_pretty(user)?),
        OutputArg::Table => {
            let mut lines = vec![
                format!("Id: {}", user.id),
                format!("姓名: {}", user.display_name()),
                format!("UPN: {}", user.upn()),
                format!("AccessLevel: {}", user.access_level.account_license_type),
                format!("授權名稱: {}", user.access_level.license_display_name),
                format!("狀態: {}", user.access_level.status),
            ];

            if include_projects {
                lines.push("可存取專案:".into());
                if user.project_entitlements.is_empty() {
                    lines.push("  （無資料）".into());
                } else {
                    for entitlement in &user.project_entitlements {
                        lines.push(format!(
                            "  - {} ({})",
                            entitlement.project_ref.name, entitlement.project_ref.id
                        ));
                    }
                }
            }

            Ok(lines.join("\n"))
        }
    }
}

pub fn render_message<T: Serialize>(
    message: &str,
    payload: &T,
    output: OutputArg,
) -> Result<String> {
    match output {
        OutputArg::Json => Ok(serde_json::to_string_pretty(payload)?),
        OutputArg::Table => Ok(message.to_owned()),
    }
}

fn render_user_table(users: &[UserEntitlement]) -> String {
    if users.is_empty() {
        return "沒有符合條件的使用者。".into();
    }

    let rows = users
        .iter()
        .map(|user| {
            vec![
                user.display_name(),
                user.upn(),
                user.id.clone(),
                user.access_level.account_license_type.clone(),
                user.access_level.license_display_name.clone(),
            ]
        })
        .collect::<Vec<_>>();

    render_table(&["姓名", "UPN", "Id", "AccessLevel", "授權名稱"], &rows)
}

fn render_project_table(projects: &[Project]) -> String {
    if projects.is_empty() {
        return "沒有符合條件的專案。".into();
    }

    let rows = projects
        .iter()
        .map(|project| {
            vec![
                project.name.clone(),
                project.id.clone(),
                project.state.clone(),
                project.visibility.clone(),
                project.last_update_time.clone(),
            ]
        })
        .collect::<Vec<_>>();

    render_table(&["名稱", "Id", "狀態", "可見性", "最後更新"], &rows)
}

fn render_agent_pool_table(pools: &[AgentPool]) -> String {
    if pools.is_empty() {
        return "沒有符合條件的代理程式集區。".into();
    }

    let rows = pools
        .iter()
        .map(|pool| {
            vec![
                pool.name.clone(),
                pool.id.to_string(),
                pool_type_label(&pool.pool_type).to_owned(),
                boolean_label(pool.is_hosted).to_owned(),
                pool.size.to_string(),
                pool.target_size
                    .map(|size| size.to_string())
                    .unwrap_or_else(|| "-".into()),
            ]
        })
        .collect::<Vec<_>>();

    render_table(
        &[
            "名稱",
            "Id",
            "型別",
            "Microsoft 代管",
            "代理程式數",
            "目標數",
        ],
        &rows,
    )
}

fn render_agent_table(agents: &[Agent]) -> String {
    if agents.is_empty() {
        return "此代理程式集區沒有代理程式。".into();
    }

    let rows = agents
        .iter()
        .map(|agent| {
            let current_job = agent
                .assigned_request
                .as_ref()
                .map(job_display_name)
                .unwrap_or_else(|| "-".into());

            vec![
                agent.name.clone(),
                agent.id.to_string(),
                agent_status_label(&agent.status).to_owned(),
                boolean_label(agent.enabled).to_owned(),
                agent.version.clone(),
                agent.os_description.clone(),
                current_job,
            ]
        })
        .collect::<Vec<_>>();

    render_table(
        &[
            "名稱",
            "Id",
            "狀態",
            "已啟用",
            "版本",
            "作業系統",
            "目前工作",
        ],
        &rows,
    )
}

fn render_job_table(jobs: &[JobRequest]) -> String {
    if jobs.is_empty() {
        return "此代理程式集區沒有工作要求。".into();
    }

    let rows = jobs
        .iter()
        .map(|job| {
            vec![
                job_display_name(job),
                job.request_id.to_string(),
                job_state_label(job.state()).to_owned(),
                job_result_label(job.result.as_deref()).to_owned(),
                job.reserved_agent
                    .as_ref()
                    .map(|agent| agent.name.clone())
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| "-".into()),
                value_or_dash(job.queue_time.as_deref()),
                value_or_dash(job.assign_time.as_deref()),
                value_or_dash(job.finish_time.as_deref()),
            ]
        })
        .collect::<Vec<_>>();

    render_table(
        &[
            "工作名稱",
            "Request Id",
            "狀態",
            "結果",
            "代理程式",
            "排入時間",
            "指派時間",
            "完成時間",
        ],
        &rows,
    )
}

fn job_display_name(job: &JobRequest) -> String {
    if let Some(job_name) = job.job_name.as_deref().filter(|value| !value.is_empty()) {
        return job_name.to_owned();
    }
    if let Some(definition) = &job.definition
        && !definition.name.is_empty()
    {
        return definition.name.clone();
    }
    if let Some(owner) = &job.owner
        && !owner.name.is_empty()
    {
        return owner.name.clone();
    }
    "-".into()
}

fn pool_type_label(value: &str) -> &str {
    match value.to_ascii_lowercase().as_str() {
        "automation" => "自動化",
        "deployment" => "部署",
        _ => value,
    }
}

fn agent_status_label(value: &str) -> &str {
    match value.to_ascii_lowercase().as_str() {
        "online" => "上線",
        "offline" => "離線",
        _ if value.is_empty() => "未知",
        _ => value,
    }
}

fn job_state_label(value: JobRequestState) -> &'static str {
    match value {
        JobRequestState::Queued => "等候中",
        JobRequestState::Running => "執行中",
        JobRequestState::Completed => "已完成",
    }
}

fn job_result_label(value: Option<&str>) -> &str {
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        return "-";
    };

    match value.to_ascii_lowercase().as_str() {
        "succeeded" => "成功",
        "succeededwithissues" => "成功但有問題",
        "failed" => "失敗",
        "canceled" | "cancelled" => "已取消",
        "abandoned" => "已放棄",
        "skipped" => "已略過",
        _ => value,
    }
}

fn boolean_label(value: bool) -> &'static str {
    if value { "是" } else { "否" }
}

fn value_or_dash(value: Option<&str>) -> String {
    value
        .filter(|value| !value.is_empty())
        .unwrap_or("-")
        .to_owned()
}

fn render_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut widths = headers
        .iter()
        .map(|header| header.chars().count())
        .collect::<Vec<_>>();

    for row in rows {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.chars().count());
        }
    }

    let mut output = String::new();
    append_row(&mut output, headers.iter().copied(), &widths);
    append_separator(&mut output, &widths);
    for row in rows {
        append_row(&mut output, row.iter().map(String::as_str), &widths);
    }
    output.trim_end().to_owned()
}

fn append_row<'a>(output: &mut String, values: impl Iterator<Item = &'a str>, widths: &[usize]) {
    for (index, value) in values.enumerate() {
        if index > 0 {
            output.push_str(" | ");
        }
        output.push_str(value);
        for _ in 0..widths[index].saturating_sub(value.chars().count()) {
            output.push(' ');
        }
    }
    output.push('\n');
}

fn append_separator(output: &mut String, widths: &[usize]) {
    for (index, width) in widths.iter().enumerate() {
        if index > 0 {
            output.push_str("-|-");
        }
        output.push_str(&"-".repeat(*width));
    }
    output.push('\n');
}

#[allow(dead_code)]
fn _assert_error_type(_: AdoctlError) {}
