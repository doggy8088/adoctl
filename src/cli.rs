use std::{fmt, str::FromStr};

use clap::{ArgAction, Args, CommandFactory, Parser, Subcommand, error::ErrorKind};

use crate::{
    access_level::{AccessLevel, AssignableAccessLevel},
    pool_type::PoolTypeFilter,
    project_state::ProjectStateFilter,
};

pub const ROOT_HELP_TEMPLATE: &str =
    "{about-with-newline}\n用法: {usage}\n\n命令:\n{subcommands}\n選項:\n{options}";
pub const SUBCOMMAND_HELP_TEMPLATE: &str =
    "{about-with-newline}\n用法: {usage}\n\n命令:\n{subcommands}\n選項:\n{options}";
pub const COMMAND_HELP_TEMPLATE: &str = "{about-with-newline}\n用法: {usage}\n\n選項:\n{options}";

#[derive(Debug, Clone, Parser)]
#[command(
    name = "adoctl",
    version,
    about = "Azure DevOps 管理工具",
    long_about = "adoctl 是用 Rust 開發的跨平台 Azure DevOps 管理工具，協助管理使用者、授權、專案成員資格與代理程式集區。",
    help_template = ROOT_HELP_TEMPLATE,
    override_usage = "adoctl [選項] <命令>",
    subcommand_required = true,
    arg_required_else_help = true,
    propagate_version = true,
    disable_help_flag = true,
    disable_version_flag = true,
    disable_help_subcommand = true
)]
pub struct Cli {
    #[arg(
        long = "org",
        global = true,
        env = "ADOCTL_ORG",
        hide_env = true,
        value_name = "組織",
        help = "Azure DevOps organization 名稱或 URL，例如 my-org 或 https://dev.azure.com/my-org；也可設定 ADOCTL_ORG"
    )]
    pub org: Option<String>,

    #[arg(
        long = "profile",
        global = true,
        env = "ADOCTL_PROFILE",
        default_value = "default",
        hide_env = true,
        hide_default_value = true,
        value_name = "設定檔",
        help = "憑證設定檔名稱，可用來區分不同 organization 或登入身分；預設為 default，也可設定 ADOCTL_PROFILE"
    )]
    pub profile: String,

    #[arg(
        long = "auth",
        global = true,
        env = "ADOCTL_AUTH_METHOD",
        hide_env = true,
        value_name = "方式",
        help = "指定認證方式；可用值：pat、azure-cli、device-code；未指定時會使用已保存的登入資訊或環境變數"
    )]
    pub auth_method: Option<AuthMethodArg>,

    #[arg(
        long = "output",
        short = 'o',
        global = true,
        default_value_t = OutputArg::Table,
        hide_default_value = true,
        value_name = "格式",
        help = "輸出格式；可用值：table、json；預設為 table"
    )]
    pub output: OutputArg,

    #[arg(
        long = "base-url",
        global = true,
        env = "ADOCTL_BASE_URL",
        hide = true,
        hide_env = true,
        help = "測試用 API base URL override"
    )]
    pub base_url: Option<String>,

    #[arg(
        long = "debug",
        global = true,
        action = ArgAction::SetTrue,
        help = "輸出較詳細的除錯資訊到 stderr，不影響 stdout 的 table/json 結果"
    )]
    pub debug: bool,

    #[arg(short = 'h', long = "help", global = true, action = ArgAction::Help, help = "顯示說明")]
    pub help: Option<bool>,

    #[arg(short = 'V', long = "version", global = true, action = ArgAction::Version, help = "顯示版本")]
    pub version: Option<bool>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Commands {
    #[command(
        about = "登入 Azure DevOps 或保存認證設定",
        help_template = COMMAND_HELP_TEMPLATE,
        override_usage = "adoctl login [選項]"
    )]
    Login(LoginArgs),
    #[command(
        name = "user",
        subcommand,
        about = "管理 Azure DevOps 使用者",
        help_template = SUBCOMMAND_HELP_TEMPLATE,
        override_usage = "adoctl user [選項] <命令>",
        subcommand_required = true,
        arg_required_else_help = true
    )]
    Users(UsersCommand),
    #[command(
        name = "project",
        subcommand,
        about = "管理 Azure DevOps 專案與成員資格",
        help_template = SUBCOMMAND_HELP_TEMPLATE,
        override_usage = "adoctl project [選項] <命令>",
        subcommand_required = true,
        arg_required_else_help = true
    )]
    Projects(ProjectsCommand),
    #[command(
        name = "pool",
        subcommand,
        about = "管理 Azure Pipelines 代理程式集區、代理程式與工作",
        help_template = SUBCOMMAND_HELP_TEMPLATE,
        override_usage = "adoctl pool [選項] <命令>",
        subcommand_required = true,
        arg_required_else_help = true
    )]
    Pools(PoolsCommand),
}

