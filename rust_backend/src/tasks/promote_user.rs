use loco_rs::prelude::*;
use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use crate::models::_entities::users;

pub struct PromoteUser;

#[async_trait]
impl task::Task for PromoteUser {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "promote_user".to_string(),
            detail: "Promote a user to admin by email".to_string(),
        }
    }

    async fn run(&self, ctx: &AppContext, vars: &task::Vars) -> Result<()> {
        let email = vars.cli_arg("email")?;

        let user = users::Entity::find()
            .filter(users::Column::Email.eq(email))
            .one(&ctx.db)
            .await?
            .ok_or_else(|| Error::string(&format!("User '{}' not found", email)))?;

        let mut active: users::ActiveModel = user.into();
        active.role = ActiveValue::Set("admin".to_string());
        active.update(&ctx.db).await?;

        println!("User '{}' promoted to admin", email);
        Ok(())
    }
}
