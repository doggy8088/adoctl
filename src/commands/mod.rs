use std::sync::Arc;

use crate::{
    ado::{client::AdoClient, pools, projects, users},
    auth::{Authenticator, load_stored_or_env_credential},
    cli::{Cli, Commands, PoolsCommand, ProjectsCommand, UsersCommand},
    config::AppConfig,
    credentials::CredentialStore,
    error::Result,
    identity::UserIdentifier,
    output,
};

mod login;

pub async fn execute(cli: Cli, store: Arc<dyn CredentialStore>) -> Result<()> {
    let config = AppConfig::from_cli(&cli)?;

    match cli.command {
        Commands::Login(args) => login::run(config, args, store).await,
        Commands::Users(command) => {
            let client = authenticated_client(&config, store)?;
            match command {
                UsersCommand::List(args) => {
                    let users =
                        users::list_users(&client, args.access_level, args.search.as_deref())
                            .await?;
                    println!("{}", output::render_users(&users, config.output)?);
                }
                UsersCommand::Get(args) => {
                    let identifier = UserIdentifier::from_parts(args.user.upn, args.user.id)?;
                    let user = users::get_user(&client, &identifier).await?;
                    println!(
                        "{}",
                        output::render_user(&user, config.output, args.include_projects)?
                    );
                }
                UsersCommand::SetAccess(args) => {
                    let identifier = UserIdentifier::from_parts(args.user.upn, args.user.id)?;
                    let user =
                        users::set_access_level(&client, &identifier, args.access_level).await?;
                    println!(
                        "{}",
                        output::render_message(
                            &format!(
                                "已將 {} 的 accessLevel 設定為 {}。",
                                user.upn(),
                                args.access_level.display_name()
                            ),
                            &user,
                            config.output
                        )?
                    );
                }
            }
            Ok(())
        }
        Commands::Projects(command) => {
            let client = authenticated_client(&config, store)?;
            match command {
                ProjectsCommand::List(args) => {
                    let projects =
                        projects::list_projects(&client, args.state, args.search.as_deref())
                            .await?;
                    println!("{}", output::render_projects(&projects, config.output)?);
                }
                ProjectsCommand::AddUser(args) => {
                    let identifier = UserIdentifier::from_parts(args.user.upn, args.user.id)?;
                    projects::add_user_to_project(&client, &args.project, &identifier, &args.group)
                        .await?;
                    println!(
                        "{}",
                        output::render_message(
                            &format!(
                                "已將 {} 加入專案 {} 的 {} 群組。",
                                identifier.label(),
                                args.project,
                                args.group
                            ),
                            &serde_json::json!({
                                "user": identifier,
                                "project": args.project,
                                "group": args.group,
                                "action": "add"
                            }),
                            config.output
                        )?
                    );
                }
                ProjectsCommand::RemoveUser(args) => {
                    let identifier = UserIdentifier::from_parts(args.user.upn, args.user.id)?;
                    projects::remove_user_from_project(
                        &client,
                        &args.project,
                        &identifier,
                        &args.group,
                    )
                    .await?;
                    println!(
                        "{}",
                        output::render_message(
                            &format!(
                                "已將 {} 從專案 {} 的 {} 群組移除。",
                                identifier.label(),
                                args.project,
                                args.group
                            ),
                            &serde_json::json!({
                                "user": identifier,
                                "project": args.project,
                                "group": args.group,
                                "action": "remove"
                            }),
                            config.output
                        )?
                    );
                }
            }
            Ok(())
        }
        Commands::Pools(command) => {
            let client = authenticated_client(&config, store)?;
            match command {
                PoolsCommand::List(args) => {
                    let pools = pools::list_agent_pools(&client, args.pool_type).await?;
                    println!("{}", output::render_agent_pools(&pools, config.output)?);
                }
                PoolsCommand::Agents(args) => {
                    let agents = pools::list_agents(&client, &args.pool).await?;
                    println!("{}", output::render_agents(&agents, config.output)?);
                }
                PoolsCommand::Jobs(args) => {
                    let jobs = pools::list_jobs(&client, &args.pool).await?;
                    println!("{}", output::render_jobs(&jobs, config.output)?);
                }
            }
            Ok(())
        }
    }
}

fn authenticated_client(config: &AppConfig, store: Arc<dyn CredentialStore>) -> Result<AdoClient> {
    let (credential, key) = load_stored_or_env_credential(
        &config.organization,
        &config.profile,
        config.auth_method,
        store.clone(),
    )?;
    let auth = Authenticator::new(credential, Some(store), Some(key));
    Ok(AdoClient::new(config.organization.clone(), auth))
}