#[derive(Debug, Clone, Args)]
pub struct LoginArgs {
    #[arg(
        long = "method",
        default_value_t = AuthMethodArg::Pat,
        hide_default_value = true,
        value_name = "方式",
        help = "登入方式；可用值：pat、azure-cli、device-code；預設為 pat"
    )]
    pub method: AuthMethodArg,

    #[arg(
        long = "pat",
        env = "ADOCTL_PAT",
        hide_env = true,
        value_name = "PAT",
        help = "直接提供 PAT；不建議寫在 shell history；也可設定 ADOCTL_PAT，未指定時會使用 AZURE_DEVOPS_EXT_PAT"
    )]
    pub pat: Option<String>,

    #[arg(long = "no-store", help = "只驗證本次登入，不保存憑證")]
    pub no_store: bool,

    #[arg(
        long = "open-browser",
        help = "PAT 模式下嘗試開啟 Azure DevOps PAT 建立頁面"
    )]
    pub open_browser: bool,

    #[arg(
        long = "device-client-id",
        env = "ADOCTL_DEVICE_CLIENT_ID",
        hide_env = true,
        value_name = "CLIENT_ID",
        help = "OAuth device code 使用的 Microsoft Entra public client id；也可設定 ADOCTL_DEVICE_CLIENT_ID"
    )]
    pub device_client_id: Option<String>,

    #[arg(
        long = "tenant",
        env = "ADOCTL_TENANT",
        default_value = "organizations",
        hide_env = true,
        hide_default_value = true,
        value_name = "TENANT",
        help = "OAuth tenant；預設使用 organizations，也可設定 ADOCTL_TENANT"
    )]
    pub tenant: String,
}

#[derive(Debug, Clone, Subcommand)]
pub enum UsersCommand {
    #[command(
        about = "列出使用者，可依 accessLevel 或關鍵字過濾",
        help_template = COMMAND_HELP_TEMPLATE,
        override_usage = "adoctl user list [選項]"
    )]
    List(UsersListArgs),
    #[command(
        about = "取得單一使用者資訊、授權與可存取專案",
        help_template = COMMAND_HELP_TEMPLATE,
        override_usage = "adoctl user get (--upn <email> | --id <id>) [選項]"
    )]
    Get(UsersGetArgs),
    #[command(
        about = "變更使用者 accessLevel",
        help_template = COMMAND_HELP_TEMPLATE,
        override_usage = "adoctl user set-access (--upn <email> | --id <id>) --access-level <等級> [選項]"
    )]
    SetAccess(UsersSetAccessArgs),
}

#[derive(Debug, Clone, Args)]
pub struct UsersListArgs {
    #[arg(
        long = "access-level",
        value_name = "等級",
        help = "依 accessLevel 過濾；可用值：stakeholder、basic（別名 express）、basic-test-plans（別名 advanced）、visual-studio-subscriber、visual-studio-enterprise、github-enterprise"
    )]
    pub access_level: Option<AccessLevel>,

    #[arg(
        long = "search",
        value_name = "關鍵字",
        help = "依姓名、UPN 或 Email 搜尋"
    )]
    pub search: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct UsersGetArgs {
    #[command(flatten)]
    pub user: UserSelectorArgs,

    #[arg(long = "include-projects", help = "輸出使用者可存取的專案資訊")]
    pub include_projects: bool,
}

