use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

#[test]
fn root_help_is_traditional_chinese() {
    let mut command = Command::cargo_bin("adoctl").unwrap();
    command
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("Azure DevOps 管理工具"))
        .stdout(contains("用法:"))
        .stdout(contains("命令:"))
        .stdout(contains("選項:"))
        .stdout(contains("登入 Azure DevOps"))
        .stdout(contains("管理 Azure DevOps 使用者"))
        .stdout(predicates::str::contains("Usage:").not())
        .stdout(predicates::str::contains("Commands:").not())
        .stdout(predicates::str::contains("Options:").not());
}

#[test]
fn root_without_args_displays_help() {
    let mut command = Command::cargo_bin("adoctl").unwrap();
    command
        .assert()
        .success()
        .stdout(contains("Azure DevOps 管理工具"))
        .stdout(contains("用法: adoctl [選項] <命令>"))
        .stdout(contains("命令:"))
        .stdout(predicates::str::contains("Usage:").not());
}

#[test]
fn root_help_mentions_debug_flag() {
    let mut command = Command::cargo_bin("adoctl").unwrap();
    command
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("--debug"))
        .stdout(contains("除錯資訊"));
}

#[test]
fn command_group_without_subcommand_displays_help() {
    let mut command = Command::cargo_bin("adoctl").unwrap();
    command
        .args(["user"])
        .assert()
        .success()
        .stdout(contains("管理 Azure DevOps 使用者"))
        .stdout(contains("用法: adoctl user [選項] <命令>"))
        .stdout(contains("list"))
        .stdout(predicates::str::contains("Usage:").not());
}

#[test]
fn command_group_without_subcommand_displays_help_with_debug_flag() {
    let mut command = Command::cargo_bin("adoctl").unwrap();
    command
        .args(["--debug", "user"])
        .assert()
        .success()
        .stdout(contains("管理 Azure DevOps 使用者"))
        .stdout(contains("用法: adoctl user [選項] <命令>"));
}

#[test]
fn project_group_without_subcommand_displays_help() {
    let mut command = Command::cargo_bin("adoctl").unwrap();
    command
        .args(["project"])
        .assert()
        .success()
        .stdout(contains("管理 Azure DevOps 專案與成員資格"))
        .stdout(contains("用法: adoctl project [選項] <命令>"))
        .stdout(contains("list"))
        .stdout(contains("add-user"))
        .stdout(predicates::str::contains("Usage:").not());
}

#[test]
fn project_list_help_mentions_filters() {
    let mut command = Command::cargo_bin("adoctl").unwrap();
    command
        .args(["project", "list", "--help"])
        .assert()
        .success()
        .stdout(contains("依專案狀態過濾"))
        .stdout(contains("--state"))
        .stdout(contains("--search"))
        .stdout(predicates::str::contains("Possible values").not());
}

#[test]
fn pool_group_without_subcommand_displays_help() {
    let mut command = Command::cargo_bin("adoctl").unwrap();
    command
        .args(["pool"])
        .assert()
        .success()
        .stdout(contains("管理 Azure Pipelines 代理程式集區"))
        .stdout(contains("用法: adoctl pool [選項] <命令>"))
        .stdout(contains("agents"))
        .stdout(contains("jobs"))
        .stdout(predicates::str::contains("Usage:").not());
}

#[test]
fn pool_list_help_mentions_pool_type_filter() {
    let mut command = Command::cargo_bin("adoctl").unwrap();
    command
        .args(["pool", "list", "--help"])
        .assert()
        .success()
        .stdout(contains("--pool-type"))
        .stdout(contains("automation"))
        .stdout(contains("deployment"))
        .stdout(predicates::str::contains("Possible values").not());
}

#[test]
fn pool_agents_requires_pool_selector() {
    let mut command = Command::cargo_bin("adoctl").unwrap();
    command
        .args(["--org", "my-org", "pool", "agents"])
        .assert()
        .failure()
        .stderr(contains("缺少必要參數"));
}

#[test]
fn login_help_mentions_pat_and_device_code() {
    let mut command = Command::cargo_bin("adoctl").unwrap();
    command
        .args(["login", "--help"])
        .assert()
        .success()
        .stdout(contains("登入方式"))
        .stdout(contains("OAuth device code"))
        .stdout(contains("PAT"))
        .stdout(predicates::str::contains("Possible values").not())
        .stdout(predicates::str::contains("default:").not())
        .stdout(predicates::str::contains("env:").not());
}

#[test]
fn user_selector_requires_upn_or_id() {
    let mut command = Command::cargo_bin("adoctl").unwrap();
    command
        .args(["--org", "my-org", "user", "get"])
        .assert()
        .failure()
        .stderr(contains("缺少必要參數"));
}