#[derive(Debug, Clone, Args)]
pub struct UsersSetAccessArgs {
    #[command(flatten)]
    pub user: UserSelectorArgs,

    #[arg(
        long = "access-level",
        value_name = "等級",
        help = "要設定的 accessLevel；可用值：stakeholder、basic（別名 express）、basic-test-plans（別名 advanced）、visual-studio-subscriber、visual-studio-enterprise；GitHub Enterprise 由 Azure DevOps 自動偵測"
    )]
    pub access_level: AssignableAccessLevel,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ProjectsCommand {
    #[command(
        about = "列出專案，可依狀態或關鍵字過濾",
        help_template = COMMAND_HELP_TEMPLATE,
        override_usage = "adoctl project list [選項]"
    )]
    List(ProjectsListArgs),
    #[command(
        about = "將使用者加入專案群組；未指定 --group 時使用 Contributors",
        help_template = COMMAND_HELP_TEMPLATE,
        override_usage = "adoctl project add-user --project <專案> (--upn <email> | --id <id>) [選項]"
    )]
    AddUser(ProjectMembershipArgs),
    #[command(
        about = "將使用者從專案群組移除；未指定 --group 時使用 Contributors",
        help_template = COMMAND_HELP_TEMPLATE,
        override_usage = "adoctl project remove-user --project <專案> (--upn <email> | --id <id>) [選項]"
    )]
    RemoveUser(ProjectMembershipArgs),
}

#[derive(Debug, Clone, Args)]
pub struct ProjectsListArgs {
    #[arg(
        long = "state",
        default_value_t = ProjectStateFilter::All,
        hide_default_value = true,
        value_name = "狀態",
        help = "依專案狀態過濾；可用值：all、new、create-pending、well-formed、deleting；預設為 all"
    )]
    pub state: ProjectStateFilter,

    #[arg(
        long = "search",
        value_name = "關鍵字",
        help = "依專案名稱、描述或 Id 搜尋"
    )]
    pub search: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct ProjectMembershipArgs {
    #[arg(long = "project", value_name = "專案", help = "專案名稱或專案 Id")]
    pub project: String,

    #[command(flatten)]
    pub user: UserSelectorArgs,

    #[arg(
        long = "group",
        default_value = "Contributors",
        hide_default_value = true,
        value_name = "群組",
        help = "專案群組名稱或 descriptor；預設為 Contributors"
    )]
    pub group: String,
}

#[derive(Debug, Clone, Subcommand)]
pub enum PoolsCommand {
    #[command(
        about = "列出 organization 內的代理程式集區，可依集區型別過濾",
        help_template = COMMAND_HELP_TEMPLATE,
        override_usage = "adoctl pool list [選項]"
    )]
    List(PoolsListArgs),
    #[command(
        about = "列出指定代理程式集區內的所有代理程式與狀態",
        help_template = COMMAND_HELP_TEMPLATE,
        override_usage = "adoctl pool agents --pool <集區>"
    )]
    Agents(PoolSelectorArgs),
    #[command(
        about = "列出指定代理程式集區保留的所有工作要求",
        help_template = COMMAND_HELP_TEMPLATE,
        override_usage = "adoctl pool jobs --pool <集區>"
    )]
    Jobs(PoolSelectorArgs),
}

#[derive(Debug, Clone, Args)]
pub struct PoolsListArgs {
    #[arg(
        long = "pool-type",
        value_name = "型別",
        help = "依代理程式集區型別過濾；可用值：automation、deployment"
    )]
    pub pool_type: Option<PoolTypeFilter>,
}

#[derive(Debug, Clone, Args)]
pub struct PoolSelectorArgs {
    #[arg(long = "pool", value_name = "集區", help = "代理程式集區名稱或數字 Id")]
    pub pool: String,
}

#[derive(Debug, Clone, Args)]
pub struct UserSelectorArgs {
    #[arg(
        long = "upn",
        conflicts_with = "id",
        required_unless_present = "id",
        value_name = "EMAIL",
        help = "使用者 UPN / Email"
    )]
    pub upn: Option<String>,

    #[arg(
        long = "id",
        conflicts_with = "upn",
        required_unless_present = "upn",
        value_name = "ID",
        help = "Azure DevOps 使用者 Id"
    )]
    pub id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethodArg {
    Pat,
    AzureCli,
    DeviceCode,
}

impl fmt::Display for AuthMethodArg {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Pat => "pat",
            Self::AzureCli => "azure-cli",
            Self::DeviceCode => "device-code",
        })
    }
}

impl FromStr for AuthMethodArg {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "pat" => Ok(Self::Pat),
            "azure-cli" => Ok(Self::AzureCli),
            "device-code" => Ok(Self::DeviceCode),
            _ => Err("認證方式無效；可用值：pat、azure-cli、device-code".into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputArg {
    Table,
    Json,
}

impl fmt::Display for OutputArg {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Table => "table",
            Self::Json => "json",
        })
    }
}

impl FromStr for OutputArg {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "table" => Ok(Self::Table),
            "json" => Ok(Self::Json),
            _ => Err("輸出格式無效；可用值：table、json".into()),
        }
    }
}

pub fn render_parse_error(error: &clap::Error) -> String {
    match error.kind() {
        ErrorKind::UnknownArgument => "參數無法辨識，請使用 --help 查看說明。".into(),
        ErrorKind::InvalidValue | ErrorKind::ValueValidation => {
            "參數值無效，請使用 --help 查看可用值。".into()
        }
        ErrorKind::MissingRequiredArgument => "缺少必要參數，請使用 --help 查看說明。".into(),
        ErrorKind::ArgumentConflict => "參數彼此衝突，請檢查後重試。".into(),
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => error.to_string(),
        _ => "命令格式無效，請使用 --help 查看說明。".into(),
    }
}

pub fn render_help_if_missing_command(
    args: impl IntoIterator<Item = impl Into<std::ffi::OsString>>,
) -> Option<String> {
    let args = args
        .into_iter()
        .map(|arg| arg.into().to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "-V" | "--version"))
    {
        return None;
    }

    let command_path = command_path_without_global_args(&args)?;
    match command_path.as_slice() {
        [] => Some(render_command_help(None)),
        [command] if command == "user" => Some(render_command_help(Some("user"))),
        [command] if command == "project" => Some(render_command_help(Some("project"))),
        [command] if command == "pool" => Some(render_command_help(Some("pool"))),
        _ => None,
    }
}

fn command_path_without_global_args(args: &[String]) -> Option<Vec<String>> {
    let mut command_path = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if matches!(
            arg.as_str(),
            "--org" | "--profile" | "--auth" | "--output" | "-o" | "--base-url"
        ) {
            index += 2;
            if index > args.len() {
                return None;
            }
            continue;
        }

        if arg == "--debug" {
            index += 1;
            continue;
        }

        if arg.starts_with("--org=")
            || arg.starts_with("--profile=")
            || arg.starts_with("--auth=")
            || arg.starts_with("--output=")
            || arg.starts_with("--base-url=")
        {
            index += 1;
            continue;
        }

        command_path.push(arg.clone());
        index += 1;
    }

    Some(command_path)
}

fn render_command_help(subcommand: Option<&str>) -> String {
    let mut command = Cli::command();
    match subcommand {
        Some(name) => command
            .find_subcommand_mut(name)
            .expect("CLI subcommand should exist")
            .render_help()
            .to_string(),
        None => command.render_help().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }
}
